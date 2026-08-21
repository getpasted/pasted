export interface ClipFocusRequest {
  clipId: number;
  requestId: number;
  viewKey: string;
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
