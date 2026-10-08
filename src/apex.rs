// SPDX-FileCopyrightText: 2026 Andrew Gunnerson
// SPDX-License-Identifier: GPL-3.0-only

use prost::Message;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

use crate::error::Error;
use crate::models::{PayloadApexInfo, PayloadDynamicPartitionGroup};
use crate::protobuf::apex::proto::ApexManifest;

/// Inspects an APEX or CAPEX zip file.
/// If 'original_apex' is present, marks the APEX as compressed and records its decompressed size.
/// Parses 'apex_manifest.pb' to retrieve package name and version.
pub fn parse_apex(path: &Path) -> Result<PayloadApexInfo, Error> {
    let file = File::open(path)?;
    let mut zip = ZipArchive::new(file)?;

    let mut apex_info = PayloadApexInfo::default();

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry.name().to_string();

        if name == "original_apex" {
            apex_info.is_compressed = Some(true);
            apex_info.decompressed_size = Some(entry.size());
        } else if name == "apex_manifest.pb" {
            let mut buf = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut buf)?;

            let manifest = ApexManifest::decode(&*buf)?;
            apex_info.package_name = Some(manifest.name);
            apex_info.version = Some(manifest.version);
        }
    }

    Ok(apex_info)
}

/// Scans dynamic partitions for APEX files.
/// System partition looks in fs_tree/system/apex,
/// other partitions look in fs_tree/apex.
/// Files ending in .apex or .capex are sorted deterministically and inspected.
pub fn parse_apex_infos(
    filesystems: &Path,
    dpg: &PayloadDynamicPartitionGroup,
) -> Result<Vec<PayloadApexInfo>, Error> {
    let mut apex_infos = Vec::new();

    for name in &dpg.partition_names {
        let fs_tree = filesystems.join(name).join("fs_tree");

        let apex_dir = if name == "system" {
            fs_tree.join("system").join("apex")
        } else {
            fs_tree.join("apex")
        };

        if !apex_dir.exists() {
            continue;
        }

        let mut apex_paths = Vec::new();
        for entry in std::fs::read_dir(&apex_dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or_default();

            if file_name.ends_with(".apex") || file_name.ends_with(".capex") {
                apex_paths.push(path);
            }
        }

        apex_paths.sort();

        for apex in apex_paths {
            apex_infos.push(parse_apex(&apex)?);
        }
    }

    Ok(apex_infos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    #[test]
    fn test_parse_apex() {
        let manifest = ApexManifest {
            name: "com.android.test_apex".to_string(),
            version: 42,
            ..Default::default()
        };
        let mut manifest_bytes = Vec::new();
        manifest.encode(&mut manifest_bytes).unwrap();

        let temp_file = NamedTempFile::new().unwrap();
        let mut zip_writer = ZipWriter::new(File::create(temp_file.path()).unwrap());

        let options = SimpleFileOptions::default();
        zip_writer.start_file("apex_manifest.pb", options).unwrap();
        zip_writer.write_all(&manifest_bytes).unwrap();

        zip_writer.start_file("original_apex", options).unwrap();
        zip_writer
            .write_all(b"fake decompressed apex data")
            .unwrap();

        zip_writer.finish().unwrap();

        let apex_info = parse_apex(temp_file.path()).unwrap();
        assert_eq!(apex_info.package_name, Some("com.android.test_apex".into()));
        assert_eq!(apex_info.version, Some(42));
        assert_eq!(apex_info.is_compressed, Some(true));
        assert_eq!(apex_info.decompressed_size, Some(27));
    }
}
