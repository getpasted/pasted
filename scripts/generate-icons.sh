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

magick -size 128x128 xc:none \
  -fill none -stroke '#000000' -strokewidth 10 \
  -draw "path 'M20,52 L17,11 L48,29 C58,27.8 70,28 80,29.4 L111,12 L108,52 C114,64 116,78 113,88 C109,105 89,117 64,117 C39,117 19,105 15,88 C12,78 14,64 20,52 Z'" \
  -strokewidth 7 \
  -draw "path 'M27,76 L6,66 M28,88 L9,94 M101,76 L122,66 M100,88 L119,94 M39,65 C42.2,60.4 48.8,60.4 52,65 M76,65.4 C79.2,60.8 85.8,60.8 89,65.4 M64,79.2 C64,87 58.4,88.4 53,83 M64,79.2 C64,87 69.6,88.4 75,83'" \
  -strokewidth 6 \
  -draw "path 'M61.4,77 L64,79.2 L66.6,77'" \
  -define png:color-type=6 src-tauri/icons/tray-icon-copycat-128.png
magick src-tauri/icons/tray-icon-copycat-128.png -resize 64x64 -define png:color-type=6 src-tauri/icons/tray-icon-copycat@2x.png
magick src-tauri/icons/tray-icon-copycat-128.png -resize 32x32 -define png:color-type=6 src-tauri/icons/tray-icon-copycat.png

tray_alpha_max="$(magick src-tauri/icons/tray-icon@2x.png -alpha extract -format '%[fx:maxima]' info:)"
if [[ "$tray_alpha_max" == "0" ]]; then
  echo "Generated tray icon has no visible artwork." >&2
  exit 1
fi
copycat_tray_alpha_max="$(magick src-tauri/icons/tray-icon-copycat@2x.png -alpha extract -format '%[fx:maxima]' info:)"
if [[ "$copycat_tray_alpha_max" == "0" ]]; then
  echo "Generated Copycat tray icon has no visible artwork." >&2
  exit 1
fi
magick src-tauri/app-icon.png -resize 512x512 public/app_icon.png

echo "Generated desktop, mobile, tray, and browser icon assets."
