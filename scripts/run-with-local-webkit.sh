#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: npm run dev:local-webkit -- /absolute/path/to/WebKitBuild/Release

Builds the Pasted debug application, starts its Vite frontend, and launches the
application directly with a local WebKit build selected for both the app and its
XPC services. The script exits unless vmmap confirms that Pasted loaded that
WebKit.framework instead of the system framework. It uses a temporary, seeded
database and pauses clipboard capture so private Pasted history cannot appear.

Set PASTED_LOCAL_WEBKIT_SKIP_BUILD=1 to reuse an existing debug application.
EOF
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  usage
  exit 0
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "A local WebKit preview can only run on macOS." >&2
  exit 1
fi

if [[ $# -ne 1 ]]; then
  usage >&2
  exit 1
fi

webkit_build="$1"
if [[ "$webkit_build" != /* ]]; then
  echo "Pass an absolute WebKit build directory." >&2
  exit 1
fi

webkit_binary="$webkit_build/WebKit.framework/Versions/A/WebKit"
if [[ ! -f "$webkit_binary" ]]; then
  echo "WebKit.framework was not found at $webkit_build." >&2
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app_binary="$repo_root/src-tauri/target/debug/pasted-app"
cli_binary="$repo_root/src-tauri/target/debug/pasted"
preview_parent="${TMPDIR:-/tmp}"
preview_parent="${preview_parent%/}"
vite_pid=""
app_pid=""
preview_root=""

cleanup() {
  local status=$?
  trap - EXIT INT TERM HUP
  if [[ -n "$app_pid" ]] && kill -0 "$app_pid" 2>/dev/null; then
    kill "$app_pid" 2>/dev/null || true
    wait "$app_pid" 2>/dev/null || true
  fi
  if [[ -n "$vite_pid" ]] && kill -0 "$vite_pid" 2>/dev/null; then
    kill "$vite_pid" 2>/dev/null || true
    wait "$vite_pid" 2>/dev/null || true
  fi
  if [[ -n "$preview_root" && "$preview_root" == "$preview_parent/pasted-local-webkit."* ]]; then
    rm -rf -- "$preview_root"
  fi
  exit "$status"
}
trap cleanup EXIT INT TERM HUP

cd "$repo_root"

if [[ "${PASTED_LOCAL_WEBKIT_SKIP_BUILD:-0}" != "1" ]]; then
  if [[ ! -d node_modules ]]; then
    echo "Install the locked frontend dependencies with npm ci first." >&2
    exit 1
  fi
  cargo build --locked --manifest-path src-tauri/Cargo.toml --bin pasted-app
  cargo build --locked --manifest-path src-tauri/Cargo.toml --no-default-features --features cli --bin pasted
fi

if [[ ! -x "$app_binary" || ! -x "$cli_binary" ]]; then
  echo "Pasted's debug application and CLI were not found in src-tauri/target/debug." >&2
  exit 1
fi

preview_root="$(mktemp -d "$preview_parent/pasted-local-webkit.XXXXXX")"
preview_database="$preview_root/pasted.db"
preview_seed="$preview_root/demo-clips.csv"
printf '%s\n' \
  'id,content_type,source,is_pinned,created_at,name,text_content' \
  '1,"text","Pasted Demo",true,"2026-09-01T18:00:00Z","Release engineering","Ship it, but make the pixels behave first."' \
  '2,"text","Pasted Demo",false,"2026-09-01T18:01:00Z","WebKit field notes","Yellow and cyan have agreed to remain perfectly still."' \
  '3,"text","Pasted Demo",false,"2026-09-01T18:02:00Z","Important research","Can a window corner outrun a layout viewport? Not anymore."' \
  '4,"text","Pasted Demo",false,"2026-09-01T18:03:00Z","Extremely official memo","This clipboard contains zero secrets and several excellent rectangles."' \
  > "$preview_seed"
PASTED_DATABASE_PATH="$preview_database" \
  "$cli_binary" clip import "$preview_seed" --format csv >/dev/null

npm run dev -- --host 127.0.0.1 &
vite_pid=$!

vite_ready=false
for _ in {1..100}; do
  if curl --silent --fail --output /dev/null http://127.0.0.1:1420/; then
    vite_ready=true
    break
  fi
  if ! kill -0 "$vite_pid" 2>/dev/null; then
    wait "$vite_pid"
  fi
  sleep 0.1
done

if [[ "$vite_ready" != "true" ]]; then
  echo "Vite did not become ready at http://127.0.0.1:1420/." >&2
  exit 1
fi

env \
  DYLD_FRAMEWORK_PATH="$webkit_build" \
  __XPC_DYLD_FRAMEWORK_PATH="$webkit_build" \
  DYLD_LIBRARY_PATH="$webkit_build" \
  __XPC_DYLD_LIBRARY_PATH="$webkit_build" \
  PASTED_PREVIEW_DATABASE_PATH="$preview_database" \
  "$app_binary" --skip-welcome &
app_pid=$!

verified_path=""
alternate_build="${webkit_build#/private}"
for _ in {1..100}; do
  if ! kill -0 "$app_pid" 2>/dev/null; then
    wait "$app_pid"
  fi
  loaded_webkit="$(vmmap "$app_pid" 2>/dev/null | sed -n '/\/WebKit\.framework\/.*\/WebKit$/p' || true)"
  if [[ "$loaded_webkit" == *"$webkit_build/WebKit.framework/"* \
    || "$loaded_webkit" == *"$alternate_build/WebKit.framework/"* ]]; then
    verified_path="$loaded_webkit"
    break
  fi
  sleep 0.1
done

if [[ -z "$verified_path" ]]; then
  echo "Pasted launched, but vmmap did not confirm the requested WebKit.framework." >&2
  exit 1
fi

echo "Verified local WebKit.framework:"
echo "$verified_path"
echo "Quit Pasted to stop the preview and its Vite server."

wait "$app_pid"
app_pid=""
