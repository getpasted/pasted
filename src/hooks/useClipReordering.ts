import { useCallback, useMemo, type RefObject } from 'react';
import type { ClipItem, SequentialStatus } from '../types';
import type { ClipCollectionDefinition } from '../utils/clipCollections';
import { orderClipsForStableReorder } from '../utils/clipListViewport';
import { safeInvoke as invoke } from '../utils/tauri';
import { useStableVerticalReorder } from './useStableVerticalReorder';

interface UseClipReorderingOptions {
  collection?: ClipCollectionDefinition | null;
  selectedBinId: number | null;
  displayedClips: ClipItem[];
  sequentialStatus: SequentialStatus | null;
  loadedClipCount: number;
  totalClipCount: number;
  clipListRef: RefObject<HTMLElement | null>;
  fetchBins: () => Promise<void>;
  fetchSequentialStatus: () => Promise<void>;
}

export function useClipReordering({
  collection,
  selectedBinId,
  displayedClips,
  sequentialStatus,
  loadedClipCount,
  totalClipCount,
  clipListRef,
  fetchBins,
  fetchSequentialStatus,
}: UseClipReorderingOptions) {
  const isQueueCollection = collection?.membership === 'queue';
  const isBinCollection = collection?.membership === 'bin' && selectedBinId !== null;
  const queueReorderIds = useMemo(
    () => isQueueCollection ? (sequentialStatus?.item_ids ?? []).map(String) : [],
    [isQueueCollection, sequentialStatus?.item_ids],
  );
  const commitQueueOrder = useCallback((orderedIds: string[]) => {
    return invoke('reorder_sequential_items', { itemIds: orderedIds.map(Number) })
      .then(fetchSequentialStatus)
      .catch((error) => {
        console.error('Failed to reorder Copy Queue:', error);
        return fetchSequentialStatus();
      });
  }, [fetchSequentialStatus]);
  const queueReorder = useStableVerticalReorder({
    itemIds: queueReorderIds,
    containerRef: clipListRef,
    onCommit: commitQueueOrder,
    disabled: !collection?.capabilities.canReorder || !isQueueCollection || queueReorderIds.length < 2,
  });

  const binReorderIds = useMemo(
    () => isBinCollection ? displayedClips.map((clip) => String(clip.id)) : [],
    [displayedClips, isBinCollection],
  );
  const commitBinOrder = useCallback((orderedIds: string[]) => {
    if (selectedBinId === null) return;
    return invoke('reorder_bin_clips', {
      binId: selectedBinId,
      clipIds: orderedIds.map(Number),
    })
      .then(fetchBins)
      .catch((error) => {
        console.error('Failed to save Bin clip order:', error);
        void fetchBins();
      });
  }, [fetchBins, selectedBinId]);
  const binClipReorder = useStableVerticalReorder({
    itemIds: binReorderIds,
    containerRef: clipListRef,
    onCommit: commitBinOrder,
    disabled: !collection?.capabilities.canReorder
      || !isBinCollection
      || loadedClipCount < totalClipCount
      || binReorderIds.length < 2,
  });

  const reorderIdsForClip = useCallback((clip: ClipItem, _index: number) => {
    const queueId = isQueueCollection ? String(-clip.id) : undefined;
    const binId = isBinCollection ? String(clip.id) : undefined;
    return { queueId, binId, stableId: queueId ?? binId };
  }, [isBinCollection, isQueueCollection]);
  const settlingOrder = queueReorder.settlingOrder ?? binClipReorder.settlingOrder;
  const displayedClipsForRender = useMemo(() => orderClipsForStableReorder(
    displayedClips,
    settlingOrder,
    (clip) => String(isQueueCollection ? -clip.id : clip.id),
  ), [displayedClips, isQueueCollection, settlingOrder]);

  return {
    binClipReorder,
    isBinCollection,
    isQueueCollection,
    queueReorder,
    reorderIdsForClip,
    displayedClipsForRender,
  };
}
