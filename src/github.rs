use std::io::{self, Error, ErrorKind};

use serde::{Deserialize, Serialize};

/// # Examples
///
/// To download with bash:
///
/// ```
/// VERSION=1.9.7
/// rm -rf /tmp/avalanchego.tar.gz /tmp/avalanchego-v${VERSION}
/// curl -L ${DOWNLOAD_URL}/v${VERSION}/avalanchego-linux-amd64-v${VERSION}.tar.gz -o /tmp/avalanchego.tar.gz
/// tar xzvf /tmp/avalanchego.tar.gz -C /tmp
/// find /tmp/avalanchego-v${VERSION}
/// ```
///
/// ref. https://github.com/ava-labs/avalanchego/releases
/// ref. https://api.github.com/repos/ava-labs/avalanchego/releases/latest
pub async fn fetch_latest_release(org: &str, repo: &str) -> io::Result<ReleaseResponse> {
    let ep = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        org, repo
    );
    log::info!("fetching {}", ep);

    let rb = http_manager::get_non_tls(&ep, "").await?;
    let resp: ReleaseResponse = match serde_json::from_slice(&rb) {
        Ok(p) => p,
        Err(e) => {
            return Err(Error::new(
                ErrorKind::Other,
                format!("failed to decode {}", e),
            ));
        }
    };
    Ok(resp)
}

/// ref. https://api.github.com/repos/ava-labs/avalanchego/releases/latest
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ReleaseResponse {
    /// Sometimes empty for github API consistency issue.
    pub tag_name: Option<String>,
    /// Sometimes empty for github API consistency issue.
    pub assets: Option<Vec<Asset>>,

    #[serde(default)]
    pub prerelease: bool,
}

impl Default for ReleaseResponse {
    fn default() -> Self {
        Self::default()
    }
}

impl ReleaseResponse {
    pub fn default() -> Self {
        Self {
            tag_name: None,
            assets: None,
            prerelease: false,
        }
    }
}

/// ref. https://api.github.com/repos/ava-labs/avalanchego/releases/latest
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}
