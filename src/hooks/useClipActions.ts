import { useCallback, useState, type Dispatch, type SetStateAction } from 'react';
import { type AppSettings, type Bin, type ClipItem, type ManualTransform, type SavedTransform } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { clipsApi } from '../api/clips';
import { soundManager } from '../utils/sound';
import { runTransformation } from '../utils/transformExecution';
import { htmlToPlainText } from '../utils/plainText';
import type { ClipPropertyAssociationId } from '../utils/clipPropertyAssociations';
import { useClipPropertyActions } from './useClipPropertyActions';
import { useClipBinActions } from './useClipBinActions';

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
  onClipPropertyRemoved?: (association: ClipPropertyAssociationId, ids: number[]) => void;
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
  onClipPropertyRemoved,
}: ClipActionsInput) {
  const [transformingClipIds, setTransformingClipIds] = useState<Set<number>>(() => new Set());
  const [transformErrorsByClipId, setTransformErrorsByClipId] = useState<Map<number, string>>(() => new Map());

  const {
    togglePin,
    toggleProtected,
    toggleConcealed,
    setPinned,
    setProtected,
    setConcealed,
  } = useClipPropertyActions({
    allClips,
    setAllClips,
    setSelectedClip,
    selectedClipIds,
    fetchClips,
    onCollectionChanged,
    onClipsRepositioned,
    onClipPropertyRemoved,
  });
  const { assignClipToBin, removeClipFromBin } = useClipBinActions({
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
  });

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

  const updateClipNameLocally = useCallback((clipId: number, name: string | null) => {
    setAllClips((previous) => previous.map((clip) => clip.id === clipId ? { ...clip, name } : clip));
    setSelectedClip((previous) => previous?.id === clipId ? { ...previous, name } : previous);
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
    toggleConcealed,
    setPinned,
    setProtected,
    setConcealed,
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
    updateClipNameLocally,
    deleteNoteFromClip,
    transformingClipIds,
    transformErrorsByClipId,
  };
}
