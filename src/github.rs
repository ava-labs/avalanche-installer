use std::{
    io::{self, Error, ErrorKind},
    time::Duration,
};

use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};

/// ref. https://github.com/ava-labs/avalanchego/releases
/// ref. https://api.github.com/repos/ava-labs/avalanchego/releases/latest
pub async fn fetch_latest_release(org: &str, repo: &str) -> io::Result<ReleaseResponse> {
    let ep = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        org, repo
    );
    log::info!("fetching {}", ep);

    let cli = ClientBuilder::new()
        .user_agent(env!("CARGO_PKG_NAME"))
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(15))
        .connection_verbose(true)
        .build()
        .map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("failed ClientBuilder build {}", e),
            )
        })?;
    let resp =
        cli.get(&ep).send().await.map_err(|e| {
            Error::new(ErrorKind::Other, format!("failed ClientBuilder send {}", e))
        })?;
    let out = resp
        .bytes()
        .await
        .map_err(|e| Error::new(ErrorKind::Other, format!("failed ClientBuilder send {}", e)))?;
    let out: Vec<u8> = out.into();

    let resp: ReleaseResponse = match serde_json::from_slice(&out) {
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
