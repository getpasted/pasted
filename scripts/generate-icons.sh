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
  -fill none -stroke '#000000' -strokewidth 28 \
  -draw "stroke-linecap round stroke-linejoin round affine 0.250980392,0,0,0.250980392,-107.149302,-36.462933 path 'M762.025 620.149C792.044 608.035 843.614 576.749 851.237 451.875C894.917 385.847 905.932 326.914 905.525 294.084C905.384 269.438 829.823 286.574 786.666 321.664C771.878 304.128 721.228 275.201 676.949 275.591C668.951 234.12 653.991 172.808 628.925 167.302C596.322 165.665 547.394 263.912 544.258 306.23C487.41 359.697 383.39 506.226 577.086 579.023'" \
  -draw "stroke-linecap round stroke-linejoin round affine 0.250980392,0,0,0.250980392,-107.149302,-36.462933 path 'M520.286 410.156C535.146 421.203 483.689 380.877 448.343 368.058M509.991 433.363C495.373 429.18 456.115 418.987 437.84 418.137M784.061 512.338C800.928 514.918 858.348 539.313 874.407 550.167M773.018 539.306C778.17 543.192 834.805 582.42 846.539 596.353'" \
  -draw "stroke-linecap round stroke-linejoin round affine 0.250980392,0,0,0.250980392,-107.149302,-36.462933 path 'M560.77 388.272C577.759 371.079 613.842 374.504 611.894 412.646M705.409 452.09C721.064 433.29 756.195 433.246 759.572 472.095M642.718 461.304C633.555 480.661 601.968 504.159 595.795 464.351M673.881 497.937C662.085 505.343 624.699 507.824 642.952 461.683'" \
  -strokewidth 21.35 \
  -draw "stroke-linecap round stroke-linejoin round affine 0.299660549,0.129788988,-0.131703969,0.304082824,-74.684988,-175.429773 path 'M672.335 546.462C674.301 544.126 678.233 544.126 680.2 545.294C682.166 546.462 682.166 548.797 680.2 551.133C678.823 552.885 675.284 554.636 672.335 555.804C669.386 554.636 665.847 552.885 664.47 551.133C662.504 548.797 662.504 546.462 664.47 545.294C666.436 544.126 670.369 544.126 672.335 546.462Z'" \
  -fill 'rgba(0,0,0,0.32)' -stroke none \
  -draw "affine 0.205803921,0,0,0.205803921,-78.935286,-27.552733 path 'M568.875 301.546C568.519 284.235 606.629 194.829 624.529 197.231C637.313 198.946 648.148 275.414 648.148 275.414S610.988 280.491 568.875 301.546Z'" \
  -draw "affine -0.184741331,-0.192994274,-0.174732880,0.167260963,261.351167,122.696708 path 'M574.828 299.41C579.864 275.433 595.27 222.372 619.789 207.431C629.713 207.255 642.219 275.66 642.219 275.66S616.941 278.355 574.828 299.41Z'" \
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
