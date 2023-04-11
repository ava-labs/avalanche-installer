use std::{
    env, fmt,
    fs::{self, File},
    io::{self, copy, Cursor, Error, ErrorKind},
    os::unix::fs::PermissionsExt,
    path::Path,
};

use compress_manager::DirDecoder;
use tokio::time::{sleep, Duration};

/// Downloads the latest "avalanchego" from the github release page.
pub async fn download_latest(arch: Option<Arch>, os: Option<Os>) -> io::Result<String> {
    download(arch, os, None).await
}

/// ref. <https://github.com/ava-labs/avalanchego/releases>
pub const DEFAULT_TAG_NAME: &str = "v1.9.16";

/// Downloads the official "avalanchego" binaries from the GitHub release page.
/// Returns the path to the binary path.
///
/// Leave "release_tag" none to download the latest.
///
/// Leave "arch" and "os" empty to auto-detect from its local system.
/// "arch" must be either "amd64" or "arm64".
/// "os" must be either "macos", "linux", or "win".
/// ref. <https://github.com/ava-labs/avalanchego/releases>
pub async fn download(
    arch: Option<Arch>,
    os: Option<Os>,
    release_tag: Option<String>,
) -> io::Result<String> {
    // e.g., "v1.9.16"
    let tag_name = if let Some(v) = release_tag {
        // "https://github.com/ava-labs/avalanchego/releases" doesn't have "latest" tag
        if v.eq("latest") {
            log::warn!("falling back 'latest' to {DEFAULT_TAG_NAME}");
            DEFAULT_TAG_NAME.to_owned()
        } else {
            v
        }
    } else {
        log::info!("fetching the latest git tags");
        let mut release_info = crate::github::ReleaseResponse::default();
        for round in 0..10 {
            let info = match crate::github::fetch_latest_release("ava-labs", "avalanchego").await {
                Ok(v) => v,
                Err(e) => {
                    log::warn!(
                        "failed fetch_latest_release {} -- retrying {}...",
                        e,
                        round + 1
                    );
                    sleep(Duration::from_secs((round + 1) * 3)).await;
                    continue;
                }
            };

            release_info = info;
            if release_info.tag_name.is_some() {
                break;
            }

            log::warn!("release_info.tag_name is None -- retrying {}...", round + 1);
            sleep(Duration::from_secs((round + 1) * 3)).await;
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

    // ref. <https://github.com/ava-labs/avalanchego/releases>
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

    // TODO: handle Apple arm64 when the official binary is available
    // ref. <https://github.com/ava-labs/avalanchego/releases>
    let (file_name, dir_decoder) = {
        if os.is_none() {
            if cfg!(target_os = "macos") {
                (
                    format!("avalanchego-macos-{}.zip", tag_name),
                    DirDecoder::Zip,
                )
            } else if cfg!(unix) {
                (
                    format!("avalanchego-linux-{}-{}.tar.gz", arch, tag_name),
                    DirDecoder::TarGzip,
                )
            } else if cfg!(windows) {
                (
                    format!("avalanchego-win-{}-experimental.zip", tag_name),
                    DirDecoder::Zip,
                )
            } else {
                (String::new(), DirDecoder::Zip)
            }
        } else {
            let os = os.unwrap();
            match os {
                Os::MacOs => (
                    format!("avalanchego-macos-{}.zip", tag_name),
                    DirDecoder::Zip,
                ),
                Os::Linux => (
                    format!("avalanchego-linux-{}-{}.tar.gz", arch, tag_name),
                    DirDecoder::TarGzip,
                ),
                Os::Windows => (
                    format!("avalanchego-win-{}-experimental.zip", tag_name),
                    DirDecoder::Zip,
                ),
            }
        }
    };
    if file_name.is_empty() {
        return Err(Error::new(
            ErrorKind::Other,
            format!("unknown platform '{}'", env::consts::OS),
        ));
    }

    log::info!("downloading latest avalanchego '{}'", file_name);
    let download_url = format!(
        "https://github.com/ava-labs/avalanchego/releases/download/{}/{}",
        tag_name, file_name
    );
    let tmp_file_path = random_manager::tmp_path(10, Some(dir_decoder.suffix()))?;
    download_file(&download_url, &tmp_file_path).await?;

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

    let avalanchego_path = if dir_decoder.clone().suffix() == DirDecoder::Zip.suffix() {
        Path::new(&dst_dir_path).join("build").join("avalanchego")
    } else {
        Path::new(&dst_dir_path)
            .join(format!("avalanchego-{}", tag_name))
            .join("avalanchego")
    };

    {
        let f = File::open(&avalanchego_path)?;
        f.set_permissions(PermissionsExt::from_mode(0o777))?;
    }
    Ok(String::from(avalanchego_path.as_os_str().to_str().unwrap()))
}

/// Represents the AvalancheGo release "arch".
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

/// Represents the AvalancheGo release "os".
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

/// Downloads a file to the "file_path".
pub async fn download_file(ep: &str, file_path: &str) -> io::Result<()> {
    log::info!("downloading the file via {}", ep);
    let resp = reqwest::get(ep)
        .await
        .map_err(|e| Error::new(ErrorKind::Other, format!("failed reqwest::get {}", e)))?;

    let mut content = Cursor::new(
        resp.bytes()
            .await
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed bytes {}", e)))?,
    );

    let mut f = File::create(file_path)?;
    copy(&mut content, &mut f)?;

    Ok(())
}
