# Linux and SteamOS testing

Pasted uses native decorated windows on Linux. The first Linux acceptance target
is Valve's x86_64 Steam Machine running SteamOS 3.8 in KDE Desktop Mode under
Wayland. The AppImage is intentionally unsandboxed so clipboard, tray, hotkey,
HUD, and paste-target behavior can be evaluated without Flatpak permissions
changing the result.

## Build

On a development machine with Docker running:

```bash
npm run release:linux
```

The Debian 12 builder provides a conservative GLIBC baseline. An x86_64 host
writes the AppImage plus `SHA256SUMS` to `release-artifacts/linux/`. Apple
Silicon can compile the valid x86_64 GUI and CLI probe binaries, but cannot run
the nested x86_64 `linuxdeploy` helper needed to finish an AppImage. GitHub's
manually dispatchable **Linux AppImage** workflow runs natively and produces
the self-contained artifact plus a standalone CLI.

## Transfer and launch

Prefer the AppImage from the native Linux workflow:

```bash
scp release-artifacts/linux/Pasted_1.0.0_amd64.AppImage USER@STEAM-MACHINE:~/
ssh USER@STEAM-MACHINE
chmod +x ~/Pasted_1.0.0_amd64.AppImage
~/Pasted_1.0.0_amd64.AppImage
```

If SteamOS does not mount the AppImage through FUSE, launch it with:

```bash
~/Pasted_1.0.0_amd64.AppImage --appimage-extract-and-run
```

On Wayland AppImage sessions, Pasted automatically preloads the host's Wayland
client library before WebKitGTK starts its renderer processes. This keeps the
bundled browser runtime on the same Mesa/EGL boundary as SteamOS and prevents a
blank window with `EGL_BAD_PARAMETER`. Native packages and X11 sessions are not
changed.

For a quick dependency-compatibility probe from an Apple Silicon build:

```bash
scp release-artifacts/linux/pasted-linux-x86_64 USER@STEAM-MACHINE:~/
ssh USER@STEAM-MACHINE 'chmod +x ~/pasted-linux-x86_64 && ~/pasted-linux-x86_64'
```

If that reports a missing shared library, use the AppImage rather than changing
SteamOS's immutable base system.

## Acceptance pass

Record `uname -m`, `/etc/os-release`, `$XDG_SESSION_TYPE`, and
`$XDG_CURRENT_DESKTOP`, then verify:

- KDE owns the native titlebar and its upper-right window controls.
- Pasted does not reserve a macOS traffic-light inset in the sidebar.
- Main-window geometry, menus, tray icon, and appearance survive relaunch.
- Text, images, files, search, Bins, Queue, and backup/import work.
- Hotkeys and automatic paste either work or explain the Wayland capability
  limitation without consuming Queue items.
- HUD opens, accepts keyboard navigation, and returns focus correctly.
- `pasted` reports the same collection and clip state as the GUI.

The AppImage is a test artifact until these checks pass on actual SteamOS.
