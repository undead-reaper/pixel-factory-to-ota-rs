// SPDX-FileCopyrightText: 2026 Andrew Gunnerson
// SPDX-License-Identifier: GPL-3.0-only

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::Error;

/// Converts a path to an absolute path if it is relative.
pub fn to_absolute(path: &Path) -> std::io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let current = env::current_dir()?;
        Ok(current.join(path))
    }
}

/// Executes an external command directly without a shell,
/// printing the shlex-quoted command line and working directory to stderr.
pub fn cmd<I, S>(args: I, cwd: Option<&Path>) -> Result<(), Error>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let raw_args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    if raw_args.is_empty() {
        return Err(Error::Other("Cannot execute empty command".into()));
    }

    let resolved_cwd = match cwd {
        Some(dir) => to_absolute(dir)?,
        None => env::current_dir()?,
    };

    let program = &raw_args[0];
    let cmd_args = &raw_args[1..];

    let quoted_cmd: String = raw_args
        .iter()
        .map(|arg| {
            let s = arg.to_string_lossy();
            shlex::try_quote(&s)
                .map(|q| q.into_owned())
                .unwrap_or_else(|_| s.into_owned())
        })
        .collect::<Vec<_>>()
        .join(" ");

    eprintln!("Running: {} [cwd: {}]", quoted_cmd, resolved_cwd.display());

    let status = Command::new(program)
        .args(cmd_args)
        .current_dir(&resolved_cwd)
        .status()?;

    if !status.success() {
        return Err(Error::CommandFailed {
            command: quoted_cmd,
            cwd: resolved_cwd,
            status,
        });
    }

    Ok(())
}
