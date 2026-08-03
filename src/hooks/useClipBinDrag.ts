import { useCallback, useMemo, useState, type Dispatch, type SetStateAction } from 'react';
import type { Bin, ClipItem } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';

interface ClipDragPreview {
  clipId: number;
  x: number;
  y: number;
}

interface ClipBinDragInput {
  allClips: ClipItem[];
  setAllClips: Dispatch<SetStateAction<ClipItem[]>>;
  bins: Bin[];
  selectedClipIds: Set<number>;
  fetchClips: () => Promise<void>;
  assignClipToBin: (clipId: number, binId: number) => Promise<void>;
}

export function useClipBinDrag({
  allClips,
  setAllClips,
  bins,
  selectedClipIds,
  fetchClips,
  assignClipToBin,
}: ClipBinDragInput) {
  const [draggedClipId, setDraggedClipId] = useState<number | null>(null);
  const [pointerDropTargetBinId, setPointerDropTargetBinId] = useState<number | null>(null);
  const [clipDragPreview, setClipDragPreview] = useState<ClipDragPreview | null>(null);

  const disabledDropBinId = useMemo(() => {
    if (draggedClipId === null) return null;
    const draggedIds = selectedClipIds.size > 1 && selectedClipIds.has(draggedClipId)
      ? Array.from(selectedClipIds)
      : [draggedClipId];
    const draggedClips = allClips.filter((clip) => draggedIds.includes(clip.id));
    if (draggedClips.length !== draggedIds.length) return null;
    const currentBinId = draggedClips[0]?.bin_id ?? null;
    if (currentBinId === null || !draggedClips.every((clip) => clip.bin_id === currentBinId)) {
      return null;
    }
    return bins.find((bin) => bin.id === currentBinId && bin.bin_type !== 'tag')
      ? currentBinId
      : null;
  }, [allClips, bins, draggedClipId, selectedClipIds]);

  const getPointerDropTarget = useCallback((x: number, y: number) => {
    const target = document
      .elementFromPoint(x, y)
      ?.closest<HTMLElement>('[data-bin-drop-id]');
    if (!target) return null;
    const binId = Number(target.dataset.binDropId);
    return Number.isInteger(binId) && binId > 0 ? binId : null;
  }, []);

  const finishClipPointerDrag = useCallback(async (x: number, y: number, clipId: number) => {
    const binId = getPointerDropTarget(x, y);
    setPointerDropTargetBinId(null);
    setClipDragPreview(null);

    if (binId !== null) {
      await assignClipToBin(clipId, binId);
      return;
    }

    const target = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-clip-id]');
    const targetId = Number(target?.dataset.clipId);
    if (!Number.isInteger(targetId) || targetId === clipId) return;

    const pinnedClips = allClips.filter((clip) => clip.is_pinned);
    const draggedIndex = pinnedClips.findIndex((clip) => clip.id === clipId);
    const targetIndex = pinnedClips.findIndex((clip) => clip.id === targetId);
    if (draggedIndex === -1 || targetIndex === -1) return;

    const reordered = [...pinnedClips];
    const [moved] = reordered.splice(draggedIndex, 1);
    reordered.splice(targetIndex, 0, moved);
    setAllClips([...reordered, ...allClips.filter((clip) => !clip.is_pinned)]);

    try {
      await invoke('reorder_pinned_clips', { ids: reordered.map((clip) => clip.id) });
    } catch (error) {
      console.error('Failed to save pin order:', error);
      void fetchClips();
    }
  }, [allClips, assignClipToBin, fetchClips, getPointerDropTarget, setAllClips]);

  return {
    draggedClipId,
    setDraggedClipId,
    pointerDropTargetBinId,
    setPointerDropTargetBinId,
    clipDragPreview,
    setClipDragPreview,
    disabledDropBinId,
    getPointerDropTarget,
    finishClipPointerDrag,
  };
}
