// SPDX-FileCopyrightText: 2026 Andrew Gunnerson
// SPDX-License-Identifier: GPL-3.0-only

use std::path::PathBuf;
use std::process::ExitStatus;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("Protobuf decode error: {0}")]
    ProtobufDecode(#[from] prost::DecodeError),

    #[error("Protobuf encode error: {0}")]
    ProtobufEncode(#[from] prost::EncodeError),

    #[error("\"image-*.zip\" not found in factory zip")]
    ImageZipNotFound,

    #[error("'{0}' is not uncompressed")]
    ImageZipNotUncompressed(String),

    #[error("Unsafe name: {0}")]
    UnsafePartitionName(String),

    #[error("Missing dynamic partition group: {0}")]
    MissingDynamicPartitionGroup(String),

    #[error("Invalid build.prop line: '{0}'")]
    InvalidBuildPropLine(String),

    #[error("Missing build property key '{key}' in partition '{partition}'")]
    MissingBuildProp { partition: String, key: String },

    #[error("Invalid integer in build property '{key}' in partition '{partition}': {value}")]
    InvalidBuildPropInt {
        partition: String,
        key: String,
        value: String,
    },

    #[error("Command '{command}' failed with exit status {status} in working directory '{cwd}'")]
    CommandFailed {
        command: String,
        cwd: PathBuf,
        status: ExitStatus,
    },

    #[error("{0}")]
    Other(String),
}
