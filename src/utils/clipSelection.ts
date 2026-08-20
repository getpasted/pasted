export function selectionIdsForContextMenu(
  selectedIds: Set<number>,
  clipId: number,
): Set<number> {
  return selectedIds.has(clipId) ? selectedIds : new Set([clipId]);
}
