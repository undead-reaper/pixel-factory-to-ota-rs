fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=protobuf/apex_manifest.proto");
    println!("cargo:rerun-if-changed=protobuf/ota_metadata.proto");

    if std::env::var_os("PROTOC").is_none()
        && let Ok(protoc_path) = protoc_bin_vendored::protoc_bin_path()
    {
        // SAFETY: In build script main, no concurrent threads are accessing environment variables.
        unsafe {
            std::env::set_var("PROTOC", protoc_path);
        }
    }

    prost_build::Config::new().compile_protos(
        &[
            "protobuf/apex_manifest.proto",
            "protobuf/ota_metadata.proto",
        ],
        &["protobuf"],
    )?;

    Ok(())
}
