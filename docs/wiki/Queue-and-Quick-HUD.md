# Queue and HUD

## Copy Queue

Queue records an ordered sequence for later paste. Add clips with the context menu, drag clips onto Queue, or enable Queue recording.

- Queue order persists across relaunches.
- Drag rows to reorder them.
- **Paste Next** consumes one item only after a successful target and paste.
- **Paste All** combines queued text into one internal clipboard write, pastes once, and avoids creating duplicate history.
- A failed target or paste keeps the affected items in Queue.

Automatic paste targets the previously focused application. Pasted never treats itself as the intended destination.

## HUD

HUD is the compact keyboard-driven history window.

- Type to filter.
- Use arrow keys to change selection.
- Press `1`–`9` for visible numbered results.
- Press Enter to paste the selected result.
- Press Esc to close.

Internal clipboard writes made for HUD and Queue paste are marked and excluded from ordinary capture history.

## Platform limits

macOS automatic paste requires Accessibility permission. Windows and X11 use platform-specific focus/paste integrations. Constrained Wayland sessions may decline system-wide shortcuts or automatic paste; Pasted reports that capability failure and does not consume queued data.
