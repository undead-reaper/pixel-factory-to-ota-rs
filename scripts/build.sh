#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Andrew Gunnerson
# SPDX-License-Identifier: GPL-3.0-only

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${ROOT_DIR}"

MODE="${1:-native}"

echo "==> Building standalone pixel-factory-to-ota (mode: ${MODE})"

case "${MODE}" in
  native)
    echo "==> Compiling host-optimized release binary (target-cpu=native)..."
    RUSTFLAGS="-C target-cpu=native" cargo build --release
    BIN="target/release/pixel-factory-to-ota"
    ;;
  musl)
    echo "==> Compiling fully static musl binary (x86_64-unknown-linux-musl)..."
    rustup target add x86_64-unknown-linux-musl 2>/dev/null || true
    cargo build --release --target x86_64-unknown-linux-musl
    BIN="target/x86_64-unknown-linux-musl/release/pixel-factory-to-ota"
    ;;
  portable)
    echo "==> Compiling portable release binary..."
    cargo build --release
    BIN="target/release/pixel-factory-to-ota"
    ;;
  *)
    echo "Usage: $0 [native|musl|portable]"
    exit 1
    ;;
esac

echo ""
echo "==> Build successful!"
echo "Binary path: ${ROOT_DIR}/${BIN}"
echo "Binary size: $(du -h "${ROOT_DIR}/${BIN}" | cut -f1)"
echo "File info:   $(file "${ROOT_DIR}/${BIN}")"
