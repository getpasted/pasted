#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "The universal macOS CLI must be built on macOS." >&2
  exit 1
fi

manifest_path="src-tauri/Cargo.toml"
output_dir="src-tauri/target/universal-apple-darwin/release"
output_path="$output_dir/pasted"

for target in aarch64-apple-darwin x86_64-apple-darwin; do
  cargo build \
    --manifest-path "$manifest_path" \
    --release \
    --bin pasted \
    --target "$target"
done

mkdir -p "$output_dir"
lipo -create \
  src-tauri/target/aarch64-apple-darwin/release/pasted \
  src-tauri/target/x86_64-apple-darwin/release/pasted \
  -output "$output_path"

lipo "$output_path" -verify_arch arm64 x86_64
echo "Built universal Pasted CLI at $output_path"
