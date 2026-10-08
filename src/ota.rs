// SPDX-FileCopyrightText: 2026 Andrew Gunnerson
// SPDX-License-Identifier: GPL-3.0-only

use prost::Message;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use crate::error::Error;
use crate::models::{
    OtaDeviceState, OtaInfo, OtaMetadata, OtaPartitionState, PayloadApexInfo,
    PayloadDynamicPartitionGroup,
};
use crate::protobuf::build::tools::releasetools::{ApexInfo, ApexMetadata};

/// Constructs ota.toml metadata matching avbroot's OtaInfo specification.
pub fn create_ota_toml(
    dpg: &PayloadDynamicPartitionGroup,
    build_props: &HashMap<String, HashMap<String, String>>,
    ota_toml: &Path,
) -> Result<(), Error> {
    let vendor_props = build_props
        .get("vendor")
        .ok_or_else(|| Error::MissingBuildProp {
            partition: "vendor".into(),
            key: "vendor partition missing from build_props".into(),
        })?;

    let system_props = build_props
        .get("system")
        .ok_or_else(|| Error::MissingBuildProp {
            partition: "system".into(),
            key: "system partition missing from build_props".into(),
        })?;

    let codename = vendor_props
        .get("ro.product.vendor.device")
        .ok_or_else(|| Error::MissingBuildProp {
            partition: "vendor".into(),
            key: "ro.product.vendor.device".into(),
        })?
        .clone();

    let vendor_fingerprint = vendor_props
        .get("ro.vendor.build.fingerprint")
        .ok_or_else(|| Error::MissingBuildProp {
            partition: "vendor".into(),
            key: "ro.vendor.build.fingerprint".into(),
        })?
        .clone();

    let build_incremental = system_props
        .get("ro.build.version.incremental")
        .ok_or_else(|| Error::MissingBuildProp {
            partition: "system".into(),
            key: "ro.build.version.incremental".into(),
        })?
        .clone();

    let timestamp_str =
        system_props
            .get("ro.build.date.utc")
            .ok_or_else(|| Error::MissingBuildProp {
                partition: "system".into(),
                key: "ro.build.date.utc".into(),
            })?;

    let timestamp = timestamp_str
        .parse::<i64>()
        .map_err(|_| Error::InvalidBuildPropInt {
            partition: "system".into(),
            key: "ro.build.date.utc".into(),
            value: timestamp_str.clone(),
        })?;

    let sdk_level = system_props
        .get("ro.build.version.sdk")
        .ok_or_else(|| Error::MissingBuildProp {
            partition: "system".into(),
            key: "ro.build.version.sdk".into(),
        })?
        .clone();

    let security_patch_level = system_props
        .get("ro.build.version.security_patch")
        .ok_or_else(|| Error::MissingBuildProp {
            partition: "system".into(),
            key: "ro.build.version.security_patch".into(),
        })?
        .clone();

    let mut partition_state = Vec::new();
    let mut sorted_partitions = dpg.partition_names.clone();
    sorted_partitions.sort();

    for name in sorted_partitions {
        let part_props = build_props
            .get(&name)
            .ok_or_else(|| Error::MissingBuildProp {
                partition: name.clone(),
                key: "partition missing from build_props".into(),
            })?;

        let device_key = format!("ro.product.{name}.device");
        let device = part_props
            .get(&device_key)
            .ok_or_else(|| Error::MissingBuildProp {
                partition: name.clone(),
                key: device_key,
            })?
            .clone();

        let fingerprint_key = format!("ro.{name}.build.fingerprint");
        let fingerprint = part_props
            .get(&fingerprint_key)
            .ok_or_else(|| Error::MissingBuildProp {
                partition: name.clone(),
                key: fingerprint_key,
            })?
            .clone();

        let date_key = format!("ro.{name}.build.date.utc");
        let version = part_props
            .get(&date_key)
            .ok_or_else(|| Error::MissingBuildProp {
                partition: name.clone(),
                key: date_key,
            })?
            .clone();

        partition_state.push(OtaPartitionState {
            partition_name: name,
            device: vec![device],
            build: vec![fingerprint],
            version,
        });
    }

    let ota_info = OtaInfo {
        files: vec![
            "apex_info.pb".into(),
            "payload.bin".into(),
            "payload_properties.txt".into(),
        ],
        metadata: OtaMetadata {
            ota_type: "AB".into(),
            wipe: false,
            downgrade: false,
            property_files: BTreeMap::new(),
            precondition: OtaDeviceState {
                device: vec![codename.clone()],
                build: vec![],
                build_incremental: "".into(),
                timestamp: 0,
                sdk_level: "".into(),
                security_patch_level: "".into(),
                partition_state: vec![],
            },
            postcondition: OtaDeviceState {
                device: vec![codename],
                build: vec![vendor_fingerprint],
                build_incremental,
                timestamp,
                sdk_level,
                security_patch_level,
                partition_state,
            },
            required_cache: 0,
            spl_downgrade: false,
        },
    };

    let toml_str = toml::to_string_pretty(&ota_info)?;
    fs::write(ota_toml, toml_str)?;

    Ok(())
}

/// Creates apex_info.pb binary protobuf file from parsed APEX information.
pub fn create_apex_info(apex_infos: &[PayloadApexInfo], apex_info_pb: &Path) -> Result<(), Error> {
    let mut metadata = ApexMetadata::default();

    for payload_apex_info in apex_infos {
        let apex_info = ApexInfo {
            package_name: payload_apex_info.package_name.clone().unwrap_or_default(),
            version: payload_apex_info.version.unwrap_or(0),
            is_compressed: payload_apex_info.is_compressed.unwrap_or(false),
            decompressed_size: payload_apex_info.decompressed_size.unwrap_or(0) as i64,
            source_version: 0,
        };
        metadata.apex_info.push(apex_info);
    }

    let mut buf = Vec::new();
    metadata.encode(&mut buf)?;
    fs::write(apex_info_pb, buf)?;

    Ok(())
}
