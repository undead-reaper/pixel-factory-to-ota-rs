// SPDX-FileCopyrightText: 2026 Andrew Gunnerson
// SPDX-License-Identifier: GPL-3.0-only

use pixel_factory_to_ota::apex::parse_apex;
use pixel_factory_to_ota::build_prop::parse_build_prop;
use pixel_factory_to_ota::dynamic_partitions::validate_partition_name;
use pixel_factory_to_ota::models::*;
use pixel_factory_to_ota::ota::{create_apex_info, create_ota_toml};
use pixel_factory_to_ota::partial_reader::BoundedReader;
use pixel_factory_to_ota::payload::create_payload_toml;
use pixel_factory_to_ota::protobuf::apex::proto::ApexManifest;
use pixel_factory_to_ota::protobuf::build::tools::releasetools::ApexMetadata;
use prost::Message;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use tempfile::NamedTempFile;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

#[test]
fn test_bounded_reader_comprehensive() {
    let raw = b"ABCDEFGHIJ0123456789KLMNOPQRST";
    let cursor = Cursor::new(raw.to_vec());
    let mut reader = BoundedReader::new(cursor, 10, 10).unwrap();

    assert_eq!(reader.start(), 10);
    assert_eq!(reader.size(), 10);
    assert_eq!(reader.position(), 0);

    // Read full slice
    let mut buf = vec![0u8; 10];
    reader.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, b"0123456789");
    assert_eq!(reader.position(), 10);

    // EOF on further reads
    assert_eq!(reader.read(&mut buf).unwrap(), 0);

    // Seek back to start
    reader.seek(SeekFrom::Start(0)).unwrap();
    assert_eq!(reader.position(), 0);

    // Seek from current
    reader.seek(SeekFrom::Current(5)).unwrap();
    assert_eq!(reader.position(), 5);

    let mut small = [0u8; 3];
    reader.read_exact(&mut small).unwrap();
    assert_eq!(&small, b"567");

    // Seek from end
    reader.seek(SeekFrom::End(-2)).unwrap();
    let mut end_buf = [0u8; 2];
    reader.read_exact(&mut end_buf).unwrap();
    assert_eq!(&end_buf, b"89");

    // Invalid negative seek
    assert!(reader.seek(SeekFrom::Start(0)).is_ok());
    assert!(reader.seek(SeekFrom::Current(-1)).is_err());
    assert!(reader.seek(SeekFrom::End(-11)).is_err());
}

#[test]
fn test_dynamic_partition_name_validation() {
    assert!(validate_partition_name("system").is_ok());
    assert!(validate_partition_name("system_ext").is_ok());
    assert!(validate_partition_name("product").is_ok());
    assert!(validate_partition_name("vendor_dlkm").is_ok());

    assert!(validate_partition_name("../system").is_err());
    assert!(validate_partition_name("system/nested").is_err());
    assert!(validate_partition_name("/absolute").is_err());
    assert!(validate_partition_name(".").is_err());
    assert!(validate_partition_name("..").is_err());
    assert!(validate_partition_name("").is_err());
    assert!(validate_partition_name("foo\\bar").is_err());
}

#[test]
fn test_build_prop_parsing_parity() {
    let content = r#"
# Build property header
ro.build.id=AP4A.250305.001
ro.build.version.incremental=12345
ro.build.date.utc=1741132800

# Overwrite test
ro.test.key=initial_value
ro.test.key=overwritten_value

# Whitespace handling
  ro.whitespace.test  =  trimmed_value  

# Empty value
ro.empty=
"#;

    let mut temp = NamedTempFile::new().unwrap();
    write!(temp, "{}", content).unwrap();

    let props = parse_build_prop(temp.path()).unwrap();
    assert_eq!(props.get("ro.build.id").unwrap(), "AP4A.250305.001");
    assert_eq!(props.get("ro.build.version.incremental").unwrap(), "12345");
    assert_eq!(props.get("ro.build.date.utc").unwrap(), "1741132800");
    assert_eq!(props.get("ro.test.key").unwrap(), "overwritten_value");
    assert_eq!(
        props.get("ro.whitespace.test  ").unwrap(),
        "  trimmed_value"
    );
    assert_eq!(props.get("ro.empty").unwrap(), "");
}

#[test]
fn test_apex_manifest_and_original_decompression() {
    let manifest = ApexManifest {
        name: "com.android.art".to_string(),
        version: 350000000,
        ..Default::default()
    };
    let mut manifest_bytes = Vec::new();
    manifest.encode(&mut manifest_bytes).unwrap();

    let temp = NamedTempFile::new().unwrap();
    let mut zip = ZipWriter::new(File::create(temp.path()).unwrap());
    let opts = SimpleFileOptions::default();

    zip.start_file("apex_manifest.pb", opts).unwrap();
    zip.write_all(&manifest_bytes).unwrap();

    zip.start_file("original_apex", opts).unwrap();
    zip.write_all(b"decompressed_apex_binary_stream_here")
        .unwrap();

    zip.finish().unwrap();

    let apex_info = parse_apex(temp.path()).unwrap();
    assert_eq!(apex_info.package_name, Some("com.android.art".into()));
    assert_eq!(apex_info.version, Some(350000000));
    assert_eq!(apex_info.is_compressed, Some(true));
    assert_eq!(apex_info.decompressed_size, Some(36));
}

#[test]
fn test_payload_and_ota_toml_models_serialization() {
    let dpg = PayloadDynamicPartitionGroup {
        name: "google_dynamic_partitions".into(),
        size: Some(8000000000),
        partition_names: vec!["system".into(), "vendor".into()],
    };

    let mut system_props = HashMap::new();
    system_props.insert(
        "ro.build.version.security_patch".into(),
        "2026-03-05".into(),
    );
    system_props.insert("ro.system.build.date.utc".into(), "1741132800".into());
    system_props.insert("ro.build.version.incremental".into(), "12345678".into());
    system_props.insert("ro.build.date.utc".into(), "1741132800".into());
    system_props.insert("ro.build.version.sdk".into(), "35".into());
    system_props.insert("ro.product.system.device".into(), "akita".into());
    system_props.insert(
        "ro.system.build.fingerprint".into(),
        "google/akita:15".into(),
    );

    let mut vendor_props = HashMap::new();
    vendor_props.insert("ro.product.vendor.device".into(), "akita".into());
    vendor_props.insert(
        "ro.vendor.build.fingerprint".into(),
        "google/akita:15".into(),
    );
    vendor_props.insert("ro.vendor.build.date.utc".into(), "1741132800".into());

    let mut build_props = HashMap::new();
    build_props.insert("system".into(), system_props);
    build_props.insert("vendor".into(), vendor_props);

    let temp_dir = tempfile::tempdir().unwrap();
    let payload_images = temp_dir.path().join("payload_images");
    fs::create_dir_all(&payload_images).unwrap();

    File::create(payload_images.join("boot.img")).unwrap();
    File::create(payload_images.join("system.img")).unwrap();
    File::create(payload_images.join("vendor.img")).unwrap();

    let apex_infos = vec![PayloadApexInfo {
        package_name: Some("com.android.art".into()),
        version: Some(350000000),
        is_compressed: Some(true),
        decompressed_size: Some(50000),
    }];

    let payload_toml = temp_dir.path().join("payload.toml");
    create_payload_toml(
        &payload_images,
        &dpg,
        &build_props,
        &apex_infos,
        &payload_toml,
    )
    .unwrap();

    let payload_content = fs::read_to_string(&payload_toml).unwrap();
    assert!(payload_content.contains("version = 2"));
    assert!(payload_content.contains("security_patch_level = \"2026-03-05\""));
    assert!(payload_content.contains("otapreopt_script"));
    assert!(payload_content.contains("checkpoint_gc"));

    let ota_toml = temp_dir.path().join("ota.toml");
    create_ota_toml(&dpg, &build_props, &ota_toml).unwrap();

    let ota_content = fs::read_to_string(&ota_toml).unwrap();
    assert!(ota_content.contains("type = \"AB\""));
    assert!(ota_content.contains("wipe = false"));
    assert!(ota_content.contains("google/akita:15"));

    let apex_pb_path = temp_dir.path().join("apex_info.pb");
    create_apex_info(&apex_infos, &apex_pb_path).unwrap();

    let pb_bytes = fs::read(&apex_pb_path).unwrap();
    let decoded = ApexMetadata::decode(&*pb_bytes).unwrap();
    assert_eq!(decoded.apex_info.len(), 1);
    assert_eq!(decoded.apex_info[0].package_name, "com.android.art");
    assert_eq!(decoded.apex_info[0].version, 350000000);
    assert!(decoded.apex_info[0].is_compressed);
    assert_eq!(decoded.apex_info[0].decompressed_size, 50000);
}
