#!/usr/bin/env bash

set -euo pipefail

usage() {
  cat <<'EOF'
Usage: npm run dev:local-webkit -- /absolute/path/to/WebKitBuild/Release

Builds the Pasted debug application, starts its Vite frontend, and launches the
application directly with a local WebKit build selected for both the app and its
XPC services. The script exits unless vmmap confirms that Pasted loaded that
WebKit.framework instead of the system framework.

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
vite_pid=""
app_pid=""

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
fi

if [[ ! -x "$app_binary" ]]; then
  echo "Pasted's debug application was not found at $app_binary." >&2
  exit 1
fi

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
  "$app_binary" &
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
