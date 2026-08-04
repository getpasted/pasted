import { useCallback, type Dispatch, type SetStateAction } from 'react';
import type { AppSettings, Bin, ClipItem, FilterRule } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { soundManager } from '../utils/sound';

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
  settings: Pick<AppSettings, 'alwaysPastePlainText' | 'enableSounds' | 'enableTrash'>;
  fetchBins: () => Promise<void>;
  fetchClips: () => Promise<void>;
  fetchTrashedClips: () => Promise<void>;
  fetchSequentialStatus: () => Promise<void>;
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
}: ClipActionsInput) {
  const togglePin = useCallback((id: number) => {
    const isBatch = selectedClipIds.size > 1 && selectedClipIds.has(id);
    const targetIds = isBatch ? Array.from(selectedClipIds) : [id];
    const nextPinState = !(allClips.find((clip) => clip.id === id)?.is_pinned ?? false);

    const targetIdSet = new Set(targetIds);
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
        return [
          ...newlyPinned,
          ...existingPinned,
          ...updated.filter((clip) => !clip.is_pinned),
        ];
      }
      return [
        ...updated.filter((clip) => clip.is_pinned),
        ...updated.filter((clip) => !clip.is_pinned),
      ];
    });
    setSelectedClip((previous) => previous && targetIds.includes(previous.id)
      ? { ...previous, is_pinned: nextPinState, pin_order: nextPinState ? 0 : previous.pin_order }
      : previous);

    const request = isBatch
      ? invoke('batch_pin_clips', { ids: targetIds, pinState: nextPinState })
      : invoke('toggle_pin_clip', { id });
    void request.catch((error) => {
      console.error('Failed to update pinned state:', error);
      void fetchClips();
    });
  }, [allClips, fetchClips, selectedClipIds, setAllClips, setSelectedClip]);

  const toggleProtected = useCallback((id: number) => {
    setAllClips((previous) => previous.map((clip) => (
      clip.id === id ? { ...clip, is_protected: !clip.is_protected } : clip
    )));
    setSelectedClip((previous) => previous?.id === id
      ? { ...previous, is_protected: !previous.is_protected }
      : previous);

    void invoke('toggle_clip_protected', { clipId: id }).catch((error) => {
      console.error('Failed to toggle protected state:', error);
      void fetchClips();
    });
  }, [fetchClips, setAllClips, setSelectedClip]);

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
        return [...newlyPinned, ...existingPinned, ...updated.filter((clip) => !clip.is_pinned)];
      }
      return [...updated.filter((clip) => clip.is_pinned), ...updated.filter((clip) => !clip.is_pinned)];
    });
    setSelectedClip((previous) => previous && changedIdSet.has(previous.id)
      ? { ...previous, is_pinned: pinState, pin_order: pinState ? 0 : previous.pin_order }
      : previous);

    void invoke('batch_pin_clips', { ids: idsToChange, pinState }).catch((error) => {
      console.error('Failed to set pinned state:', error);
      void fetchClips();
    });
  }, [allClips, fetchClips, selectedClipIds, setAllClips, setSelectedClip]);

  const setProtected = useCallback((id: number, protectedState: boolean) => {
    const targetIds = selectedClipIds.size > 1 && selectedClipIds.has(id)
      ? Array.from(selectedClipIds)
      : [id];
    const idsToChange = allClips
      .filter((clip) => targetIds.includes(clip.id) && Boolean(clip.is_protected) !== protectedState)
      .map((clip) => clip.id);
    if (idsToChange.length === 0) return;
    const changedIdSet = new Set(idsToChange);

    setAllClips((previous) => previous.map((clip) => (
      changedIdSet.has(clip.id) ? { ...clip, is_protected: protectedState } : clip
    )));
    setSelectedClip((previous) => previous && changedIdSet.has(previous.id)
      ? { ...previous, is_protected: protectedState }
      : previous);

    void Promise.all(idsToChange.map((clipId) => invoke('toggle_clip_protected', { clipId })))
      .catch((error) => {
        console.error('Failed to set protected state:', error);
        void fetchClips();
      });
  }, [allClips, fetchClips, selectedClipIds, setAllClips, setSelectedClip]);

  const deleteClipIds = useCallback((requestedIds: number[], forcePermanent = false) => {
    const ids = requestedIds.filter((id) => !allClips.find((clip) => clip.id === id)?.is_protected);
    if (ids.length === 0) return;

    const categoryBinIds = new Set(bins.filter((bin) => bin.bin_type !== 'tag').map((bin) => bin.id));
    const deletedItems = allClips
      .filter((clip) => ids.includes(clip.id))
      .map((clip) => ({
        ...clip,
        is_trashed: true,
        bin_id: null,
        bin_ids: (clip.bin_ids || []).filter((binId) => !categoryBinIds.has(binId)),
      }));
    const deletedSourceClips = allClips.filter((clip) => ids.includes(clip.id));
    setBins((previous) => previous.map((bin) => {
      if (bin.bin_type === 'tag') return bin;
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
    setSelectedClipIds((previous) => new Set(Array.from(previous).filter((id) => !ids.includes(id))));
    setSelectedClip((previous) => previous && ids.includes(previous.id) ? null : previous);
    setTotalClipCount((previous) => Math.max(0, previous - ids.length));

    const request = permanently
      ? Promise.all(ids.map((id) => invoke('purge_clip_permanently', { id })))
      : ids.length > 1
        ? invoke('batch_trash_clips', { ids })
        : invoke('delete_clip', { id: ids[0] });
    void request
      // Moving a clip to Trash is already fully represented in local clip,
      // selection, Bin-count, Trash-count, and total-count state. Refetching
      // both collections on success delayed the visible drop completion.
      // Permanent deletion still reconciles because it may originate from the
      // separately loaded Trash collection.
      .then(() => permanently
        ? Promise.all([fetchClips(), fetchTrashedClips()])
        : undefined)
      .catch((error) => {
        console.error(permanently ? 'Failed to permanently delete clips:' : 'Failed to trash clips:', error);
        void fetchClips();
        void fetchTrashedClips();
      });
  }, [allClips, bins, fetchClips, fetchTrashedClips, setAllClips, setSelectedClip, setSelectedClipIds, setTotalClipCount, setTrashedClips, settings.enableTrash]);

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
      const text = settings.alwaysPastePlainText && clip.text_content
        ? clip.text_content.replace(/<[^>]*>/g, '')
        : clip.text_content;
      await invoke('copy_clip_to_system', {
        text,
        imageBase64: settings.alwaysPastePlainText ? null : clip.image_base64,
      });
      soundManager.playCopySound(settings.enableSounds);
    } catch (error) {
      console.error('Failed to copy clip:', error);
    }
  }, [settings.alwaysPastePlainText, settings.enableSounds]);

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
    const categoryBinIds = new Set(bins.filter((bin) => bin.bin_type !== 'tag').map((bin) => bin.id));

    const updateClip = (clip: ClipItem) => {
      if (!targetIds.includes(clip.id)) return clip;
      const tagIds = (clip.bin_ids || []).filter((id) => !categoryBinIds.has(id));
      return { ...clip, bin_id: binId, bin_ids: binId === null ? tagIds : [...tagIds, binId] };
    };
    setAllClips((previous) => previous.map(updateClip));
    setSelectedClip((previous) => previous ? updateClip(previous) : previous);

    setBins((previous) => previous.map((bin) => {
      if (bin.bin_type === 'tag') return bin;
      let delta = 0;
      for (const clip of targetClips) {
        const oldBinIds = new Set([
          ...(clip.bin_ids || []).filter((id) => categoryBinIds.has(id)),
          ...(clip.bin_id && categoryBinIds.has(clip.bin_id) ? [clip.bin_id] : []),
        ]);
        if (oldBinIds.has(bin.id) && bin.id !== binId) delta -= 1;
        if (bin.id === binId && !oldBinIds.has(bin.id)) delta += 1;
      }
      return delta === 0 ? bin : { ...bin, clip_count: Math.max(0, (bin.clip_count || 0) + delta) };
    }));

    if (options.playSound) {
      requestAnimationFrame(() => soundManager.playCopySound(settings.enableSounds));
    }

    try {
      if (targetIds.length > 1) {
        await invoke('batch_assign_bin_clips', { ids: targetIds, binId });
      } else {
        await invoke('assign_clip_bin', { clipId, binId });
      }
    } catch (error) {
      console.error('Failed to assign clips to bin:', error);
      // The optimistic clip and count updates are authoritative on success.
      // Reconcile the complete data sets only when persistence fails.
      void fetchClips();
      void fetchBins();
    }
  }, [allClips, bins, fetchBins, fetchClips, selectedClipIds, setAllClips, setBins, setSelectedClip, settings.enableSounds]);

  const applyFilterToClip = useCallback(async (clip: ClipItem, filter: FilterRule) => {
    if (!clip.text_content) return;
    try {
      const transformed = await invoke<string>('transform_text', {
        input: clip.text_content,
        filterType: filter.filter_type,
        config: filter.config,
      });
      await invoke('copy_clip_to_system', { text: transformed, imageBase64: null });
      soundManager.playPasteSound(settings.enableSounds);
    } catch (error) {
      console.error('Failed to apply filter:', error);
    }
  }, [settings.enableSounds]);

  const addToSequentialStack = useCallback(async (clip: ClipItem) => {
    const item = clip.text_content || (clip.content_type === 'image' ? '[Image Clip]' : 'Clip item');
    try {
      await invoke('push_sequential_item', { item });
      soundManager.playStackSound(settings.enableSounds);
      void fetchSequentialStatus();
    } catch (error) {
      console.error('Failed to add clip to queue:', error);
    }
  }, [fetchSequentialStatus, settings.enableSounds]);

  const updateClipNoteLocally = useCallback((clipId: number, note: string | null) => {
    setAllClips((previous) => previous.map((clip) => clip.id === clipId ? { ...clip, note } : clip));
    setSelectedClip((previous) => previous?.id === clipId ? { ...previous, note } : previous);
  }, [setAllClips, setSelectedClip]);

  const deleteNoteFromClip = useCallback(async (clipId: number) => {
    updateClipNoteLocally(clipId, null);
    try {
      await invoke('update_clip_note', { clipId, note: null });
    } catch (error) {
      console.error('Failed to delete clip note:', error);
      void fetchClips();
    }
  }, [fetchClips, updateClipNoteLocally]);

  return {
    togglePin,
    toggleProtected,
    setPinned,
    setProtected,
    deleteSelectedClips,
    deleteClip,
    copyClip,
    assignClipToBin,
    applyFilterToClip,
    addToSequentialStack,
    updateClipNoteLocally,
    deleteNoteFromClip,
  };
}
