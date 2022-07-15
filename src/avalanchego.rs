use std::{
    env, fmt,
    fs::{self, File},
    io::{self, Error, ErrorKind},
    os::unix::fs::PermissionsExt,
    path::Path,
};

use compress_manager::DirDecoder;

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

/// Downloads the official "avalanchego" binaries from the GitHub release page.
/// Returns the path to the binary path and "plugins" directory.
/// Leave "arch" and "os" empty to auto-detect from its local system.
/// "arch" must be either "amd64" or "arm64".
/// "os" must be either "macos", "linux", or "win".
/// ref. https://github.com/ava-labs/avalanchego/releases
pub async fn download(arch: Option<Arch>, os: Option<Os>) -> io::Result<(String, String)> {
    log::info!("fetching the latest git tags");
    let release_info = crate::fetch_latest_release("ava-labs", "avalanchego").await?;

    // ref. https://github.com/ava-labs/avalanchego/releases
    log::info!("detecting arch and platform");
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
    // ref. https://github.com/ava-labs/avalanchego/releases
    let (file_name, dir_decoder) = {
        if os.is_none() {
            if cfg!(target_os = "macos") {
                (
                    format!("avalanchego-macos-{}.zip", release_info.tag_name),
                    DirDecoder::Zip,
                )
            } else if cfg!(unix) {
                (
                    format!(
                        "avalanchego-linux-{}-{}.tar.gz",
                        arch, release_info.tag_name
                    ),
                    DirDecoder::TarGzip,
                )
            } else if cfg!(windows) {
                (
                    format!("avalanchego-win-{}-experimental.zip", release_info.tag_name),
                    DirDecoder::Zip,
                )
            } else {
                (String::new(), DirDecoder::Zip)
            }
        } else {
            let os = os.unwrap();
            match os {
                Os::MacOs => (
                    format!("avalanchego-macos-{}.zip", release_info.tag_name),
                    DirDecoder::Zip,
                ),
                Os::Linux => (
                    format!(
                        "avalanchego-linux-{}-{}.tar.gz",
                        arch, release_info.tag_name
                    ),
                    DirDecoder::TarGzip,
                ),
                Os::Windows => (
                    format!("avalanchego-win-{}-experimental.zip", release_info.tag_name),
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
        release_info.tag_name, file_name
    );
    let tmp_file_path = random_manager::tmp_path(10, Some(dir_decoder.suffix()))?;
    http_manager::download_file(&download_url, &tmp_file_path).await?;

    let dst_dir_path = random_manager::tmp_path(10, None)?;
    log::info!("unpacking {} to {}", tmp_file_path, dst_dir_path);
    compress_manager::unpack_directory(&tmp_file_path, &dst_dir_path, dir_decoder.clone())?;

    log::info!("cleaning up downloaded files");
    fs::remove_file(&tmp_file_path)?;

    let (avalanchego_path, plugins_dir) = {
        if dir_decoder.clone().suffix() == DirDecoder::Zip.suffix() {
            (
                Path::new(&dst_dir_path).join("build").join("avalanchego"),
                Path::new(&dst_dir_path).join("build").join("plugins"),
            )
        } else {
            (
                Path::new(&dst_dir_path)
                    .join(format!("avalanchego-{}", release_info.tag_name))
                    .join("avalanchego"),
                Path::new(&dst_dir_path)
                    .join(format!("avalanchego-{}", release_info.tag_name))
                    .join("plugins"),
            )
        }
    };

    {
        let f = File::open(&avalanchego_path)?;
        f.set_permissions(PermissionsExt::from_mode(0o777))?;
    }
    Ok((
        String::from(avalanchego_path.as_os_str().to_str().unwrap()),
        String::from(plugins_dir.as_os_str().to_str().unwrap()),
    ))
}

///  build
///    ├── avalanchego (the binary from compiling the app directory)
///    └── plugins
///        └── evm
pub fn get_plugins_dir<P: AsRef<Path>>(avalanche_bin: P) -> String {
    let parent_dir = avalanche_bin.as_ref().parent().unwrap();
    String::from(
        parent_dir
            .join(Path::new("plugins"))
            .as_path()
            .to_str()
            .unwrap(),
    )
}
