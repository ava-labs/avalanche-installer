use log::info;

/// cargo run --example random_manager
fn main() {
    // ref. https://github.com/env-logger-rs/env_logger/issues/47
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let random1 = random_manager::string(100);
    let random2 = random_manager::string(100);
    info!("random1: {}", random1);
    info!("random2: {}", random2);
}
