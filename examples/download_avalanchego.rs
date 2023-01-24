use std::{
    env, fs,
    io::{self, Write},
    sync::Arc,
};

use avalanche_installer::avalanchego::{github, s3 as avalanchego_s3};
use aws_manager::{self, s3};
use tokio::time::{sleep, Duration};

/// cargo run --example download_avalanchego
#[tokio::main]
async fn main() -> io::Result<()> {
    // ref. https://github.com/env-logger-rs/env_logger/issues/47
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let avalanchego_path = github::download_latest(None, None).await.unwrap();
    log::info!("avalanchego path: {}", avalanchego_path);

    let shared_config = aws_manager::load_config(Some(String::from("us-east-1")))
        .await
        .unwrap();
    let s3_manager = s3::Manager::new(&shared_config);
    let s3_bucket = format!("installer-{}", random_manager::string(10).to_lowercase());

    s3_manager.create_bucket(&s3_bucket).await.unwrap();

    sleep(Duration::from_secs(2)).await;
    let avalanchego_s3_key = "sub-dir/avalanchego".to_string();
    s3_manager
        .put_object(
            Arc::new(avalanchego_path.clone()),
            Arc::new(s3_bucket.clone()),
            Arc::new(avalanchego_s3_key.clone()),
        )
        .await
        .unwrap();

    let contents = vec![7; 1024];
    let mut upload_file = tempfile::NamedTempFile::new().unwrap();
    upload_file.write_all(&contents.to_vec()).unwrap();
    let upload_file_path = upload_file.path().to_str().unwrap().to_string();
    let plugin_s3_key = "sub-dir/plugin/aaa".to_string();
    s3_manager
        .put_object(
            Arc::new(upload_file_path.clone()),
            Arc::new(s3_bucket.clone()),
            Arc::new(plugin_s3_key.clone()),
        )
        .await
        .unwrap();

    sleep(Duration::from_secs(5)).await;
    let target_avalanchego_bin_path = random_manager::tmp_path(15, None)?;
    let target_plugin_dir = env::temp_dir().as_os_str().to_str().unwrap().to_string();
    avalanchego_s3::download_avalanche_and_plugins(
        true,
        Arc::new(s3_manager.clone()),
        &s3_bucket,
        &avalanchego_s3_key,
        &target_avalanchego_bin_path,
        "sub-dir/plugin",
        &target_plugin_dir,
    )
    .await
    .unwrap();

    log::info!("removing {target_avalanchego_bin_path}");
    fs::remove_file(&target_avalanchego_bin_path)?;

    s3_manager
        .delete_objects(Arc::new(s3_bucket.clone()), None)
        .await
        .unwrap();

    sleep(Duration::from_secs(2)).await;
    s3_manager.delete_bucket(&s3_bucket).await.unwrap();

    Ok(())
}
