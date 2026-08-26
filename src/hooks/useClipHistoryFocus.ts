import { useCallback, useRef, useState, type Dispatch, type SetStateAction } from 'react';
import type { ClipItem } from '../types';
import type { ClipPropertyAssociationId } from '../utils/clipPropertyAssociations';
import type { ClipFocusRequest } from '../utils/clipSelection';

interface UseClipHistoryFocusOptions {
  currentTab: string;
  currentAssociation?: ClipPropertyAssociationId;
  selectedClip: ClipItem | null;
  setCurrentTab: Dispatch<SetStateAction<string>>;
  setSelectedBinId: Dispatch<SetStateAction<number | null>>;
  restoreClip: (clipId: number) => Promise<void>;
}

export function useClipHistoryFocus({
  currentTab,
  currentAssociation,
  selectedClip,
  setCurrentTab,
  setSelectedBinId,
  restoreClip,
}: UseClipHistoryFocusOptions) {
  const [focusRequest, setFocusRequest] = useState<ClipFocusRequest | null>(null);
  const requestIdRef = useRef(0);

  const focusClipInHistory = useCallback((clipId: number) => {
    requestIdRef.current += 1;
    setFocusRequest({ clipId, requestId: requestIdRef.current, viewKey: 'section:all' });
    setSelectedBinId(null);
    setCurrentTab('all');
  }, [setCurrentTab, setSelectedBinId]);

  const handlePropertyRemoved = useCallback((association: ClipPropertyAssociationId, ids: number[]) => {
    if (currentAssociation !== association || ids.length === 0) return;
    const clipId = selectedClip && ids.includes(selectedClip.id) ? selectedClip.id : ids[0];
    focusClipInHistory(clipId);
  }, [currentAssociation, focusClipInHistory, selectedClip]);

  const restoreClipToHistory = useCallback((clipId: number) => {
    const restore = restoreClip(clipId);
    if (currentTab === 'trash') focusClipInHistory(clipId);
    return restore;
  }, [currentTab, focusClipInHistory, restoreClip]);

  const restoreClipsFromTrash = useCallback(
    (clipIds: number[]) => Promise.all(clipIds.map((clipId) => restoreClip(clipId))),
    [restoreClip],
  );

  return { focusRequest, handlePropertyRemoved, restoreClipToHistory, restoreClipsFromTrash };
}
