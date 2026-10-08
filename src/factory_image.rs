// SPDX-FileCopyrightText: 2026 Andrew Gunnerson
// SPDX-License-Identifier: GPL-3.0-only

use std::fs::{self, File};
use std::io;
use std::path::Path;
use zip::CompressionMethod;
use zip::ZipArchive;

use crate::dynamic_partitions::validate_partition_name;
use crate::error::Error;
use crate::partial_reader::BoundedReader;

/// Extracts required partition images from the nested image-*.zip inside a factory ZIP.
/// super_empty.img is extracted to super_dir, and other partitions to images_dir.
/// The nested ZIP is read directly via BoundedReader without extracting it to disk.
pub fn extract_factory_image(
    factory: &Path,
    images_dir: &Path,
    super_dir: &Path,
) -> Result<(), Error> {
    let file = File::open(factory)?;
    let mut outer_zip = ZipArchive::new(file)?;

    let mut image_zip_offset = None;
    let mut image_zip_size = None;

    for i in 0..outer_zip.len() {
        let entry = outer_zip.by_index(i)?;
        let entry_name = entry.name().to_string();
        let path = Path::new(&entry_name);
        let name = path.file_name().and_then(|f| f.to_str()).unwrap_or("");

        if !name.starts_with("image-") || !name.ends_with(".zip") {
            continue;
        }

        if entry.compression() != CompressionMethod::Stored {
            return Err(Error::ImageZipNotUncompressed(name.to_string()));
        }

        let size = entry.size();
        let offset = entry.data_start().ok_or_else(|| {
            Error::Other(format!(
                "Missing data offset for entry {name} in factory zip"
            ))
        })?;

        image_zip_size = Some(size);
        image_zip_offset = Some(offset);
        break;
    }

    let (offset, size) = match (image_zip_offset, image_zip_size) {
        (Some(o), Some(s)) => (o, s),
        _ => return Err(Error::ImageZipNotFound),
    };

    let underlying_file = outer_zip.into_inner();
    let bounded = BoundedReader::new(underlying_file, offset, size)?;
    let mut nested_zip = ZipArchive::new(bounded)?;

    for i in 0..nested_zip.len() {
        let mut entry = nested_zip.by_index(i)?;
        let entry_name = entry.name().to_string();
        let path = Path::new(&entry_name);
        let name = match path.file_name().and_then(|f| f.to_str()) {
            Some(n) if !n.is_empty() => n,
            _ => continue,
        };

        if matches!(
            name,
            "android-info.txt"
                | "fastboot-info.txt"
                | "kernel_16k"
                | "ramdisk_16k.img"
                | "system_other.img"
                | "userdata_exp.ai.img"
        ) {
            continue;
        }

        validate_partition_name(name)?;

        let output_dir = if name == "super_empty.img" {
            super_dir
        } else {
            images_dir
        };

        let output_file = output_dir.join(name);
        if output_file.exists() {
            continue;
        }

        println!("Extracting from factory image: {name}");

        let extract_result = (|| -> Result<(), io::Error> {
            let mut out = File::create(&output_file)?;
            io::copy(&mut entry, &mut out)?;
            out.sync_all()?;
            Ok(())
        })();

        if let Err(e) = extract_result {
            let _ = fs::remove_file(&output_file);
            return Err(Error::Io(e));
        }
    }

    Ok(())
}
