// SPDX-FileCopyrightText: 2026 Andrew Gunnerson
// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ==========================================
// OTA TOML Models
// ==========================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtaPartitionState {
    pub partition_name: String,
    pub device: Vec<String>,
    pub build: Vec<String>,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtaDeviceState {
    pub device: Vec<String>,
    pub build: Vec<String>,
    pub build_incremental: String,
    pub timestamp: i64,
    pub sdk_level: String,
    pub security_patch_level: String,
    #[serde(default)]
    pub partition_state: Vec<OtaPartitionState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtaMetadata {
    #[serde(rename = "type")]
    pub ota_type: String,
    pub wipe: bool,
    pub downgrade: bool,
    #[serde(default)]
    pub property_files: BTreeMap<String, String>,
    pub precondition: OtaDeviceState,
    pub postcondition: OtaDeviceState,
    pub required_cache: u64,
    pub spl_downgrade: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtaInfo {
    pub files: Vec<String>,
    pub metadata: OtaMetadata,
}

// ==========================================
// Payload TOML Models
// ==========================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PayloadPartition {
    pub partition_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_postinstall: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postinstall_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filesystem_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postinstall_optional: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PayloadDynamicPartitionGroup {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(default)]
    pub partition_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PayloadVABCFeatureSet {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threaded: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_writes: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PayloadDynamicPartitionMetadata {
    #[serde(default)]
    pub groups: Vec<PayloadDynamicPartitionGroup>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vabc_enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vabc_compression_param: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cow_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vabc_feature_set: Option<PayloadVABCFeatureSet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compression_factor: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable_ublk: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PayloadApexInfo {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_compressed: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decompressed_size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PayloadManifest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_size: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minor_version: Option<u32>,
    #[serde(default)]
    pub partitions: Vec<PayloadPartition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_timestamp: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamic_partition_metadata: Option<PayloadDynamicPartitionMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partial_update: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub apex_info: Vec<PayloadApexInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_patch_level: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayloadInfo {
    pub version: u32,
    pub manifest: PayloadManifest,
}

// ==========================================
// LP Metadata Models (lp.toml)
// ==========================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LpPartition {
    pub name: String,
    pub attributes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LpPartitionGroup {
    pub name: String,
    pub flags: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum_size: Option<u64>,
    pub partitions: Vec<LpPartition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LpBlockDevice {
    pub first_logical_sector: u64,
    pub alignment: u64,
    pub alignment_offset: u64,
    pub size: u64,
    pub partition_name: String,
    pub flags: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LpMetadataSlot {
    pub major_version: u32,
    pub minor_version: u32,
    pub groups: Vec<LpPartitionGroup>,
    pub block_devices: Vec<LpBlockDevice>,
    pub flags: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LpMetadata {
    pub image_type: String,
    pub metadata_max_size: u64,
    pub metadata_slot_count: u32,
    pub logical_block_size: u64,
    pub slots: Vec<LpMetadataSlot>,
}
