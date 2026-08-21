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
