#!/bin/sh
set -eu
palette_root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
palette_cache=${XDG_CACHE_HOME:-$HOME/.cache}/hyper-palette
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$palette_cache/target}
cargo build --locked --manifest-path "$palette_root/Cargo.toml"
palette_bundle="$palette_cache/Hyper Palette Prototype.app"
mkdir -p "$palette_bundle/Contents/MacOS"
cp "$palette_root/macos/Info.plist" "$palette_bundle/Contents/Info.plist"
cp "$palette_root/macos/demo" "$palette_bundle/Contents/MacOS/demo"
cp "$CARGO_TARGET_DIR/debug/hyper-palette" "$palette_bundle/Contents/MacOS/hyper-palette"
chmod +x "$palette_bundle/Contents/MacOS/demo"
printf '%s\n' "$palette_bundle"
