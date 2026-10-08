fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=protobuf/apex_manifest.proto");
    println!("cargo:rerun-if-changed=protobuf/ota_metadata.proto");

    prost_build::Config::new().compile_protos(
        &[
            "protobuf/apex_manifest.proto",
            "protobuf/ota_metadata.proto",
        ],
        &["protobuf"],
    )?;

    Ok(())
}
