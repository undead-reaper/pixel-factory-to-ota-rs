// SPDX-FileCopyrightText: 2026 Andrew Gunnerson
// SPDX-License-Identifier: GPL-3.0-only

use std::fs;
use std::path::Path;

use crate::apex::parse_apex_infos;
use crate::build_prop::parse_build_props;
use crate::command::{cmd, to_absolute};
use crate::dynamic_partitions::parse_dynamic_partitions;
use crate::error::Error;
use crate::factory_image::extract_factory_image;
use crate::ota::{create_apex_info, create_ota_toml};
use crate::payload::create_payload_toml;

/// Executes the full conversion pipeline from Pixel factory ZIP to signed A/B OTA ZIP.
pub fn generate_ota(input_factory: &Path, output_ota: &Path, work_dir: &Path) -> Result<(), Error> {
    let ota_dir = work_dir.join("ota");
    fs::create_dir_all(&ota_dir)?;

    let ota_files = ota_dir.join("ota_files");
    fs::create_dir_all(&ota_files)?;

    let payload_images = ota_dir.join("payload_images");
    fs::create_dir_all(&payload_images)?;

    let super_dir = work_dir.join("super");
    fs::create_dir_all(&super_dir)?;

    let filesystems = work_dir.join("filesystems");
    fs::create_dir_all(&filesystems)?;

    // We only need the images from the nested image-*.zip within the factory
    // zip. It includes the separated bootloader and modem partitions so it is
    // not necessary to unpack Google's proprietary fbpack format from
    // bootloader-*.img and radio-*.img.
    extract_factory_image(input_factory, &payload_images, &super_dir)?;

    // The dynamic partition group metadata is needed to create the payload.
    let super_toml = super_dir.join("lp.toml");
    if !super_toml.exists() {
        cmd(
            ["avbroot", "lp", "unpack", "-q", "-i", "super_empty.img"],
            Some(&super_dir),
        )?;
    }

    let dpg = parse_dynamic_partitions(&super_toml)?;

    for name in &dpg.partition_names {
        let extracted = filesystems.join(name);
        fs::create_dir_all(&extracted)?;

        if !extracted.join("fs_metadata.toml").exists() {
            let img_path = to_absolute(&payload_images.join(format!("{name}.img")))?;
            cmd(
                [
                    "afsr".as_ref(),
                    "unpack".as_ref(),
                    "-i".as_ref(),
                    img_path.as_os_str(),
                ],
                Some(&extracted),
            )?;
        }
    }

    let build_props = parse_build_props(&filesystems, &dpg)?;
    let apex_infos = parse_apex_infos(&filesystems, &dpg)?;

    let payload_toml = ota_dir.join("payload.toml");
    create_payload_toml(
        &payload_images,
        &dpg,
        &build_props,
        &apex_infos,
        &payload_toml,
    )?;

    let ota_toml = ota_dir.join("ota.toml");
    create_ota_toml(&dpg, &build_props, &ota_toml)?;

    let apex_info_pb = ota_files.join("apex_info.pb");
    create_apex_info(&apex_infos, &apex_info_pb)?;

    // We intentionally use an ephemeral signing keypair. The output is not meant
    // to be flashed. It must be patched with avbroot first.
    let ota_key = work_dir.join("ota.key");
    let ota_cert = work_dir.join("ota.crt");

    if !ota_key.exists() {
        let abs_ota_key = to_absolute(&ota_key)?;
        cmd(
            [
                "avbroot".as_ref(),
                "key".as_ref(),
                "generate-key".as_ref(),
                "-t".as_ref(),
                "rsa4096".as_ref(),
                "-o".as_ref(),
                abs_ota_key.as_os_str(),
                "--pass-file".as_ref(),
                "/dev/null".as_ref(),
            ],
            None,
        )?;
    }

    if !ota_cert.exists() {
        let abs_ota_key = to_absolute(&ota_key)?;
        let abs_ota_cert = to_absolute(&ota_cert)?;
        cmd(
            [
                "avbroot".as_ref(),
                "key".as_ref(),
                "generate-cert".as_ref(),
                "-k".as_ref(),
                abs_ota_key.as_os_str(),
                "-o".as_ref(),
                abs_ota_cert.as_os_str(),
            ],
            None,
        )?;
    }

    // Pixel devices include bootloader partitions in the AVB metadata, so
    // verifying ensures they were not omitted.
    let abs_vbmeta = to_absolute(&payload_images.join("vbmeta.img"))?;
    cmd(
        [
            "avbroot".as_ref(),
            "avb".as_ref(),
            "verify".as_ref(),
            "-i".as_ref(),
            abs_vbmeta.as_os_str(),
        ],
        None,
    )?;

    // Create the OTA!
    let abs_output_ota = to_absolute(output_ota)?;
    let abs_ota_key = to_absolute(&ota_key)?;
    let abs_ota_cert = to_absolute(&ota_cert)?;
    cmd(
        [
            "avbroot".as_ref(),
            "zip".as_ref(),
            "pack".as_ref(),
            "-q".as_ref(),
            "-o".as_ref(),
            abs_output_ota.as_os_str(),
            "-k".as_ref(),
            abs_ota_key.as_os_str(),
            "-c".as_ref(),
            abs_ota_cert.as_os_str(),
            "--payload".as_ref(),
        ],
        Some(&ota_dir),
    )?;

    Ok(())
}
