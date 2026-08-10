#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

dmg_path="${1:-}"
mode="${2:-}"
if [[ -z "$dmg_path" ]]; then
  dmg_path="$(find src-tauri/target/release/bundle/dmg -maxdepth 1 -name 'Pasted_*.dmg' -type f -print | sort | tail -n 1)"
fi

if [[ -z "$dmg_path" || ! -f "$dmg_path" ]]; then
  echo "Usage: $0 [path-to-dmg] [--local]" >&2
  exit 1
fi

hdiutil verify "$dmg_path"

mount_dir="$(mktemp -d /private/tmp/pasted-release.XXXXXX)"
cleanup() {
  if mount | grep -Fq "on $mount_dir "; then
    diskutil eject "$mount_dir" >/dev/null
  fi
  rmdir "$mount_dir" 2>/dev/null || true
}
trap cleanup EXIT

diskutil image attach --readOnly --mountOptions nobrowse --mountPoint "$mount_dir" "$dmg_path" >/dev/null
app_path="$mount_dir/Pasted.app"

for presentation_asset in \
  "$mount_dir/.DS_Store" \
  "$mount_dir/.background/background.png" \
  "$mount_dir/.VolumeIcon.icns"; do
  if [[ ! -f "$presentation_asset" ]]; then
    echo "The DMG is missing Finder presentation metadata: $presentation_asset" >&2
    exit 1
  fi
done

if [[ ! -L "$mount_dir/Applications" || "$(readlink "$mount_dir/Applications")" != "/Applications" ]]; then
  echo "The DMG is missing its Applications folder link." >&2
  exit 1
fi

echo "Branded DMG background, Finder layout, volume icon, and Applications link verified."

codesign --verify --deep --strict --verbose=2 "$app_path"

if [[ "$mode" == "--local" ]]; then
  signature_details="$(codesign -dvv "$app_path" 2>&1)"
  if [[ "$signature_details" != *"Signature=adhoc"* ]]; then
    echo "Expected the local package to use an ad-hoc signature." >&2
    exit 1
  fi
  echo "Local DMG verified. It is intentionally not trusted by Gatekeeper."
else
  codesign_details="$(codesign -dvv "$app_path" 2>&1)"
  if [[ "$codesign_details" != *"Authority=Developer ID Application:"* ]]; then
    echo "The packaged app is not signed with a Developer ID Application certificate." >&2
    exit 1
  fi
  spctl --assess --type execute --verbose=2 "$app_path"
  xcrun stapler validate "$dmg_path"
  echo "Developer ID signature, Gatekeeper assessment, and the distributable DMG ticket verified."
fi

shasum -a 256 "$dmg_path"
