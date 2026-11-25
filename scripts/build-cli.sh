#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build --release -p overstory-cli
mkdir -p src-tauri/binaries
cp src-tauri/target/release/overstory-cli src-tauri/binaries/overstory-cli
cp scripts/cli-manager.sh src-tauri/binaries/cli-manager.sh
chmod 755 src-tauri/binaries/overstory-cli src-tauri/binaries/cli-manager.sh
codesign --force --sign - src-tauri/binaries/overstory-cli
codesign --verify --strict src-tauri/binaries/overstory-cli
