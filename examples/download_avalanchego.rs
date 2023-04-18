use std::io;

use avalanche_installer::avalanchego::github;

/// cargo run --example download_avalanchego
#[tokio::main]
async fn main() -> io::Result<()> {
    // ref. https://github.com/env-logger-rs/env_logger/issues/47
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let avalanchego_path = github::download_latest(None, None).await.unwrap();
    log::info!("avalanchego path: {}", avalanchego_path);

    Ok(())
}
