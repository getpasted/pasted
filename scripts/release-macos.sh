#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "macOS releases must be built on macOS." >&2
  exit 1
fi

local_build=false
if [[ "${1:-}" == "--local" ]]; then
  local_build=true
elif [[ $# -gt 0 ]]; then
  echo "Usage: $0 [--local]" >&2
  exit 1
fi

if $local_build; then
  export APPLE_SIGNING_IDENTITY="-"
  echo "Building a local ad-hoc signed DMG. Gatekeeper will reject this artifact on other Macs."
else
  identity="${APPLE_SIGNING_IDENTITY:-}"
  if [[ -z "$identity" ]]; then
    identity="$(security find-identity -v -p codesigning | sed -n 's/.*"\(Developer ID Application:.*\)"/\1/p' | head -n 1)"
  fi

  if [[ -z "$identity" ]]; then
    echo "No Developer ID Application identity is available in the keychain." >&2
    echo "Install the certificate, or set APPLE_SIGNING_IDENTITY explicitly." >&2
    exit 1
  fi

  if [[ "$identity" != Developer\ ID\ Application:* ]]; then
    echo "APPLE_SIGNING_IDENTITY must identify a Developer ID Application certificate." >&2
    exit 1
  fi

  has_api_credentials=false
  if [[ -n "${APPLE_API_ISSUER:-}" && -n "${APPLE_API_KEY:-}" && -n "${APPLE_API_KEY_PATH:-}" ]]; then
    has_api_credentials=true
  fi

  has_apple_id_credentials=false
  if [[ -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
    has_apple_id_credentials=true
  fi

  keychain_profile="${APPLE_KEYCHAIN_PROFILE:-Pasted}"
  has_keychain_profile=false
  if ! $has_api_credentials && ! $has_apple_id_credentials; then
    if xcrun notarytool history --keychain-profile "$keychain_profile" >/dev/null 2>&1; then
      has_keychain_profile=true
    fi
  fi

  if ! $has_api_credentials && ! $has_apple_id_credentials && ! $has_keychain_profile; then
    echo "Notarization credentials are incomplete." >&2
    echo "Provide APPLE_API_ISSUER, APPLE_API_KEY, and APPLE_API_KEY_PATH;" >&2
    echo "APPLE_ID, APPLE_PASSWORD, and APPLE_TEAM_ID;" >&2
    echo "or store an Apple notarization profile named '$keychain_profile' in Keychain." >&2
    exit 1
  fi

  if $has_api_credentials && [[ ! -f "$APPLE_API_KEY_PATH" ]]; then
    echo "APPLE_API_KEY_PATH does not point to a readable App Store Connect private key." >&2
    exit 1
  fi

  export APPLE_SIGNING_IDENTITY="$identity"
  if $has_keychain_profile; then
    echo "Building and signing Pasted with a Developer ID identity."
    echo "Notarization will use Keychain profile '$keychain_profile'."
  else
    echo "Building, signing, notarizing, and stapling Pasted with a Developer ID identity."
  fi
fi

npm run test:all
cargo build \
  --manifest-path src-tauri/Cargo.toml \
  --release \
  --no-default-features \
  --features cli \
  --bin pasted
npm run tauri -- build --bundles dmg

dmg_path="$(find src-tauri/target/release/bundle/dmg -maxdepth 1 -name 'Pasted_*.dmg' -type f -print | sort | tail -n 1)"
if [[ -z "$dmg_path" ]]; then
  echo "Tauri completed without producing a Pasted DMG." >&2
  exit 1
fi

if ! $local_build && $has_keychain_profile; then
  echo "Submitting the signed DMG to Apple's notary service."
  xcrun notarytool submit "$dmg_path" --keychain-profile "$keychain_profile" --wait
  echo "Stapling Apple's notarization ticket to the DMG."
  xcrun stapler staple "$dmg_path"
fi

verify_args=("$dmg_path")
if $local_build; then
  verify_args+=("--local")
fi
bash scripts/verify-macos-release.sh "${verify_args[@]}"
