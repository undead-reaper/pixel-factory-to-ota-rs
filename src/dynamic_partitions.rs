// SPDX-FileCopyrightText: 2026 Andrew Gunnerson
// SPDX-License-Identifier: GPL-3.0-only

use std::fs;
use std::path::Path;

use crate::error::Error;
use crate::models::{LpMetadata, PayloadDynamicPartitionGroup};

/// Validates that a partition name is safe to use in filesystem paths.
/// Checks that the name has no directory components, path separators, or traversal elements.
pub fn validate_partition_name(name: &str) -> Result<(), Error> {
    let path = Path::new(name);
    let is_safe = path.file_name().and_then(|f| f.to_str()) == Some(name)
        && !name.contains('/')
        && !name.contains('\\')
        && name != "."
        && name != "..";

    if !is_safe {
        return Err(Error::UnsafePartitionName(name.to_string()));
    }

    Ok(())
}

/// Parses the logical partition metadata from super_empty.img's unpacked lp.toml.
/// Identifies the dynamic partition group (the first group where name != "default"),
/// strips "_a" suffixes, validates partition names, and returns the group metadata.
pub fn parse_dynamic_partitions(super_toml: &Path) -> Result<PayloadDynamicPartitionGroup, Error> {
    let content = fs::read_to_string(super_toml)?;
    let super_info: LpMetadata = toml::from_str(&content)?;

    let first_slot = super_info
        .slots
        .first()
        .ok_or_else(|| Error::MissingDynamicPartitionGroup("No slots found in lp.toml".into()))?;

    for group in &first_slot.groups {
        if group.name != "default" {
            let mut partitions = Vec::new();

            for partition in &group.partitions {
                validate_partition_name(&partition.name)?;

                let stripped = partition
                    .name
                    .strip_suffix("_a")
                    .unwrap_or(&partition.name)
                    .to_string();
                partitions.push(stripped);
            }

            let group_name = group
                .name
                .strip_suffix("_a")
                .unwrap_or(&group.name)
                .to_string();

            return Ok(PayloadDynamicPartitionGroup {
                name: group_name,
                size: group.maximum_size,
                partition_names: partitions,
            });
        }
    }

    Err(Error::MissingDynamicPartitionGroup(format!(
        "{super_info:?}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_validate_partition_name() {
        assert!(validate_partition_name("system").is_ok());
        assert!(validate_partition_name("system_a").is_ok());
        assert!(validate_partition_name("vendor_dlkm").is_ok());

        assert!(validate_partition_name("../system").is_err());
        assert!(validate_partition_name("/system").is_err());
        assert!(validate_partition_name("foo/bar").is_err());
        assert!(validate_partition_name("foo\\bar").is_err());
        assert!(validate_partition_name(".").is_err());
        assert!(validate_partition_name("..").is_err());
    }

    #[test]
    fn test_parse_dynamic_partitions_success() {
        let toml_data = r#"
image_type = "Empty"
metadata_max_size = 65536
metadata_slot_count = 3
logical_block_size = 4096

[[slots]]
major_version = 10
minor_version = 0
flags = "none"

[[slots.block_devices]]
first_logical_sector = 2048
alignment = 1048576
alignment_offset = 0
size = 8000000000
partition_name = "super"
flags = "none"

[[slots.groups]]
name = "default"
flags = "none"
partitions = []

[[slots.groups]]
name = "google_dynamic_partitions_a"
flags = "none"
maximum_size = 7500000000

[[slots.groups.partitions]]
name = "system_a"
attributes = "readonly"

[[slots.groups.partitions]]
name = "vendor_a"
attributes = "readonly"

[[slots.groups.partitions]]
name = "product_a"
attributes = "readonly"
"#;

        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", toml_data).unwrap();

        let dpg = parse_dynamic_partitions(file.path()).unwrap();
        assert_eq!(dpg.name, "google_dynamic_partitions");
        assert_eq!(dpg.size, Some(7500000000));
        assert_eq!(dpg.partition_names, vec!["system", "vendor", "product"]);
    }

    #[test]
    fn test_parse_dynamic_partitions_unsafe() {
        let toml_data = r#"
image_type = "Empty"
metadata_max_size = 65536
metadata_slot_count = 3
logical_block_size = 4096

[[slots]]
major_version = 10
minor_version = 0
flags = "none"
block_devices = []

[[slots.groups]]
name = "google_dynamic_partitions"
flags = "none"

[[slots.groups.partitions]]
name = "../system"
attributes = "readonly"
"#;

        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", toml_data).unwrap();

        assert!(parse_dynamic_partitions(file.path()).is_err());
    }
}
