# Pasted icon assets

- `app-icon.png` is the canonical transparent 1024px fallback icon.
- `app-icon-source.png` retains the generated master before chroma removal.
- `icons/tray-icon.svg` is the canonical monochrome tray mark.
- `icons/tray-icon-copycat.svg` documents the optional monochrome Copycat tray mark mirrored by the icon generator.
- `Pasted.icon` contains clean layered source artwork for Apple's Icon Composer.

Run `npm run icons:generate` after changing either canonical fallback asset. This
regenerates Tauri's desktop/mobile files, the tray PNGs, and the browser icon.

Tauri 2.11 can compile `Pasted.icon` into a native Liquid Glass `Assets.car` and
ship it alongside `icon.icns`. It is intentionally not listed in
`tauri.conf.json` yet because the current Xcode 27 beta asset compiler fails on
Icon Composer packages—including known-good third-party packages—with a bad
file descriptor. Keep the `.icns` fallback enabled when adopting it.
