// Build script for auto-generating Rust code from Protobuf schemas
// Runs at compile time via prost-build

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Watch all proto files for changes
    println!("cargo:rerun-if-changed=../shared/proto/game_state.proto");
    println!("cargo:rerun-if-changed=../shared/proto/entities.proto");
    println!("cargo:rerun-if-changed=../shared/proto/economy.proto");
    println!("cargo:rerun-if-changed=../shared/proto/social.proto");
    println!("cargo:rerun-if-changed=../shared/proto/quests.proto");

    // Use downloaded protoc binary from .tools directory
    let protoc_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join(".tools/protoc/bin/protoc.exe");

    if protoc_path.exists() {
        std::env::set_var("PROTOC", protoc_path);
    }

    let mut config = prost_build::Config::new();

    // Add serde derives for all Protobuf types (for FFI JSON conversion + LMDB storage)
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");

    // Compile all proto schemas
    config.compile_protos(
        &[
            "../shared/proto/game_state.proto",
            "../shared/proto/entities.proto",
            "../shared/proto/economy.proto",
            "../shared/proto/social.proto",
            "../shared/proto/quests.proto",
        ],
        &["../shared/proto/"],
    )?;

    Ok(())
}
