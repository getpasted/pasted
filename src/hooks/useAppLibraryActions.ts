import { useCallback } from 'react';
import { clipsApi } from '../api/clips';
import { safeInvoke as invoke } from '../utils/tauri';

type ClearHistoryMode = 'purge' | 'trash' | null;

interface AppLibraryActionsOptions {
  clearHistoryMode: ClearHistoryMode;
  setClearHistoryMode: (mode: ClearHistoryMode) => void;
  updateClipNameLocally: (clipId: number, name: string | null) => void;
  handlePropertyRemoved: (property: 'name', clipIds: number[]) => void;
  fetchClips: () => Promise<unknown>;
  fetchTrashedClips: () => Promise<unknown>;
  fetchBins: () => Promise<unknown>;
  fetchClipCollectionSummary: () => Promise<unknown>;
}

export function useAppLibraryActions({
  clearHistoryMode,
  setClearHistoryMode,
  updateClipNameLocally,
  handlePropertyRemoved,
  fetchClips,
  fetchTrashedClips,
  fetchBins,
  fetchClipCollectionSummary,
}: AppLibraryActionsOptions) {
  const handleClearClipName = useCallback(async (clipId: number) => {
    updateClipNameLocally(clipId, null);
    handlePropertyRemoved('name', [clipId]);
    try {
      await clipsApi.updateName(clipId, null);
      await fetchClipCollectionSummary();
    } catch (error) {
      console.error(error);
      void fetchClips();
    }
  }, [fetchClipCollectionSummary, fetchClips, handlePropertyRemoved, updateClipNameLocally]);

  const handleClearHistory = async () => {
    if (!clearHistoryMode) return;
    try {
      if (clearHistoryMode === 'purge') await invoke('purge_unpinned_clips');
      else await invoke('trash_unpinned_clips');
      setClearHistoryMode(null);
      await Promise.all([fetchClips(), fetchTrashedClips(), fetchBins(), fetchClipCollectionSummary()]);
    } catch (error) {
      console.error(error);
    }
  };

  const handleRestoreAllTrashedClips = async () => {
    const summary = await clipsApi.restoreAll();
    await Promise.all([fetchClips(), fetchTrashedClips(), fetchBins(), fetchClipCollectionSummary()]);
    return summary.changedCount;
  };

  return { handleClearClipName, handleClearHistory, handleRestoreAllTrashedClips };
}
