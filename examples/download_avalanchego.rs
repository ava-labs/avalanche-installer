use avalanche_installer::avalanchego;

/// cargo run --example download_avalanchego
fn main() {
    // ref. https://github.com/env-logger-rs/env_logger/issues/47
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    macro_rules! ab {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    let (avalanchego_path, plugins_dir) = ab!(avalanchego::download_latest(None, None)).unwrap();
    log::info!("avalanchego path: {}", avalanchego_path);
    log::info!("plugins path: {}", plugins_dir);
}
