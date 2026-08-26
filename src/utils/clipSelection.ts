export interface ClipFocusRequest {
  clipId: number;
  requestId: number;
  viewKey: string;
}

export function clipCollectionViewKey(currentTab: string, selectedBinId: number | null): string {
  return currentTab === 'bin' ? `bin:${selectedBinId ?? 'none'}` : `section:${currentTab}`;
}

export function pendingClipFocusId(
  request: ClipFocusRequest | null | undefined,
  viewKey: string,
  handledRequestId: number | null,
): number | null {
  return request?.viewKey === viewKey && request.requestId !== handledRequestId
    ? request.clipId
    : null;
}

export function selectionIdsForContextMenu(
  selectedIds: Set<number>,
  clipId: number,
): Set<number> {
  return selectedIds.has(clipId) ? selectedIds : new Set([clipId]);
}

export function isSelectAllShortcut(event: Pick<KeyboardEvent, 'altKey' | 'ctrlKey' | 'key' | 'metaKey'>): boolean {
  return !event.altKey && (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'a';
}

export function clipIdsForSelectAll(clips: Array<{ id: number }>): Set<number> {
  return new Set(clips.map(({ id }) => id));
}
