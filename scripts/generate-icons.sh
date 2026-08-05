#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_root"

command -v magick >/dev/null || {
  echo "ImageMagick is required to generate Pasted icons." >&2
  exit 1
}

npm run tauri -- icon src-tauri/app-icon.png

magick -size 128x128 xc:none \
  -fill none -stroke '#000000' -strokewidth 9 \
  -draw "roundrectangle 27,25 101,111 13,13 path 'M46,28 L46,22 C46,16 50,12 56,12 L72,12 C78,12 82,16 82,22 L82,28' line 45,56 83,56 line 45,76 74,76" \
  -define png:color-type=6 src-tauri/icons/tray-icon-128.png
magick src-tauri/icons/tray-icon-128.png -resize 64x64 -define png:color-type=6 src-tauri/icons/tray-icon@2x.png
magick src-tauri/icons/tray-icon-128.png -resize 32x32 -define png:color-type=6 src-tauri/icons/tray-icon.png

tray_alpha_max="$(magick src-tauri/icons/tray-icon@2x.png -alpha extract -format '%[fx:maxima]' info:)"
if [[ "$tray_alpha_max" == "0" ]]; then
  echo "Generated tray icon has no visible artwork." >&2
  exit 1
fi
magick src-tauri/app-icon.png -resize 512x512 public/app_icon.png

echo "Generated desktop, mobile, tray, and browser icon assets."
