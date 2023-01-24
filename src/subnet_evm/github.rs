use std::{
    env, fmt,
    fs::{self, File},
    io::{self, Error, ErrorKind},
    os::unix::fs::PermissionsExt,
    path::Path,
};

use compress_manager::DirDecoder;
use tokio::time::{sleep, Duration};

/// Downloads the latest "subnet-evm" from the github release page.
pub async fn download_latest(arch: Option<Arch>, os: Option<Os>) -> io::Result<String> {
    download(arch, os, None).await
}

pub const DEFAULT_TAG_NAME: &str = "v0.4.8";

/// ref. https://github.com/ava-labs/subnet-evm/releases
pub async fn download(
    arch: Option<Arch>,
    os: Option<Os>,
    release_tag: Option<String>,
) -> io::Result<String> {
    // e.g., "v0.4.8"
    let tag_name = if let Some(v) = release_tag {
        v
    } else {
        log::info!("fetching the latest git tags");
        let mut release_info = crate::github::ReleaseResponse::default();
        for round in 0..20 {
            let info = match crate::github::fetch_latest_release("ava-labs", "subnet-evm").await {
                Ok(v) => v,
                Err(e) => {
                    log::warn!(
                        "failed fetch_latest_release {} -- retrying {}...",
                        e,
                        round + 1
                    );
                    sleep(Duration::from_secs((round + 1) * 5)).await;
                    continue;
                }
            };

            release_info = info;
            if release_info.tag_name.is_some() {
                break;
            }

            log::warn!("release_info.tag_name is None -- retrying {}...", round + 1);
            sleep(Duration::from_secs((round + 1) * 5)).await;
        }

        if release_info.tag_name.is_none() {
            log::warn!("release_info.tag_name not found -- defaults to {DEFAULT_TAG_NAME}");
            release_info.tag_name = Some(DEFAULT_TAG_NAME.to_string());
        }

        if release_info.prerelease {
            log::warn!(
                "latest release '{}' is prerelease, falling back to default tag name '{}'",
                release_info.tag_name.unwrap(),
                DEFAULT_TAG_NAME
            );
            DEFAULT_TAG_NAME.to_string()
        } else {
            release_info.tag_name.unwrap()
        }
    };

    // ref. https://github.com/ava-labs/subnet-evm/releases
    log::info!(
        "detecting arch and platform for the release version tag {}",
        tag_name
    );
    let arch = {
        if arch.is_none() {
            match env::consts::ARCH {
                "x86_64" => String::from("amd64"),
                "aarch64" => String::from("arm64"),
                _ => String::from(""),
            }
        } else {
            let arch = arch.unwrap();
            arch.to_string()
        }
    };

    // ref. https://github.com/ava-labs/subnet-evm/releases
    let (file_name, dir_decoder) = {
        if os.is_none() {
            if cfg!(target_os = "macos") {
                (
                    format!(
                        "subnet-evm_{}_darwin_{arch}.tar.gz",
                        tag_name.trim_start_matches("v")
                    ),
                    DirDecoder::TarGzip,
                )
            } else if cfg!(unix) {
                (
                    format!(
                        "subnet-evm_{}_linux_{arch}.tar.gz",
                        tag_name.trim_start_matches("v")
                    ),
                    DirDecoder::TarGzip,
                )
            } else {
                return Err(Error::new(ErrorKind::Other, "unknown OS"));
            }
        } else {
            let os = os.unwrap();
            match os {
                Os::MacOs => (
                    format!(
                        "subnet-evm_{}_darwin_{arch}.tar.gz",
                        tag_name.trim_start_matches("v")
                    ),
                    DirDecoder::TarGzip,
                ),
                Os::Linux => (
                    format!(
                        "subnet-evm_{}_linux_{arch}.tar.gz",
                        tag_name.trim_start_matches("v")
                    ),
                    DirDecoder::TarGzip,
                ),
                Os::Windows => return Err(Error::new(ErrorKind::Other, "windows not supported")),
            }
        }
    };
    if file_name.is_empty() {
        return Err(Error::new(
            ErrorKind::Other,
            format!("unknown platform '{}'", env::consts::OS),
        ));
    }

    log::info!("downloading latest subnet-evm '{}'", file_name);
    let download_url = format!(
        "https://github.com/ava-labs/subnet-evm/releases/download/{}/{}",
        tag_name, file_name
    );
    let tmp_file_path = random_manager::tmp_path(10, Some(dir_decoder.suffix()))?;
    http_manager::download_file(&download_url, &tmp_file_path).await?;

    let dst_dir_path = random_manager::tmp_path(10, None)?;
    log::info!("unpacking {} to {}", tmp_file_path, dst_dir_path);
    compress_manager::unpack_directory(&tmp_file_path, &dst_dir_path, dir_decoder.clone())?;

    // TODO: this can fail due to files being still busy...
    log::info!("cleaning up downloaded file {}", tmp_file_path);
    match fs::remove_file(&tmp_file_path) {
        Ok(_) => log::info!("removed downloaded file {}", tmp_file_path),
        Err(e) => log::warn!(
            "failed to remove downloaded file {} ({}), skipping for now...",
            tmp_file_path,
            e
        ),
    }

    let subnet_evm_path = Path::new(&dst_dir_path).join("subnet-evm");
    {
        let f = File::open(&subnet_evm_path)?;
        f.set_permissions(PermissionsExt::from_mode(0o777))?;
    }
    Ok(String::from(subnet_evm_path.as_os_str().to_str().unwrap()))
}

/// Represents the subnet-evm release "arch".
#[derive(Eq, PartialEq, Clone)]
pub enum Arch {
    Amd64,
    Arm64,
}

/// ref. https://doc.rust-lang.org/std/string/trait.ToString.html
/// ref. https://doc.rust-lang.org/std/fmt/trait.Display.html
/// Use "Self.to_string()" to directly invoke this
impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Arch::Amd64 => write!(f, "amd64"),
            Arch::Arm64 => write!(f, "arm64"),
        }
    }
}

impl Arch {
    pub fn new(arch: &str) -> io::Result<Self> {
        match arch {
            "amd64" => Ok(Arch::Amd64),
            "arm64" => Ok(Arch::Arm64),
            _ => Err(Error::new(
                ErrorKind::InvalidInput,
                format!("unknown arch {}", arch),
            )),
        }
    }
}

/// Represents the subnet-evm release "os".
#[derive(Eq, PartialEq, Clone)]
pub enum Os {
    MacOs,
    Linux,
    Windows,
}

/// ref. https://doc.rust-lang.org/std/string/trait.ToString.html
/// ref. https://doc.rust-lang.org/std/fmt/trait.Display.html
/// Use "Self.to_string()" to directly invoke this
impl fmt::Display for Os {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Os::MacOs => write!(f, "macos"),
            Os::Linux => write!(f, "linux"),
            Os::Windows => write!(f, "win"),
        }
    }
}

impl Os {
    pub fn new(os: &str) -> io::Result<Self> {
        match os {
            "macos" => Ok(Os::MacOs),
            "linux" => Ok(Os::Linux),
            "win" => Ok(Os::Windows),
            _ => Err(Error::new(
                ErrorKind::InvalidInput,
                format!("unknown os {}", os),
            )),
        }
    }
}