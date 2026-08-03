import { useCallback, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { Bin, ClipItem, FilterRule, SequentialStatus } from '../types';
import { soundManager } from '../utils/sound';
import { safeInvoke as invoke } from '../utils/tauri';

function readCachedArray<T>(key: string): T[] {
  try {
    const value = localStorage.getItem(key);
    if (!value) return [];
    const parsed: unknown = JSON.parse(value);
    return Array.isArray(parsed) ? parsed as T[] : [];
  } catch {
    return [];
  }
}

export function useAppData(enableSounds: boolean) {
  const [allClips, setAllClips] = useState<ClipItem[]>(() => readCachedArray('pasted_cache_clips'));
  const [trashedClips, setTrashedClips] = useState<ClipItem[]>([]);
  const [bins, setBins] = useState<Bin[]>(() => readCachedArray('pasted_cache_bins'));
  const [filters, setFilters] = useState<FilterRule[]>([]);
  const [sequentialStatus, setSequentialStatus] = useState<SequentialStatus | null>(null);
  const [totalClipCount, setTotalClipCount] = useState(0);
  const [isClipboardPaused, setIsClipboardPaused] = useState(false);
  const [ignoredAppStatus, setIgnoredAppStatus] = useState<{ app_name: string; timestamp: number } | null>(null);

  const fetchTotalClipCount = useCallback(async () => {
    try {
      setTotalClipCount(await invoke<number>('get_total_clip_count'));
    } catch (error) {
      console.error('Failed to fetch total count:', error);
    }
  }, []);

  const fetchClips = useCallback(async () => {
    try {
      const clips = await invoke<ClipItem[]>('get_clips', {
        searchQuery: null,
        binId: null,
        onlyPinned: false,
      });
      setAllClips(clips);
      void fetchTotalClipCount();
      try {
        localStorage.setItem('pasted_cache_clips', JSON.stringify(clips.slice(0, 50)));
      } catch {
        // The database remains authoritative when browser storage is unavailable or full.
      }
    } catch (error) {
      console.error('Failed to fetch clips:', error);
    }
  }, [fetchTotalClipCount]);

  const fetchTrashedClips = useCallback(async () => {
    try {
      setTrashedClips(await invoke<ClipItem[]>('get_trashed_clips'));
    } catch (error) {
      console.error('Failed to fetch trashed clips:', error);
    }
  }, []);

  const fetchBins = useCallback(async () => {
    try {
      const nextBins = await invoke<Bin[]>('get_bins');
      setBins(nextBins);
      try {
        localStorage.setItem('pasted_cache_bins', JSON.stringify(nextBins));
      } catch {
        // The database remains authoritative when browser storage is unavailable or full.
      }
    } catch (error) {
      console.error('Failed to fetch bins:', error);
    }
  }, []);

  const fetchFilters = useCallback(async () => {
    try {
      setFilters(await invoke<FilterRule[]>('get_filters'));
    } catch (error) {
      console.error('Failed to fetch filters:', error);
    }
  }, []);

  const fetchSequentialStatus = useCallback(async () => {
    try {
      setSequentialStatus(await invoke<SequentialStatus>('get_sequential_status'));
    } catch (error) {
      console.error('Failed to fetch sequential status:', error);
    }
  }, []);

  const toggleClipboardPause = useCallback(async () => {
    try {
      setIsClipboardPaused(await invoke<boolean>('toggle_clipboard_pause'));
    } catch (error) {
      console.error('Failed to toggle clipboard pause:', error);
    }
  }, []);

  const restoreClip = useCallback(async (clipId: number) => {
    const restored = trashedClips.find((clip) => clip.id === clipId);
    setTrashedClips((previous) => previous.filter((clip) => clip.id !== clipId));
    if (restored) {
      setAllClips((previous) => [restored, ...previous]);
      setTotalClipCount((previous) => previous + 1);
    }
    try {
      await invoke('restore_clip', { id: clipId });
    } catch (error) {
      console.error('Failed to restore clip:', error);
      void fetchClips();
      void fetchTrashedClips();
    }
  }, [fetchClips, fetchTrashedClips, trashedClips]);

  const purgeClipPermanently = useCallback(async (clipId: number) => {
    if (trashedClips.find((clip) => clip.id === clipId)?.is_protected) return;
    setTrashedClips((previous) => previous.filter((clip) => clip.id !== clipId));
    try {
      await invoke('purge_clip_permanently', { id: clipId });
    } catch (error) {
      console.error('Failed to permanently delete clip:', error);
      void fetchTrashedClips();
    }
  }, [fetchTrashedClips, trashedClips]);

  const emptyTrash = useCallback(async () => {
    setTrashedClips((previous) => previous.filter((clip) => clip.is_protected));
    try {
      await invoke('empty_trash');
    } catch (error) {
      console.error('Failed to empty trash:', error);
      void fetchTrashedClips();
    }
  }, [fetchTrashedClips]);

  useEffect(() => {
    void Promise.all([
      fetchClips(),
      fetchBins(),
      fetchFilters(),
      fetchSequentialStatus(),
      fetchTrashedClips(),
      invoke<boolean>('is_clipboard_paused')
        .then(setIsClipboardPaused)
        .catch((error) => console.error('Failed to read clipboard pause state:', error)),
    ]);
  }, [fetchBins, fetchClips, fetchFilters, fetchSequentialStatus, fetchTrashedClips]);

  useEffect(() => {
    if (typeof window === 'undefined' || !(window as any).__TAURI_INTERNALS__) return;

    let ignoredStatusTimer: ReturnType<typeof setTimeout> | undefined;
    const unlistenClip = listen<ClipItem>('clip-added', () => {
      void fetchClips();
      soundManager.playCopySound(enableSounds);
    });
    const unlistenSequential = listen<SequentialStatus>('sequential-updated', (event) => {
      setSequentialStatus(event.payload);
    });
    const unlistenBlacklist = listen<{ app_name: string }>('blacklist-clip-ignored', (event) => {
      setIgnoredAppStatus({ app_name: event.payload.app_name, timestamp: Date.now() });
      if (ignoredStatusTimer) clearTimeout(ignoredStatusTimer);
      ignoredStatusTimer = setTimeout(() => setIgnoredAppStatus(null), 4000);
    });
    const unlistenPause = listen<{ is_paused: boolean }>('clipboard-pause-changed', (event) => {
      setIsClipboardPaused(event.payload.is_paused);
    });

    return () => {
      if (ignoredStatusTimer) clearTimeout(ignoredStatusTimer);
      void unlistenClip.then((unlisten) => unlisten());
      void unlistenSequential.then((unlisten) => unlisten());
      void unlistenBlacklist.then((unlisten) => unlisten());
      void unlistenPause.then((unlisten) => unlisten());
    };
  }, [enableSounds, fetchClips]);

  return {
    allClips,
    setAllClips,
    trashedClips,
    setTrashedClips,
    bins,
    setBins,
    filters,
    sequentialStatus,
    totalClipCount,
    setTotalClipCount,
    isClipboardPaused,
    ignoredAppStatus,
    fetchClips,
    fetchTrashedClips,
    fetchBins,
    fetchFilters,
    fetchSequentialStatus,
    toggleClipboardPause,
    restoreClip,
    purgeClipPermanently,
    emptyTrash,
  };
}
