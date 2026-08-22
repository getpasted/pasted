import { useCallback, type Dispatch, type SetStateAction } from 'react';
import type { Bin, ClipItem } from '../types';
import { clipsApi } from '../api/clips';
import { soundManager } from '../utils/sound';

interface AssignOptions {
  includeSelection?: boolean;
  playSound?: boolean;
}

interface ClipBinActionsInput {
  allClips: ClipItem[];
  bins: Bin[];
  selectedClipIds: Set<number>;
  setAllClips: Dispatch<SetStateAction<ClipItem[]>>;
  setBins: Dispatch<SetStateAction<Bin[]>>;
  setSelectedClip: Dispatch<SetStateAction<ClipItem | null>>;
  setTransformingClipIds: Dispatch<SetStateAction<Set<number>>>;
  setTransformErrorsByClipId: Dispatch<SetStateAction<Map<number, string>>>;
  fetchBins: () => Promise<void>;
  fetchClips: () => Promise<void>;
}

export function useClipBinActions({
  allClips,
  bins,
  selectedClipIds,
  setAllClips,
  setBins,
  setSelectedClip,
  setTransformingClipIds,
  setTransformErrorsByClipId,
  fetchBins,
  fetchClips,
}: ClipBinActionsInput) {
  const assignClipToBin = useCallback(async (
    clipId: number,
    binId: number | null,
    options: AssignOptions = {},
  ) => {
    const targetIds = options.includeSelection
      && selectedClipIds.size > 1
      && selectedClipIds.has(clipId)
      ? Array.from(selectedClipIds)
      : [clipId];
    const targetClips = allClips.filter((clip) => targetIds.includes(clip.id));
    const manualBinIds = new Set(bins.filter((bin) => !bin.smart_rule).map((bin) => bin.id));
    const targetBinProtects = binId !== null
      && Boolean(bins.find((bin) => bin.id === binId)?.protect_clips);

    const updateClip = (clip: ClipItem) => {
      if (!targetIds.includes(clip.id)) return clip;
      const currentBinIds = clip.bin_ids || [];
      const currentProtectingBinIds = clip.protecting_bin_ids || [];
      const explicitlyProtected = clip.is_explicitly_protected
        ?? (Boolean(clip.is_protected) && !clip.hotkey && currentProtectingBinIds.length === 0);
      if (binId === null) {
        const nextProtectingBinIds = currentProtectingBinIds.filter((id) => !manualBinIds.has(id));
        return {
          ...clip,
          bin_id: null,
          bin_ids: currentBinIds.filter((id) => !manualBinIds.has(id)),
          protecting_bin_ids: nextProtectingBinIds,
          is_protected: explicitlyProtected || Boolean(clip.hotkey) || nextProtectingBinIds.length > 0,
        };
      }
      const nextProtectingBinIds = targetBinProtects && !currentProtectingBinIds.includes(binId)
        ? [...currentProtectingBinIds, binId]
        : currentProtectingBinIds;
      return {
        ...clip,
        bin_id: binId,
        bin_ids: currentBinIds.includes(binId) ? currentBinIds : [...currentBinIds, binId],
        protecting_bin_ids: nextProtectingBinIds,
        is_protected: explicitlyProtected || Boolean(clip.hotkey) || nextProtectingBinIds.length > 0,
      };
    };
    setAllClips((previous) => previous.map(updateClip));
    setSelectedClip((previous) => previous ? updateClip(previous) : previous);

    setBins((previous) => previous.map((bin) => {
      if (bin.smart_rule) return bin;
      let delta = 0;
      for (const clip of targetClips) {
        const oldBinIds = new Set((clip.bin_ids || []).filter((id) => manualBinIds.has(id)));
        if (binId === null && oldBinIds.has(bin.id)) delta -= 1;
        if (bin.id === binId && !oldBinIds.has(bin.id)) delta += 1;
      }
      return delta === 0 ? bin : { ...bin, clip_count: Math.max(0, (bin.clip_count || 0) + delta) };
    }));

    if (options.playSound) {
      requestAnimationFrame(() => soundManager.playCopySound());
    }

    if (binId !== null) {
      setTransformingClipIds((previous) => {
        const next = new Set(previous);
        targetIds.forEach((id) => next.add(id));
        return next;
      });
      setTransformErrorsByClipId((previous) => {
        if (!targetIds.some((id) => previous.has(id))) return previous;
        const next = new Map(previous);
        targetIds.forEach((id) => next.delete(id));
        return next;
      });
    }

    try {
      if (targetIds.length > 1) {
        const outcome = await clipsApi.assignManyToBin(targetIds, binId);
        if (outcome.updatedClips.length > 0) {
          const updatedById = new Map(outcome.updatedClips.map((clip) => [clip.id, clip]));
          setAllClips((previous) => previous.map((clip) => updatedById.get(clip.id) ?? clip));
          setSelectedClip((previous) => previous
            ? updatedById.get(previous.id) ?? previous
            : previous);
        }
      } else {
        const transformedClip = await clipsApi.assignBin(clipId, binId);
        if (transformedClip) {
          // Replace the optimistic snapshot immediately so the selected
          // inspector and its metadata update in the same frame as the card.
          setAllClips((previous) => previous.map((clip) => (
            clip.id === transformedClip.id ? transformedClip : clip
          )));
          setSelectedClip((previous) => previous?.id === transformedClip.id
            ? transformedClip
            : previous);
        }
      }
    } catch (error) {
      console.error('Failed to assign clips to bin:', error);
      if (binId !== null) {
        setTransformErrorsByClipId((previous) => {
          const next = new Map(previous);
          const message = error instanceof Error ? error.message : String(error);
          targetIds.forEach((id) => next.set(id, message));
          return next;
        });
      }
      // The optimistic clip and count updates are authoritative on success.
      // Reconcile the complete data sets only when persistence fails.
      void fetchClips();
      void fetchBins();
    } finally {
      if (binId !== null) {
        setTransformingClipIds((previous) => {
          if (!targetIds.some((id) => previous.has(id))) return previous;
          const next = new Set(previous);
          targetIds.forEach((id) => next.delete(id));
          return next;
        });
      }
    }
  }, [allClips, bins, fetchBins, fetchClips, selectedClipIds, setAllClips, setBins, setSelectedClip]);

  const removeClipFromBin = useCallback(async (clipId: number, binId: number) => {
    const manualBinIds = new Set(bins.filter((bin) => !bin.smart_rule).map((bin) => bin.id));
    const updateClip = (clip: ClipItem) => {
      if (clip.id !== clipId) return clip;
      const nextBinIds = (clip.bin_ids || []).filter((id) => id !== binId);
      const nextProtectingBinIds = (clip.protecting_bin_ids || []).filter((id) => id !== binId);
      const nextPrimary = clip.bin_id === binId
        ? nextBinIds.find((id) => manualBinIds.has(id)) ?? null
        : clip.bin_id;
      const explicitlyProtected = clip.is_explicitly_protected
        ?? (Boolean(clip.is_protected) && !clip.hotkey && (clip.protecting_bin_ids?.length ?? 0) === 0);
      return {
        ...clip,
        bin_id: nextPrimary,
        bin_ids: nextBinIds,
        protecting_bin_ids: nextProtectingBinIds,
        is_protected: explicitlyProtected || Boolean(clip.hotkey) || nextProtectingBinIds.length > 0,
      };
    };
    setAllClips((previous) => previous.map(updateClip));
    setSelectedClip((previous) => previous ? updateClip(previous) : previous);
    setBins((previous) => previous.map((bin) => (
      bin.id === binId
        ? { ...bin, clip_count: Math.max(0, (bin.clip_count || 0) - 1) }
        : bin
    )));

    try {
      const outcome = await clipsApi.removeBin(clipId, binId);
      const updatedClip = outcome.updatedClips[0];
      if (updatedClip) {
        setAllClips((previous) => previous.map((clip) => clip.id === clipId ? updatedClip : clip));
        setSelectedClip((previous) => previous?.id === clipId ? updatedClip : previous);
      }
    } catch (error) {
      console.error('Failed to remove clip from Bin:', error);
      void fetchClips();
      void fetchBins();
    }
  }, [bins, fetchBins, fetchClips, setAllClips, setBins, setSelectedClip]);

  return { assignClipToBin, removeClipFromBin };
}
