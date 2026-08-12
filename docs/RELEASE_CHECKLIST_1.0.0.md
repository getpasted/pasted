# Pasted 1.0.0 release-candidate checklist

Use the exact notarized DMG intended for publication. Do not substitute a development build during acceptance testing.

## Automated release gate

- [ ] Working tree is clean and points at the intended release commit.
- [ ] `npm run release:macos` passes the complete test suite.
- [ ] Apple accepts the notarization submission.
- [ ] The notarization ticket is stapled to the DMG.
- [ ] `scripts/verify-macos-release.sh` passes code-signing, Gatekeeper, ticket, and disk-image checks.
- [ ] The printed SHA-256 checksum is saved with the published release.

## Clean installation

- [ ] Download or transfer the DMG so it receives macOS quarantine metadata.
- [ ] Drag Pasted into `/Applications`, eject the DMG, and launch it normally.
- [ ] Gatekeeper opens Pasted without an unidentified-developer warning.
- [ ] The Dock icon, menu-bar icon, application menus, About page, and installation diagnostics are present.
- [ ] Quit and relaunch preserves window geometry and settings.

## Fresh library

- [ ] Empty collections use the correct icon and explanatory text without zero-count badges.
- [ ] Copy text, an image, multiple files, and a PDF; each appears once with the expected preview and metadata.
- [ ] Search, Smart Bins, manual Bin assignment, pinning, protection, notes, Trash, and restore behave correctly.
- [ ] OCR explains its permission/state, processes eligible images, and remains searchable after relaunch.

## Existing library and recovery

- [ ] Launching with a pre-release database migrates without losing clips, Bins, settings, revisions, or Queue order.
- [ ] Export prompts for a destination and produces a restorable backup.
- [ ] Import merges the backup and refreshes all collection counts immediately.
- [ ] Factory Reset shows its confirmation/animation, relaunches successfully, and returns to fresh defaults.

## Scale and appearance

- [ ] A library near the default 1,000-clip retention limit opens, scrolls, searches, selects, and switches collections without unacceptable stalls.
- [ ] Every appearance scheme renders the main window, Settings, Tools pages, menus, dialogs, disabled controls, selected rows, and hover states without unthemed surfaces.
- [ ] Small, Medium, and Large Row Height visibly change text density, image preview height, and card spacing.

## Operating-system integration

- [ ] Denying Accessibility leaves capture usable and explains which hotkey/paste behavior is unavailable.
- [ ] Granting Accessibility makes global hotkeys, HUD paste, Paste Next, and Paste All work.
- [ ] Queue paste targets the previously focused app, keeps Pasted visible, consumes only successful items, and creates no duplicate history clip.
- [ ] Launch at Login and Dock/menu-bar visibility settings survive relaunch.
- [ ] CLI installation produces a working `pasted`, and representative list/search/Bin/Transform commands match the GUI.

## Publication

- [ ] Publish the DMG, SHA-256 checksum, system requirements, privacy summary, and `RELEASE_NOTES_1.0.0.md` together.
- [ ] State that 1.0 supports macOS 13+ on Apple Silicon and Intel with manual DMG updates.
- [ ] Label the Linux AppImage as a preview and Windows packages as unsigned experimental downloads.
- [ ] Preserve the notarization submission identifier and final release commit in the release record.
