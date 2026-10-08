// SPDX-FileCopyrightText: 2026 Andrew Gunnerson
// SPDX-License-Identifier: GPL-3.0-only

use pixel_factory_to_ota::generate_ota;
use pixel_factory_to_ota::protobuf::apex::proto::ApexManifest;
use prost::Message;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

fn has_tool(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn make_appended_avb(raw_path: &Path, out_path: &Path, part_name: &str, fp: &str) {
    let size = fs::metadata(raw_path).unwrap().len();
    let avb_toml_path = raw_path.with_extension("avb.toml");
    let avb_toml = format!(
        r#"image_size = 0
[header]
required_libavb_version_major = 1
required_libavb_version_minor = 0
authentication_data_size = 0
auxiliary_data_size = 0
algorithm_type = "None"
hash = ""
signature = ""
public_key = ""
public_key_metadata = ""
rollback_index = 0
rollback_index_location = 0
release_string = ""
flags = 0
reserved = "{}"

[[header.descriptors]]
type = "Property"
key = "com.android.build.{}.fingerprint"
value = "{}"

[[header.descriptors]]
type = "HashTree"
dm_verity_version = 1
image_size = {}
tree_offset = 0
tree_size = 0
data_block_size = 4096
hash_block_size = 4096
fec_num_roots = 0
fec_offset = 0
fec_size = 0
hash_algorithm = "sha256"
partition_name = "{}"
salt = ""
root_digest = ""
flags = 0
reserved = "{}"

[footer]
version_major = 1
version_minor = 0
original_image_size = {}
vbmeta_offset = 0
vbmeta_size = 0
reserved = "{}"
"#,
        "00".repeat(80),
        part_name,
        fp,
        size,
        part_name,
        "00".repeat(60),
        size,
        "00".repeat(28)
    );

    fs::write(&avb_toml_path, avb_toml).unwrap();
    let status = Command::new("avbroot")
        .args([
            "avb",
            "pack",
            "--recompute-size",
            "--input-info",
            avb_toml_path.to_str().unwrap(),
            "--input-raw",
            raw_path.to_str().unwrap(),
            "--output",
            out_path.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn test_end_to_end_pipeline() {
    if !has_tool("avbroot") || !has_tool("afsr") || !has_tool("lpmake") || !has_tool("mke2fs") {
        eprintln!("Skipping test_end_to_end_pipeline because required tools are missing");
        return;
    }

    let td = tempdir().unwrap();
    let td_path = td.path();

    // 1. Build test APEX
    let manifest = ApexManifest {
        name: "com.android.runtime".to_string(),
        version: 350000000,
        ..Default::default()
    };
    let mut manifest_bytes = Vec::new();
    manifest.encode(&mut manifest_bytes).unwrap();
    let apex_path = td_path.join("com.android.runtime.apex");
    {
        let mut z = ZipWriter::new(File::create(&apex_path).unwrap());
        let opts = SimpleFileOptions::default();
        z.start_file("apex_manifest.pb", opts).unwrap();
        z.write_all(&manifest_bytes).unwrap();
        z.start_file("original_apex", opts).unwrap();
        z.write_all(b"original uncompressed apex content").unwrap();
        z.finish().unwrap();
    }

    let fp = "google/akita/akita:15/AP4A.250305.001/12345678:user/release-keys";

    // 2. Build system raw & appended ext4 image
    let sys_root = td_path.join("sys_root");
    fs::create_dir_all(sys_root.join("system/apex")).unwrap();
    fs::write(
        sys_root.join("system/build.prop"),
        format!(
            r#"ro.build.version.security_patch=2026-03-05
ro.build.version.incremental=12345678
ro.build.date.utc=1741132800
ro.build.version.sdk=35
ro.product.system.device=akita
ro.system.build.fingerprint={fp}
ro.system.build.date.utc=1741132800
"#
        ),
    )
    .unwrap();
    fs::copy(
        &apex_path,
        sys_root.join("system/apex/com.android.runtime.apex"),
    )
    .unwrap();

    let system_raw = td_path.join("system_raw.img");
    let status = Command::new("mke2fs")
        .args([
            "-t",
            "ext4",
            "-d",
            sys_root.to_str().unwrap(),
            system_raw.to_str().unwrap(),
            "25M",
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let system_img = td_path.join("system.img");
    make_appended_avb(&system_raw, &system_img, "system", fp);

    // 3. Build vendor raw & appended ext4 image
    let vendor_root = td_path.join("vendor_root");
    fs::create_dir_all(&vendor_root).unwrap();
    fs::write(
        vendor_root.join("build.prop"),
        format!(
            r#"ro.product.vendor.device=akita
ro.vendor.build.fingerprint={fp}
ro.vendor.build.date.utc=1741132800
"#
        ),
    )
    .unwrap();

    let vendor_raw = td_path.join("vendor_raw.img");
    let status = Command::new("mke2fs")
        .args([
            "-t",
            "ext4",
            "-d",
            vendor_root.to_str().unwrap(),
            vendor_raw.to_str().unwrap(),
            "15M",
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let vendor_img = td_path.join("vendor.img");
    make_appended_avb(&vendor_raw, &vendor_img, "vendor", fp);

    // 4. Build super_empty.img
    let super_empty = td_path.join("super_empty.img");
    let status = Command::new("lpmake")
        .args([
            "--device-size=8000000000",
            "--metadata-size=65536",
            "--metadata-slots=3",
            "--group=google_dynamic_partitions_a:7500000000",
            "--partition=system_a:readonly:0:google_dynamic_partitions_a",
            "--partition=vendor_a:readonly:0:google_dynamic_partitions_a",
            &format!("--output={}", super_empty.display()),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    // 5. Build vbmeta.img
    let avb_toml = td_path.join("avb.toml");
    fs::write(
        &avb_toml,
        format!(
            r#"image_size = 4096
[header]
required_libavb_version_major = 1
required_libavb_version_minor = 0
authentication_data_size = 0
auxiliary_data_size = 0
algorithm_type = "None"
hash = ""
signature = ""
public_key = ""
public_key_metadata = ""
descriptors = []
rollback_index = 0
rollback_index_location = 0
release_string = ""
flags = 0
reserved = "{}"
"#,
            "00".repeat(80)
        ),
    )
    .unwrap();
    let vbmeta_img = td_path.join("vbmeta.img");
    let status = Command::new("avbroot")
        .args([
            "avb",
            "pack",
            "--input-info",
            avb_toml.to_str().unwrap(),
            "--output",
            vbmeta_img.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    // 6. Build nested image-akita.zip (STORED)
    let image_zip = td_path.join("image-akita.zip");
    {
        let mut z = ZipWriter::new(File::create(&image_zip).unwrap());
        let stored =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        z.start_file("super_empty.img", stored).unwrap();
        std::io::copy(&mut File::open(&super_empty).unwrap(), &mut z).unwrap();

        z.start_file("system.img", stored).unwrap();
        std::io::copy(&mut File::open(&system_img).unwrap(), &mut z).unwrap();

        z.start_file("vendor.img", stored).unwrap();
        std::io::copy(&mut File::open(&vendor_img).unwrap(), &mut z).unwrap();

        z.start_file("vbmeta.img", stored).unwrap();
        std::io::copy(&mut File::open(&vbmeta_img).unwrap(), &mut z).unwrap();

        z.start_file("boot.img", stored).unwrap();
        z.write_all(&vec![0u8; 4096 * 10]).unwrap();

        z.start_file("android-info.txt", stored).unwrap();
        z.write_all(b"require board=akita\n").unwrap();

        z.start_file("fastboot-info.txt", stored).unwrap();
        z.write_all(b"flash boot\n").unwrap();

        z.finish().unwrap();
    }

    // 7. Build outer factory.zip
    let factory_zip = td_path.join("factory.zip");
    {
        let mut z = ZipWriter::new(File::create(&factory_zip).unwrap());
        let stored =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        z.start_file("image-akita.zip", stored).unwrap();
        std::io::copy(&mut File::open(&image_zip).unwrap(), &mut z).unwrap();

        let deflated =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        z.start_file("bootloader-akita.img", deflated).unwrap();
        z.write_all(b"bootloader").unwrap();

        z.start_file("radio-akita.img", deflated).unwrap();
        z.write_all(b"radio").unwrap();

        z.finish().unwrap();
    }

    // 8. Run Rust generate_ota
    let work_dir = td_path.join("work_dir");
    let output_ota = td_path.join("output_ota.zip");
    generate_ota(&factory_zip, &output_ota, &work_dir).unwrap();

    // Verify output artifacts
    assert!(output_ota.exists());
    assert!(work_dir.join("ota/payload.toml").exists());
    assert!(work_dir.join("ota/ota.toml").exists());
    assert!(work_dir.join("ota/ota_files/apex_info.pb").exists());
    assert!(work_dir.join("ota.key").exists());
    assert!(work_dir.join("ota.crt").exists());

    // Verify OTA ZIP contains expected files
    let mut zip = ZipArchive::new(File::open(&output_ota).unwrap()).unwrap();
    let mut names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .collect();
    names.sort();

    assert!(names.contains(&"apex_info.pb".to_string()));
    assert!(names.contains(&"payload.bin".to_string()));
    assert!(names.contains(&"payload_properties.txt".to_string()));
    assert!(names.contains(&"META-INF/com/android/metadata".to_string()));
}
