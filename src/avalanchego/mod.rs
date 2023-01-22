pub mod github;
pub mod s3;

use std::path::Path;

///  build
///    ├── avalanchego (the binary from compiling the app directory)
///    └── plugins
///        └── evm
///        └── abc
pub fn get_plugin_dir<P: AsRef<Path>>(avalanche_bin: P) -> String {
    let parent_dir = avalanche_bin.as_ref().parent().unwrap();
    String::from(
        parent_dir
            .join(Path::new("plugins"))
            .as_path()
            .to_str()
            .unwrap(),
    )
}
