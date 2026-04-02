#!/bin/bash
# Wrapper for cross-compiling oxigotchi for Pi Zero 2W
set -euo pipefail
source ~/.cargo/env
cd /mnt/c/msys64/home/gelum/oxigotchi/rust
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
TARGET="aarch64-unknown-linux-gnu"
cargo build --release --target="$TARGET" 2>&1
echo "Done. Binary at: target/$TARGET/release/oxigotchi"
file "target/$TARGET/release/oxigotchi"
