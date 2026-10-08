// SPDX-FileCopyrightText: 2026 Andrew Gunnerson
// SPDX-License-Identifier: GPL-3.0-only

use std::process::Command;

#[test]
fn test_cli_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_pixel-factory-to-ota"))
        .arg("--help")
        .output()
        .expect("Failed to execute binary");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("-i, --input <INPUT>"));
    assert!(stdout.contains("-o, --output <OUTPUT>"));
    assert!(stdout.contains("-w, --work-dir <WORK_DIR>"));
}

#[test]
fn test_cli_missing_required_args() {
    let output = Command::new(env!("CARGO_BIN_EXE_pixel-factory-to-ota"))
        .output()
        .expect("Failed to execute binary");

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_cli_nonexistent_input() {
    let output = Command::new(env!("CARGO_BIN_EXE_pixel-factory-to-ota"))
        .args(["-i", "nonexistent_factory.zip", "-o", "out_ota.zip"])
        .output()
        .expect("Failed to execute binary");

    assert!(!output.status.success());
    assert_ne!(output.status.code(), Some(0));
}
