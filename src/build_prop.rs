// SPDX-FileCopyrightText: 2026 Andrew Gunnerson
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::error::Error;
use crate::models::PayloadDynamicPartitionGroup;

/// Parses a build.prop file line by line.
/// Trims whitespace, ignores empty lines and comments starting with '#',
/// splits on the first '=', and returns key-value pairs.
pub fn parse_build_prop(path: &Path) -> Result<HashMap<String, String>, Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut result = HashMap::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let (key, value) = trimmed
            .split_once('=')
            .ok_or_else(|| Error::InvalidBuildPropLine(trimmed.to_string()))?;

        result.insert(key.to_string(), value.to_string());
    }

    Ok(result)
}

/// Discovers and parses build.prop files for all dynamic partitions.
/// System uses fs_tree/system/build.prop,
/// Vendor uses fs_tree/build.prop,
/// and all other partitions use fs_tree/etc/build.prop.
pub fn parse_build_props(
    filesystems: &Path,
    dpg: &PayloadDynamicPartitionGroup,
) -> Result<HashMap<String, HashMap<String, String>>, Error> {
    let mut props = HashMap::new();

    for name in &dpg.partition_names {
        let fs_tree = filesystems.join(name).join("fs_tree");

        let build_prop_path = if name == "system" {
            fs_tree.join("system").join("build.prop")
        } else if name == "vendor" {
            fs_tree.join("build.prop")
        } else {
            fs_tree.join("etc").join("build.prop")
        };

        let partition_props = parse_build_prop(&build_prop_path)?;
        props.insert(name.clone(), partition_props);
    }

    Ok(props)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_build_prop_valid() {
        let content = r#"
# Sample build.prop
ro.build.version.release=15
ro.build.date.utc=1700000000

# Comment with spaces
  ro.product.device=akita
ro.empty.val=
ro.with.equal=foo=bar
"#;
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", content).unwrap();

        let props = parse_build_prop(f.path()).unwrap();
        assert_eq!(props.get("ro.build.version.release").unwrap(), "15");
        assert_eq!(props.get("ro.build.date.utc").unwrap(), "1700000000");
        assert_eq!(props.get("ro.product.device").unwrap(), "akita");
        assert_eq!(props.get("ro.empty.val").unwrap(), "");
        assert_eq!(props.get("ro.with.equal").unwrap(), "foo=bar");
    }

    #[test]
    fn test_parse_build_prop_invalid() {
        let content = "this_line_has_no_equal_sign\n";
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", content).unwrap();

        assert!(parse_build_prop(f.path()).is_err());
    }
}
