import { useCallback, useMemo, useState, type Dispatch, type SetStateAction } from 'react';
import type { Bin, ClipItem } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { soundManager } from '../utils/sound';

interface ClipDragPreview {
  clipId: number;
  x: number;
  y: number;
}

interface ClipBinDragInput {
  allClips: ClipItem[];
  setAllClips: Dispatch<SetStateAction<ClipItem[]>>;
  bins: Bin[];
  setBins: Dispatch<SetStateAction<Bin[]>>;
  selectedClipIds: Set<number>;
  enableSounds: boolean;
  fetchBins: () => Promise<void>;
  fetchClips: () => Promise<void>;
}

export function useClipBinDrag({
  allClips,
  setAllClips,
  bins,
  setBins,
  selectedClipIds,
  enableSounds,
  fetchBins,
  fetchClips,
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
      ?.closest<HTMLElement>('[data-bin-drop-bin-id]');
    if (!target) return null;
    const binId = Number(target.dataset.binDropBinId);
    return Number.isInteger(binId) && binId > 0 ? binId : null;
  }, []);

  const assignClipToBin = useCallback(async (clipId: number, binId: number) => {
    const isBatch = selectedClipIds.size > 1 && selectedClipIds.has(clipId);
    const targetIds = isBatch ? Array.from(selectedClipIds) : [clipId];
    const targetClips = allClips.filter((clip) => targetIds.includes(clip.id));
    const categoryBinIds = new Set(
      bins.filter((bin) => bin.bin_type !== 'tag').map((bin) => bin.id)
    );

    setAllClips((previous) => previous.map((clip) => {
      if (!targetIds.includes(clip.id)) return clip;
      const tagIds = (clip.bin_ids || []).filter((id) => !categoryBinIds.has(id));
      return { ...clip, bin_id: binId, bin_ids: [...tagIds, binId] };
    }));

    setBins((previous) => previous.map((bin) => {
      if (bin.bin_type === 'tag') return bin;
      let delta = 0;
      for (const clip of targetClips) {
        const oldBinIds = new Set([
          ...(clip.bin_ids || []).filter((id) => categoryBinIds.has(id)),
          ...(clip.bin_id && categoryBinIds.has(clip.bin_id) ? [clip.bin_id] : []),
        ]);
        if (oldBinIds.has(bin.id) && bin.id !== binId) delta -= 1;
        if (bin.id === binId && !oldBinIds.has(binId)) delta += 1;
      }
      return delta === 0
        ? bin
        : { ...bin, clip_count: Math.max(0, (bin.clip_count || 0) + delta) };
    }));

    soundManager.playCopySound(enableSounds);

    try {
      if (isBatch) {
        await invoke('batch_assign_bin_clips', { ids: targetIds, binId });
      } else {
        await invoke('assign_clip_bin', { clipId, binId });
      }
      void fetchBins();
      void fetchClips();
    } catch (error) {
      console.error('Failed to assign clip to bin:', error);
      void fetchClips();
      void fetchBins();
    }
  }, [allClips, bins, enableSounds, fetchBins, fetchClips, selectedClipIds, setAllClips, setBins]);

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
    assignClipToBin,
    finishClipPointerDrag,
  };
}
