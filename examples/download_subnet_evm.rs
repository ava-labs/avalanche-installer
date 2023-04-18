use std::io;

use avalanche_installer::subnet_evm::github;

/// cargo run --example download_subnet_evm
#[tokio::main]
async fn main() -> io::Result<()> {
    // ref. https://github.com/env-logger-rs/env_logger/issues/47
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let subnet_evm_path = random_manager::tmp_path(10, None).unwrap();
    github::download_latest(None, None, &subnet_evm_path)
        .await
        .unwrap();
    log::info!("subnet_evm path: {}", subnet_evm_path);

    Ok(())
}
