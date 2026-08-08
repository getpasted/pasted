#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
IMAGE_NAME="pasted-linux-builder:debian12"
OUTPUT_DIR="$PROJECT_DIR/release-artifacts/linux"
HOST_UID="$(id -u)"
HOST_GID="$(id -g)"
HOST_ARCH="$(uname -m)"
CARGO_MANIFEST="$PROJECT_DIR/src-tauri/Cargo.toml"
MANIFEST_BACKUP="$(mktemp)"

cp "$CARGO_MANIFEST" "$MANIFEST_BACKUP"
restore_manifest() {
  cp "$MANIFEST_BACKUP" "$CARGO_MANIFEST"
  rm -f "$MANIFEST_BACKUP"
}
trap restore_manifest EXIT

command -v docker >/dev/null 2>&1 || {
  echo "Docker is required to build the portable Linux artifact." >&2
  exit 1
}

mkdir -p "$OUTPUT_DIR"
find "$OUTPUT_DIR" -mindepth 1 -maxdepth 1 -type f -delete

docker build \
  --platform linux/amd64 \
  --tag "$IMAGE_NAME" \
  "$PROJECT_DIR/packaging/linux"

docker run --rm \
  --platform linux/amd64 \
  --volume "$PROJECT_DIR:/workspace" \
  --volume pasted-linux-node-modules:/workspace/node_modules \
  --volume pasted-linux-cargo-target:/cargo-target \
  --env CARGO_TARGET_DIR=/cargo-target \
  --env CI=true \
  --env HOST_UID="$HOST_UID" \
  --env HOST_GID="$HOST_GID" \
  --env HOST_ARCH="$HOST_ARCH" \
  "$IMAGE_NAME" \
  bash -lc '
    set -euo pipefail
    npm ci
    npm run test:platform
    npm run build
    mkdir -p /workspace/release-artifacts/linux
    if [ "$HOST_ARCH" = "x86_64" ]; then
      npm run tauri build -- --bundles appimage
      cp /cargo-target/release/bundle/appimage/*.AppImage /workspace/release-artifacts/linux/
    else
      # Docker Desktop can compile amd64 on Apple Silicon, but linuxdeploy
      # launches nested amd64 AppImage helpers that its emulation cannot run.
      # Preserve the valid compatibility binaries; native CI builds AppImage.
      npm run tauri build -- --no-bundle
      cp /cargo-target/release/pasted-app /workspace/release-artifacts/linux/pasted-app-linux-x86_64
    fi
    cp /cargo-target/release/pasted /workspace/release-artifacts/linux/pasted-linux-x86_64
    cd /workspace/release-artifacts/linux
    find . -maxdepth 1 -type f ! -name SHA256SUMS -print0 \
      | sort -z \
      | xargs -0 sha256sum > SHA256SUMS
    chown -R "$HOST_UID:$HOST_GID" /workspace/release-artifacts/linux
  '

# The Tauri CLI normalizes platform-specific Cargo features while building.
# Restore the contributor manifest before running host-side checks or returning.
restore_manifest
trap - EXIT

echo
echo "SteamOS test artifacts:"
find "$OUTPUT_DIR" -maxdepth 1 -type f -print

if [ "$HOST_ARCH" != "x86_64" ]; then
  echo
  echo "Apple Silicon produced dependency-compatible x86_64 probe binaries."
  echo "Use the Linux AppImage GitHub workflow for the self-contained artifact."
fi
