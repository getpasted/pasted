import { useCallback, useState, type Dispatch, type SetStateAction } from 'react';
import { type AppSettings, type Bin, type ClipItem, type ManualTransform, type SavedTransform } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { clipsApi } from '../api/clips';
import { sortClipsForTimeline } from '../utils/clipOrder';
import { soundManager } from '../utils/sound';
import { runTransformation } from '../utils/transformExecution';
import { htmlToPlainText } from '../utils/plainText';

interface AssignOptions {
  includeSelection?: boolean;
  playSound?: boolean;
}

interface ClipActionsInput {
  allClips: ClipItem[];
  setAllClips: Dispatch<SetStateAction<ClipItem[]>>;
  setTrashedClips: Dispatch<SetStateAction<ClipItem[]>>;
  bins: Bin[];
  setBins: Dispatch<SetStateAction<Bin[]>>;
  setSelectedClip: Dispatch<SetStateAction<ClipItem | null>>;
  selectedClipIds: Set<number>;
  setSelectedClipIds: Dispatch<SetStateAction<Set<number>>>;
  setTotalClipCount: Dispatch<SetStateAction<number>>;
  settings: Pick<AppSettings, 'alwaysPastePlainText' | 'enableTrash'>;
  fetchBins: () => Promise<void>;
  fetchClips: () => Promise<void>;
  fetchTrashedClips: () => Promise<void>;
  fetchSequentialStatus: () => Promise<void>;
  queuedIndexMap: Map<string, number>;
  onCollectionChanged: () => Promise<void>;
  keepTrashedClipsVisible: boolean;
  onClipsRepositioned?: (ids: number[]) => void;
}

export function useClipActions({
  allClips,
  setAllClips,
  setTrashedClips,
  bins,
  setBins,
  setSelectedClip,
  selectedClipIds,
  setSelectedClipIds,
  setTotalClipCount,
  settings,
  fetchBins,
  fetchClips,
  fetchTrashedClips,
  fetchSequentialStatus,
  queuedIndexMap,
  onCollectionChanged,
  keepTrashedClipsVisible,
  onClipsRepositioned,
}: ClipActionsInput) {
  const [transformingClipIds, setTransformingClipIds] = useState<Set<number>>(() => new Set());
  const [transformErrorsByClipId, setTransformErrorsByClipId] = useState<Map<number, string>>(() => new Map());

  const runClipTransformationJob = useCallback(async <T,>(clipId: number, job: () => Promise<T>) => {
    setTransformingClipIds((previous) => new Set(previous).add(clipId));
    setTransformErrorsByClipId((previous) => {
      if (!previous.has(clipId)) return previous;
      const next = new Map(previous);
      next.delete(clipId);
      return next;
    });
    try {
      return await job();
    } catch (error) {
      setTransformErrorsByClipId((previous) => {
        const next = new Map(previous);
        next.set(clipId, error instanceof Error ? error.message : String(error));
        return next;
      });
      throw error;
    } finally {
      setTransformingClipIds((previous) => {
        if (!previous.has(clipId)) return previous;
        const next = new Set(previous);
        next.delete(clipId);
        return next;
      });
    }
  }, []);

  const togglePin = useCallback((id: number) => {
    const isBatch = selectedClipIds.size > 1 && selectedClipIds.has(id);
    const targetIds = isBatch ? Array.from(selectedClipIds) : [id];
    const nextPinState = !(allClips.find((clip) => clip.id === id)?.is_pinned ?? false);

    const targetIdSet = new Set(targetIds);
    onClipsRepositioned?.(allClips
      .filter((clip) => targetIdSet.has(clip.id) && Boolean(clip.is_pinned) !== nextPinState)
      .map((clip) => clip.id));
    setAllClips((previous) => {
      const updated = previous.map((clip) => (
        targetIdSet.has(clip.id) ? { ...clip, is_pinned: nextPinState } : clip
      ));
      if (nextPinState) {
        const newlyPinned = updated
          .filter((clip) => targetIdSet.has(clip.id))
          .map((clip, index) => ({ ...clip, pin_order: index }));
        const existingPinned = updated
          .filter((clip) => clip.is_pinned && !targetIdSet.has(clip.id))
          .map((clip) => ({ ...clip, pin_order: (clip.pin_order ?? 0) + newlyPinned.length }));
        return sortClipsForTimeline([
          ...newlyPinned,
          ...existingPinned,
          ...updated.filter((clip) => !clip.is_pinned),
        ]);
      }
      return sortClipsForTimeline(updated);
    });
    setSelectedClip((previous) => previous && targetIds.includes(previous.id)
      ? { ...previous, is_pinned: nextPinState, pin_order: nextPinState ? 0 : previous.pin_order }
      : previous);

    const request = isBatch
      ? clipsApi.setPinned(targetIds, nextPinState)
      : clipsApi.togglePin(id);
    void request
      .then(onCollectionChanged)
      .catch((error) => {
        console.error('Failed to update pinned state:', error);
        void fetchClips();
      });
  }, [allClips, fetchClips, onClipsRepositioned, onCollectionChanged, selectedClipIds, setAllClips, setSelectedClip]);

  const toggleProtected = useCallback((id: number) => {
    const current = allClips.find((clip) => clip.id === id);
    const explicit = current?.is_explicitly_protected ?? current?.is_protected ?? false;
    if (!current || current.hotkey || current.protecting_bin_ids?.length) return;
    const nextExplicit = !explicit;
    const update = (clip: ClipItem) => clip.id === id ? {
      ...clip,
      is_explicitly_protected: nextExplicit,
      is_protected: nextExplicit || Boolean(clip.protecting_bin_ids?.length),
    } : clip;
    setAllClips((previous) => previous.map((clip) => (
      update(clip)
    )));
    setSelectedClip((previous) => previous?.id === id
      ? update(previous)
      : previous);

    void invoke('toggle_clip_protected', { clipId: id })
      .then(onCollectionChanged)
      .catch((error) => {
        console.error('Failed to toggle protected state:', error);
        void fetchClips();
      });
  }, [allClips, fetchClips, onCollectionChanged, setAllClips, setSelectedClip]);

  const setPinned = useCallback((id: number, pinState: boolean) => {
    const targetIds = selectedClipIds.size > 1 && selectedClipIds.has(id)
      ? Array.from(selectedClipIds)
      : [id];
    const targetIdSet = new Set(targetIds);
    const idsToChange = allClips
      .filter((clip) => targetIdSet.has(clip.id) && Boolean(clip.is_pinned) !== pinState)
      .map((clip) => clip.id);
    if (idsToChange.length === 0) return;
    const changedIdSet = new Set(idsToChange);

    onClipsRepositioned?.(idsToChange);
    setAllClips((previous) => {
      const updated = previous.map((clip) => (
        changedIdSet.has(clip.id) ? { ...clip, is_pinned: pinState } : clip
      ));
      if (pinState) {
        const newlyPinned = updated
          .filter((clip) => changedIdSet.has(clip.id))
          .map((clip, index) => ({ ...clip, pin_order: index }));
        const existingPinned = updated
          .filter((clip) => clip.is_pinned && !changedIdSet.has(clip.id))
          .map((clip) => ({ ...clip, pin_order: (clip.pin_order ?? 0) + newlyPinned.length }));
        return sortClipsForTimeline([
          ...newlyPinned,
          ...existingPinned,
          ...updated.filter((clip) => !clip.is_pinned),
        ]);
      }
      return sortClipsForTimeline(updated);
    });
    setSelectedClip((previous) => previous && changedIdSet.has(previous.id)
      ? { ...previous, is_pinned: pinState, pin_order: pinState ? 0 : previous.pin_order }
      : previous);

    void clipsApi.setPinned(idsToChange, pinState)
      .then(onCollectionChanged)
      .catch((error) => {
        console.error('Failed to set pinned state:', error);
        void fetchClips();
      });
  }, [allClips, fetchClips, onClipsRepositioned, onCollectionChanged, selectedClipIds, setAllClips, setSelectedClip]);

  const setProtected = useCallback((id: number, protectedState: boolean) => {
    const targetIds = selectedClipIds.size > 1 && selectedClipIds.has(id)
      ? Array.from(selectedClipIds)
      : [id];
    const idsToChange = allClips
      .filter((clip) => targetIds.includes(clip.id)
        && !clip.hotkey
        && (protectedState || !clip.protecting_bin_ids?.length)
        && Boolean(clip.is_explicitly_protected ?? clip.is_protected) !== protectedState)
      .map((clip) => clip.id);
    if (idsToChange.length === 0) return;
    const changedIdSet = new Set(idsToChange);

    setAllClips((previous) => previous.map((clip) => (
      changedIdSet.has(clip.id) ? {
        ...clip,
        is_explicitly_protected: protectedState,
        is_protected: protectedState || Boolean(clip.protecting_bin_ids?.length),
      } : clip
    )));
    setSelectedClip((previous) => previous && changedIdSet.has(previous.id)
      ? {
        ...previous,
        is_explicitly_protected: protectedState,
        is_protected: protectedState || Boolean(previous.protecting_bin_ids?.length),
      }
      : previous);

    void clipsApi.setProtected(idsToChange, protectedState)
      .then(onCollectionChanged)
      .catch((error) => {
        console.error('Failed to set protected state:', error);
        void fetchClips();
      });
  }, [allClips, fetchClips, onCollectionChanged, selectedClipIds, setAllClips, setSelectedClip]);

  const deleteClipIds = useCallback((requestedIds: number[], forcePermanent = false) => {
    const ids = requestedIds.filter((id) => !allClips.find((clip) => clip.id === id)?.is_protected);
    if (ids.length === 0) return;

    const deletedItems = allClips
      .filter((clip) => ids.includes(clip.id))
      .map((clip) => ({
        ...clip,
        is_trashed: true,
        bin_id: null,
        bin_ids: [],
      }));
    const deletedSourceClips = allClips.filter((clip) => ids.includes(clip.id));
    setBins((previous) => previous.map((bin) => {
      const removedCount = deletedSourceClips.filter((clip) => (
        clip.bin_id === bin.id || Boolean(clip.bin_ids?.includes(bin.id))
      )).length;
      return removedCount === 0
        ? bin
        : { ...bin, clip_count: Math.max(0, (bin.clip_count || 0) - removedCount) };
    }));
    const permanently = forcePermanent || settings.enableTrash === false;
    setAllClips((previous) => previous.filter((clip) => !ids.includes(clip.id)));
    if (!permanently) {
      setTrashedClips((previous) => [...deletedItems, ...previous]);
    }
    const keepMovedSelection = !permanently && keepTrashedClipsVisible;
    if (!keepMovedSelection) {
      setSelectedClipIds((previous) => new Set(Array.from(previous).filter((id) => !ids.includes(id))));
    }
    setSelectedClip((previous) => {
      if (!previous || !ids.includes(previous.id)) return previous;
      return keepMovedSelection
        ? deletedItems.find((clip) => clip.id === previous.id) ?? previous
        : null;
    });
    setTotalClipCount((previous) => Math.max(0, previous - ids.length));

    const request = permanently
      ? Promise.all(ids.map((id) => clipsApi.purge(id)))
      : ids.length > 1
        ? clipsApi.trashMany(ids)
        : clipsApi.trash(ids[0]);
    void request
      // Moving a clip to Trash is already fully represented in local clip,
      // selection, Bin-count, Trash-count, and total-count state. Refetching
      // both collections on success delayed the visible drop completion.
      // Permanent deletion still reconciles because it may originate from the
      // separately loaded Trash collection.
      .then(() => permanently
        ? Promise.all([fetchClips(), fetchTrashedClips()])
        : undefined)
      .then(onCollectionChanged)
      .catch((error) => {
        console.error(permanently ? 'Failed to permanently delete clips:' : 'Failed to trash clips:', error);
        void fetchClips();
        void fetchTrashedClips();
      });
  }, [allClips, bins, fetchClips, fetchTrashedClips, keepTrashedClipsVisible, onCollectionChanged, setAllClips, setSelectedClip, setSelectedClipIds, setTotalClipCount, setTrashedClips, settings.enableTrash]);

  const deleteSelectedClips = useCallback((forcePermanent = false) => {
    deleteClipIds(Array.from(selectedClipIds), forcePermanent);
  }, [deleteClipIds, selectedClipIds]);

  const deleteClip = useCallback((id: number, forcePermanent = false) => {
    const ids = selectedClipIds.size > 1 && selectedClipIds.has(id)
      ? Array.from(selectedClipIds)
      : [id];
    deleteClipIds(ids, forcePermanent);
  }, [deleteClipIds, selectedClipIds]);

  const copyClip = useCallback(async (clip: ClipItem) => {
    try {
      if (clip.content_type === 'image' || clip.content_type === 'file') {
        await clipsApi.copyById(clip.id);
        soundManager.playCopySound();
        return;
      }
      const text = settings.alwaysPastePlainText && clip.text_content
        ? htmlToPlainText(clip.text_content)
        : clip.text_content;
      await clipsApi.copyContent(text, null);
      soundManager.playCopySound();
    } catch (error) {
      console.error('Failed to copy clip:', error);
    }
  }, [settings.alwaysPastePlainText]);

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

  const runManualTransformForClip = useCallback(async (
    clip: ClipItem,
    manualTransform: ManualTransform,
    destination: 'copy' | 'paste' = 'copy',
  ) => {
    if (!clip.text_content) return;
    try {
      await runClipTransformationJob(clip.id, async () => {
        const transformed = await runTransformation(
          clip.text_content!,
          { kind: 'manual_transform', transformRef: manualTransform.stableRef },
          { sourceClipId: clip.id, destination },
        );
        if (destination === 'paste') {
          await invoke('paste_text_to_frontmost', { text: transformed.output });
          soundManager.playPasteSound();
        } else {
          await clipsApi.copyContent(transformed.output, null);
          soundManager.playCopySound();
        }
      });
    } catch (error) {
      console.error(`Failed to ${destination} Advanced Transform result:`, error);
    }
  }, [runClipTransformationJob]);

  const runTransformForClip = useCallback(async (clip: ClipItem, transform: SavedTransform) => {
    if (!clip.text_content) return;
    try {
      await runClipTransformationJob(clip.id, async () => {
        const transformed = await runTransformation(
          clip.text_content!,
          { kind: 'transform', transformRef: transform.stableRef },
          { sourceClipId: clip.id, destination: 'copy' },
        );
        await clipsApi.copyContent(transformed.output, null);
        soundManager.playCopySound();
      });
    } catch (error) {
      console.error('Failed to copy Transform result:', error);
    }
  }, [runClipTransformationJob]);

  const addToSequentialStack = useCallback(async (clip: ClipItem) => {
    const item = clip.content_type === 'file' ? null : clip.text_content;
    if (!item) {
      console.warn('Only clips containing text can be added to the Copy Queue');
      return;
    }
    try {
      await invoke('push_sequential_item', { item });
      soundManager.playStackSound();
      void fetchSequentialStatus();
    } catch (error) {
      console.error('Failed to add clip to queue:', error);
    }
  }, [fetchSequentialStatus]);

  const toggleSequentialStack = useCallback(async (clip: ClipItem) => {
    const item = clip.content_type === 'file' ? null : clip.text_content;
    if (!item) return;
    const queueIndex = queuedIndexMap.get(item);
    if (queueIndex === undefined) {
      await addToSequentialStack(clip);
      return;
    }
    await invoke('remove_sequential_item_by_index', { index: queueIndex - 1 });
    await fetchSequentialStatus();
  }, [addToSequentialStack, fetchSequentialStatus, queuedIndexMap]);

  const updateClipNoteLocally = useCallback((clipId: number, note: string | null) => {
    setAllClips((previous) => previous.map((clip) => clip.id === clipId ? { ...clip, note } : clip));
    setSelectedClip((previous) => previous?.id === clipId ? { ...previous, note } : previous);
  }, [setAllClips, setSelectedClip]);

  const deleteNoteFromClip = useCallback(async (clipId: number) => {
    updateClipNoteLocally(clipId, null);
    try {
      await clipsApi.updateNote(clipId, null);
      await onCollectionChanged();
    } catch (error) {
      console.error('Failed to delete clip note:', error);
      void fetchClips();
    }
  }, [fetchClips, onCollectionChanged, updateClipNoteLocally]);

  return {
    togglePin,
    toggleProtected,
    setPinned,
    setProtected,
    deleteSelectedClips,
    deleteClip,
    copyClip,
    assignClipToBin,
    removeClipFromBin,
    runManualTransformForClip,
    runTransformForClip,
    addToSequentialStack,
    toggleSequentialStack,
    updateClipNoteLocally,
    deleteNoteFromClip,
    transformingClipIds,
    transformErrorsByClipId,
  };
}
