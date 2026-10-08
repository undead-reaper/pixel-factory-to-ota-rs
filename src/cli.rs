// SPDX-FileCopyrightText: 2026 Andrew Gunnerson
// SPDX-License-Identifier: GPL-3.0-only

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "pixel-factory-to-ota",
    about = "Convert a Pixel factory image to a full OTA"
)]
pub struct Cli {
    /// Input factory image
    #[arg(short = 'i', long = "input", required = true)]
    pub input: PathBuf,

    /// Output OTA
    #[arg(short = 'o', long = "output", required = true)]
    pub output: PathBuf,

    /// Working directory (for debugging)
    #[arg(short = 'w', long = "work-dir")]
    pub work_dir: Option<PathBuf>,
}
