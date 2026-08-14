# Notifications and Capture Feedback

Notifications provide quiet, optional feedback when Pasted captures or intentionally skips a clipboard item. They do not control capture itself: disabling notifications does not pause Pasted or prevent clips from entering History.

Enable **Notifications** under **Settings → Functionality**, then open **Settings → Notifications**.

## Capture feedback

**Capture feedback** shows a brief, non-focus-stealing confirmation near a screen corner. The feedback window appears on the display that currently contains the pointer.

Additional controls include:

- **Show skipped captures:** also reports items Pasted intentionally leaves alone;
- **Show clip preview:** displays the captured item with available quick actions;
- **Dismiss Preview After:** closes previews after 3, 5, 7, 10, 15, or 30 seconds, or leaves them until dismissed;
- **Screen Position:** chooses the top-left, top-right, bottom-left, or bottom-right corner.

The dismissal countdown pauses while the pointer is over a preview. Controls that depend on Capture feedback or previews remain unavailable until their parent option is enabled.

## Privacy

Capture feedback is a Pasted window and stays on the device. Pasted does not expose copied text, images, file names, or paths through operating-system notification services.

When **Show clip preview** is enabled, the local feedback window can visibly display the copied item. Disable previews before screen sharing or when on-screen content should remain concealed. Password-manager and sensitive-application capture rules remain governed separately by the blacklist and auto-pause behavior.

## Preview actions and safety

Available preview actions follow the same feature gates and safety rules as the main app. For example, protected or already trashed clips cannot be deleted from capture feedback. When Trash is enabled, deletion moves an eligible clip to Trash; when Trash is disabled, deletion is permanent.

## Troubleshooting

If feedback does not appear:

1. Confirm **Notifications** is enabled under **Settings → Functionality**.
2. Confirm **Capture feedback** is enabled under **Settings → Notifications**.
3. Enable **Show skipped captures** if the clipboard item was intentionally ignored.
4. Check the selected corner on the display containing the pointer.

A missing feedback window does not necessarily mean capture failed. Check History for the new clip and use [Troubleshooting and Diagnostics](Troubleshooting-and-Diagnostics) if capture itself is not working.
