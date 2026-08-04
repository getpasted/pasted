export const UI_COPY = {
  copy: 'Copy',
  copied: 'Copied',
  moveToTrash: 'Move to Trash',
  deletePermanently: 'Delete Permanently',
  restore: 'Restore',
  pin: 'Pin',
  unpin: 'Unpin',
  protect: 'Protect',
  unprotect: 'Unprotect',
} as const;

export function clipDeleteLabel({
  trashEnabled,
  permanent = false,
}: {
  trashEnabled: boolean;
  permanent?: boolean;
}) {
  return !trashEnabled || permanent ? UI_COPY.deletePermanently : UI_COPY.moveToTrash;
}

export function selectedClipDeleteLabel({
  count,
  trashEnabled,
  permanent = false,
}: {
  count: number;
  trashEnabled: boolean;
  permanent?: boolean;
}) {
  if (count <= 1) return clipDeleteLabel({ trashEnabled, permanent });
  return !trashEnabled || permanent
    ? `Delete ${count} Permanently`
    : `Move ${count} to Trash`;
}
