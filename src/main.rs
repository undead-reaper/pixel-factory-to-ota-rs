// SPDX-FileCopyrightText: 2026 Andrew Gunnerson
// SPDX-License-Identifier: GPL-3.0-only

use clap::Parser;
use pixel_factory_to_ota::{Cli, generate_ota};
use std::process;
use tempfile::Builder;

fn run() -> Result<(), pixel_factory_to_ota::Error> {
    let args = Cli::parse();

    if let Some(ref work_dir) = args.work_dir {
        generate_ota(&args.input, &args.output, work_dir)?;
    } else {
        let temp_dir = Builder::new().prefix("pixel-factory-to-ota-").tempdir()?;
        generate_ota(&args.input, &args.output, temp_dir.path())?;
    }

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        process::exit(1);
    }
}
