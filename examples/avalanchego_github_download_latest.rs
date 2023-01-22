use std::io;

use avalanche_installer::avalanchego::github;

/// cargo run --example avalanchego_github_download_latest
#[tokio::main]
async fn main() -> io::Result<()> {
    // ref. https://github.com/env-logger-rs/env_logger/issues/47
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let (avalanchego_path, plugins_dir) = github::download_latest(None, None).await.unwrap();
    log::info!("avalanchego path: {}", avalanchego_path);
    log::info!("plugins path: {}", plugins_dir);

    Ok(())
}
