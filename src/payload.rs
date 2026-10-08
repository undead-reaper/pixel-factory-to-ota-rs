// SPDX-FileCopyrightText: 2026 Andrew Gunnerson
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::error::Error;
use crate::models::{
    PayloadApexInfo, PayloadDynamicPartitionGroup, PayloadDynamicPartitionMetadata, PayloadInfo,
    PayloadManifest, PayloadPartition,
};

/// Constructs payload.toml metadata matching avbroot's PayloadInfo specification.
pub fn create_payload_toml(
    payload_images: &Path,
    dpg: &PayloadDynamicPartitionGroup,
    build_props: &HashMap<String, HashMap<String, String>>,
    apex_infos: &[PayloadApexInfo],
    payload_toml: &Path,
) -> Result<(), Error> {
    let mut partitions = Vec::new();
    for entry in fs::read_dir(payload_images)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(file_name) = path.file_name().and_then(|f| f.to_str())
            && let Some(stem) = file_name.strip_suffix(".img")
        {
            partitions.push(stem.to_string());
        }
    }
    partitions.sort();

    let system_props = build_props
        .get("system")
        .ok_or_else(|| Error::MissingBuildProp {
            partition: "system".into(),
            key: "system partition missing from build_props".into(),
        })?;

    let security_patch_level = system_props
        .get("ro.build.version.security_patch")
        .ok_or_else(|| Error::MissingBuildProp {
            partition: "system".into(),
            key: "ro.build.version.security_patch".into(),
        })?
        .clone();

    let mut manifest_partitions = Vec::new();
    for name in partitions {
        let mut partition = PayloadPartition {
            partition_name: name.clone(),
            ..Default::default()
        };

        if dpg.partition_names.contains(&name) {
            let key = format!("ro.{name}.build.date.utc");
            if let Some(part_props) = build_props.get(&name)
                && let Some(version) = part_props.get(&key)
            {
                partition.version = Some(version.clone());
            }
        }

        if name == "system" {
            partition.run_postinstall = Some(true);
            partition.postinstall_path = Some("system/bin/otapreopt_script".into());
            partition.filesystem_type = Some("ext4".into());
            partition.postinstall_optional = Some(true);
        } else if name == "vendor" {
            partition.run_postinstall = Some(true);
            partition.postinstall_path = Some("bin/checkpoint_gc".into());
            partition.filesystem_type = Some("ext4".into());
            partition.postinstall_optional = Some(true);
        }

        manifest_partitions.push(partition);
    }

    let mut max_timestamp: Option<i64> = None;
    for p in &manifest_partitions {
        if let Some(ref v) = p.version {
            let ts = v.parse::<i64>().map_err(|_| Error::InvalidBuildPropInt {
                partition: p.partition_name.clone(),
                key: format!("ro.{}.build.date.utc", p.partition_name),
                value: v.clone(),
            })?;
            max_timestamp = Some(max_timestamp.map_or(ts, |m| m.max(ts)));
        }
    }

    let dynamic_partition_metadata = PayloadDynamicPartitionMetadata {
        snapshot_enabled: Some(true),
        vabc_enabled: Some(true),
        vabc_compression_param: Some("lz4".into()),
        cow_version: Some(3),
        compression_factor: Some(65536),
        groups: vec![dpg.clone()],
        ..Default::default()
    };

    let info = PayloadInfo {
        version: 2,
        manifest: PayloadManifest {
            block_size: Some(4096),
            minor_version: Some(0),
            partitions: manifest_partitions,
            max_timestamp,
            dynamic_partition_metadata: Some(dynamic_partition_metadata),
            partial_update: None,
            apex_info: apex_infos.to_vec(),
            security_patch_level: Some(security_patch_level),
        },
    };

    let toml_str = toml::to_string_pretty(&info)?;
    fs::write(payload_toml, toml_str)?;

    Ok(())
}
