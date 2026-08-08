#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
source_svg="$repo_root/src-tauri/dmg/background.svg"
output_png="$repo_root/src-tauri/dmg/background.png"
black_font="/Library/Fonts/SF-Pro-Display-Black.otf"
medium_font="/Library/Fonts/SF-Pro-Display-Medium.otf"
temporary_dir="$(mktemp -d)"

cleanup() {
  rm -rf "$temporary_dir"
}
trap cleanup EXIT

if ! command -v magick >/dev/null 2>&1; then
  echo "ImageMagick is required to render the DMG background." >&2
  exit 1
fi

if [[ ! -f "$black_font" || ! -f "$medium_font" ]]; then
  echo "SF Pro Display must be installed to reproduce the website wordmark." >&2
  exit 1
fi

magick -background none "$source_svg" "$temporary_dir/base.png"

# Draw the exact getpasted.app mark paths through ImageMagick's native path
# renderer; its SVG delegate drops stroke-only artwork on some macOS versions.
magick \
  -size 36x36 \
  xc:none \
  -stroke '#f5f5f2' \
  -strokewidth 2.5 \
  -fill none \
  -draw "path 'M11.25 7.03125 H24.75 C26.775 7.03125 28.40625 8.6625 28.40625 10.6875 V27.5625 C28.40625 29.5875 26.775 31.21875 24.75 31.21875 H11.25 C9.225 31.21875 7.59375 29.5875 7.59375 27.5625 V10.6875 C7.59375 8.6625 9.225 7.03125 11.25 7.03125 Z' path 'M12.9375 7.875 V6.1875 C12.9375 4.6125 14.175 3.375 15.75 3.375 H20.25 C21.825 3.375 23.0625 4.6125 23.0625 6.1875 V7.875' path 'M12.65625 15.75 H23.34375 M12.65625 21.375 H20.8125'" \
  "$temporary_dir/mark.png"

magick \
  "$temporary_dir/base.png" \
  "$temporary_dir/mark.png" \
  -geometry +258+18 \
  -composite \
  -gravity northwest \
  -font "$black_font" \
  -pointsize 32 \
  -kerning -1.8 \
  -fill '#f5f5f2' \
  -annotate +306+11 'Pasted' \
  -font "$medium_font" \
  -pointsize 13 \
  -kerning 0 \
  -fill '#a8adaf' \
  -annotate +307+49 'Copy responsibly.' \
  "$output_png"

echo "Rendered $output_png"
