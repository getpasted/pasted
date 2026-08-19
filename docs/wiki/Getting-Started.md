# Getting Started

## Install on macOS

1. Download the signed, notarized universal DMG from [GitHub Releases](https://github.com/getpasted/pasted/releases).
2. Open the DMG and drag Pasted into Applications.
3. Launch Pasted from Applications.
4. Grant Accessibility only if you want system-wide hotkeys or automatic Queue/HUD paste.

Pasted begins listening while it is running. Copy text, an image, a screenshot, a PDF, or files and they appear in **History**.

The first-run setup explains local storage, offers to merge supported Alfred, Pastebot, Pasta, Paste, CopyClip 2, Maccy, or Flycut text history, and checks hotkey access. Every step can be skipped, and the setup can be reopened from **Settings → General**.

Scripted and managed installations can persistently mark the current setup walkthrough complete:

```sh
open -a Pasted --args --skip-welcome
```

## The three-column window

- **Left:** collections, Bins, Tools, and Search.
- **Middle:** clips in the current collection.
- **Right:** the selected clip, its content, actions, metadata, notes, and revisions.

Drag the column dividers to resize them. Pasted remembers the window position and layout.

## First useful actions

- Click a clip to inspect it.
- Right-click for Copy, Bin, Transform, Note, Queue, Pin, Protect, and Trash actions.
- Drag a clip onto a manual Bin, Queue, Pinned, Protected, or Trash.
- Search across active and trashed history from the lower-left Search control.
- Open **Settings → Functionality** for a simpler or more capable Pasted.

The **Simple** Functionality preset keeps the core clipboard experience visible. **Full** enables every feature, and changing individual features creates a **Custom** setup. Hiding a feature normally preserves its existing data. Review [Settings and Features](Settings-and-Features) before disabling Trash or Revision History, because those choices affect whether future actions are reversible.

## Default hotkeys

| Hotkey | Action |
| --- | --- |
| `⌥⇧V` | Open HUD |
| `⌥⇧C` | Start or stop Queue recording |
| `⌥⇧X` | Paste Next |
| `1`–`9` | Paste that HUD result |
| `Esc` | Close HUD, a menu, or a modal |

Change or disable hotkeys in **Settings → Hotkeys**.
