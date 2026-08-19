import { useCallback, useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { Bin, ClipCollectionSummary, ClipItem, ManualTransform, SequentialStatus } from '../types';
import { sortClipsForTimeline } from '../utils/clipOrder';
import { soundManager } from '../utils/sound';
import { safeInvoke as invoke } from '../utils/tauri';
import { APP_EVENTS, type ClipboardPauseChangedEvent } from '../utils/appEvents';
import { transformsApi } from '../api/transforms';

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

function normalizeClipItem(value: unknown): ClipItem | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  const source = typeof record.source === 'string'
    ? record.source
    : typeof record.source_app === 'string'
      ? record.source_app
      : 'Unknown';
  const { source_app: _legacySource, ...canonical } = record;
  return { ...canonical, source } as unknown as ClipItem;
}

function normalizeClipItems(value: unknown): ClipItem[] {
  if (!Array.isArray(value)) return [];
  return value.map(normalizeClipItem).filter((clip): clip is ClipItem => clip !== null);
}

function cacheClipSummaries(clips: ClipItem[]) {
  try {
    localStorage.setItem('pasted_cache_clips', JSON.stringify(clips.slice(0, 50)));
  } catch {
    // The database remains authoritative when browser storage is unavailable or full.
  }
}

function isCompleteClipEvent(payload: ClipItem | { id: number }): payload is ClipItem {
  return typeof (payload as Partial<ClipItem>).content_type === 'string'
    && typeof (payload as Partial<ClipItem>).content_hash === 'string'
    && typeof (payload as Partial<ClipItem>).source === 'string'
    && typeof (payload as Partial<ClipItem>).created_at === 'string';
}

function mergeClipSummary(clips: ClipItem[], incoming: ClipItem) {
  const summary = { ...incoming, html_content: null, image_base64: null };
  const existingIndex = clips.findIndex((clip) => clip.id === summary.id);
  const next = existingIndex === -1
    ? [...clips, summary]
    : clips.map((clip, index) => index === existingIndex ? summary : clip);
  return sortClipsForTimeline(next);
}

const CLIP_PAGE_SIZE = 250;
const EMPTY_COLLECTION_SUMMARY: ClipCollectionSummary = {
  activeCount: 0,
  trashCount: 0,
  pinnedCount: 0,
  protectedCount: 0,
  notedCount: 0,
  clipTypeCounts: [],
  fileFormatCounts: [],
  typeCounts: [],
  sourceCounts: [],
};

function appendUniqueClips(current: ClipItem[], incoming: ClipItem[]): ClipItem[] {
  const known = new Set(current.map((clip) => clip.id));
  return [...current, ...incoming.filter((clip) => !known.has(clip.id))];
}

export function useAppData() {
  const [allClips, setAllClips] = useState<ClipItem[]>(() => normalizeClipItems(readCachedArray('pasted_cache_clips')));
  const [trashedClips, setTrashedClips] = useState<ClipItem[]>([]);
  const [bins, setBins] = useState<Bin[]>(() => readCachedArray('pasted_cache_bins'));
  const [pipelines, setPipelines] = useState<ManualTransform[]>([]);
  const [sequentialStatus, setSequentialStatus] = useState<SequentialStatus | null>(null);
  const [totalClipCount, setTotalClipCount] = useState(0);
  const [totalTrashCount, setTotalTrashCount] = useState(0);
  const [clipCollectionSummary, setClipCollectionSummary] = useState<ClipCollectionSummary>(EMPTY_COLLECTION_SUMMARY);
  const [isLoadingMoreClips, setIsLoadingMoreClips] = useState(false);
  const [isLoadingMoreTrash, setIsLoadingMoreTrash] = useState(false);
  const activeOffsetRef = useRef(0);
  const trashOffsetRef = useRef(0);
  const activeLoadingRef = useRef(false);
  const trashLoadingRef = useRef(false);
  const [isClipboardPaused, setIsClipboardPaused] = useState(false);
  const [initialDataLoaded, setInitialDataLoaded] = useState(false);
  const [ignoredAppStatus, setIgnoredAppStatus] = useState<{ app_name: string; timestamp: number } | null>(null);

  const fetchClipCollectionSummary = useCallback(async () => {
    try {
      const summary = await invoke<ClipCollectionSummary>('get_clip_collection_summary');
      setClipCollectionSummary(summary);
      setTotalClipCount(summary.activeCount);
      setTotalTrashCount(summary.trashCount);
    } catch (error) {
      console.error('Failed to fetch clip collection summary:', error);
    }
  }, []);

  const fetchClips = useCallback(async () => {
    try {
      const clips = normalizeClipItems(await invoke<unknown[]>('get_clips', {
        binId: null,
        onlyPinned: false,
        limit: Math.max(CLIP_PAGE_SIZE, activeOffsetRef.current),
        offset: 0,
      }));
      setAllClips(clips);
      activeOffsetRef.current = clips.length;
      void fetchClipCollectionSummary();
      cacheClipSummaries(clips);
    } catch (error) {
      console.error('Failed to fetch clips:', error);
    }
  }, [fetchClipCollectionSummary]);

  const fetchTrashedClips = useCallback(async () => {
    try {
      const clips = normalizeClipItems(await invoke<unknown[]>('get_trashed_clips', {
        limit: Math.max(CLIP_PAGE_SIZE, trashOffsetRef.current),
        offset: 0,
      }));
      setTrashedClips(clips);
      trashOffsetRef.current = clips.length;
      void fetchClipCollectionSummary();
    } catch (error) {
      console.error('Failed to fetch trashed clips:', error);
    }
  }, [fetchClipCollectionSummary]);

  const loadMoreClips = useCallback(async () => {
    if (activeLoadingRef.current || activeOffsetRef.current >= totalClipCount) return;
    activeLoadingRef.current = true;
    setIsLoadingMoreClips(true);
    try {
      const page = normalizeClipItems(await invoke<unknown[]>('get_clips', {
        binId: null,
        onlyPinned: false,
        limit: CLIP_PAGE_SIZE,
        offset: activeOffsetRef.current,
      }));
      activeOffsetRef.current += page.length;
      if (page.length === 0) activeOffsetRef.current = totalClipCount;
      setAllClips((current) => appendUniqueClips(current, page));
    } catch (error) {
      console.error('Failed to load older clips:', error);
    } finally {
      activeLoadingRef.current = false;
      setIsLoadingMoreClips(false);
    }
  }, [totalClipCount]);

  const loadMoreTrashedClips = useCallback(async () => {
    if (trashLoadingRef.current || trashOffsetRef.current >= totalTrashCount) return;
    trashLoadingRef.current = true;
    setIsLoadingMoreTrash(true);
    try {
      const page = normalizeClipItems(await invoke<unknown[]>('get_trashed_clips', {
        limit: CLIP_PAGE_SIZE,
        offset: trashOffsetRef.current,
      }));
      trashOffsetRef.current += page.length;
      if (page.length === 0) trashOffsetRef.current = totalTrashCount;
      setTrashedClips((current) => appendUniqueClips(current, page));
    } catch (error) {
      console.error('Failed to load older trashed clips:', error);
    } finally {
      trashLoadingRef.current = false;
      setIsLoadingMoreTrash(false);
    }
  }, [totalTrashCount]);

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

  const fetchPipelines = useCallback(async () => {
    try {
      setPipelines(await transformsApi.listManual());
    } catch (error) {
      console.error('Failed to fetch manual Transforms:', error);
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
    setTotalTrashCount((previous) => Math.max(0, previous - 1));
    if (restored) {
      const restoredActiveClip: ClipItem = {
        ...restored,
        is_trashed: false,
        trashed_at: null,
        bin_id: null,
      };
      setAllClips((previous) => [restoredActiveClip, ...previous]);
      setTotalClipCount((previous) => previous + 1);
    }
    try {
      await invoke('restore_clip', { id: clipId });
      await fetchClipCollectionSummary();
    } catch (error) {
      console.error('Failed to restore clip:', error);
      void fetchClips();
      void fetchTrashedClips();
    }
  }, [fetchClipCollectionSummary, fetchClips, fetchTrashedClips, trashedClips]);

  const purgeClipPermanently = useCallback(async (clipId: number) => {
    if (trashedClips.find((clip) => clip.id === clipId)?.is_protected) return;
    setTrashedClips((previous) => previous.filter((clip) => clip.id !== clipId));
    setTotalTrashCount((previous) => Math.max(0, previous - 1));
    try {
      await invoke('purge_clip_permanently', { id: clipId });
      await fetchClipCollectionSummary();
    } catch (error) {
      console.error('Failed to permanently delete clip:', error);
      void fetchTrashedClips();
    }
  }, [fetchClipCollectionSummary, fetchTrashedClips, trashedClips]);

  const emptyTrash = useCallback(async () => {
    setTrashedClips((previous) => {
      const retained = previous.filter((clip) => clip.is_protected);
      setTotalTrashCount(retained.length);
      return retained;
    });
    try {
      await invoke('empty_trash');
      await fetchClipCollectionSummary();
    } catch (error) {
      console.error('Failed to empty trash:', error);
      void fetchTrashedClips();
    }
  }, [fetchClipCollectionSummary, fetchTrashedClips]);

  useEffect(() => {
    let cancelled = false;
    void Promise.all([
      fetchClips(),
      fetchBins(),
      fetchPipelines(),
      fetchSequentialStatus(),
      fetchTrashedClips(),
      invoke<boolean>('is_clipboard_paused')
        .then(setIsClipboardPaused)
        .catch((error) => console.error('Failed to read clipboard pause state:', error)),
    ]).finally(() => {
      if (!cancelled) setInitialDataLoaded(true);
    });
    return () => {
      cancelled = true;
    };
  }, [fetchBins, fetchClipCollectionSummary, fetchClips, fetchPipelines, fetchSequentialStatus, fetchTrashedClips]);

  useEffect(() => {
    if (typeof window === 'undefined' || !(window as any).__TAURI_INTERNALS__) return;

    let ignoredStatusTimer: ReturnType<typeof setTimeout> | undefined;
    const unlistenClip = listen<ClipItem | { id: number }>(APP_EVENTS.clipAdded, (event) => {
      const payload = normalizeClipItem(event.payload);
      if (payload && isCompleteClipEvent(payload)) {
        setAllClips((previous) => {
          const next = mergeClipSummary(previous, payload);
          cacheClipSummaries(next);
          return next;
        });
        void fetchClipCollectionSummary();
      } else {
        // OCR currently emits only an ID after updating the stored clip.
        void fetchClips();
      }
      soundManager.playCopySound();
    });
    const unlistenSequential = listen<SequentialStatus>(APP_EVENTS.sequentialUpdated, (event) => {
      setSequentialStatus(event.payload);
    });
    const unlistenBlacklist = listen<{ app_name: string }>(APP_EVENTS.blacklistClipIgnored, (event) => {
      setIgnoredAppStatus({ app_name: event.payload.app_name, timestamp: Date.now() });
      if (ignoredStatusTimer) clearTimeout(ignoredStatusTimer);
      ignoredStatusTimer = setTimeout(() => setIgnoredAppStatus(null), 4000);
    });
    const unlistenPause = listen<ClipboardPauseChangedEvent>(APP_EVENTS.clipboardPauseChanged, (event) => {
      setIsClipboardPaused(event.payload.isPaused);
    });
    const unlistenLibraryChanged = listen(APP_EVENTS.clipLibraryChanged, () => {
      void Promise.all([fetchClips(), fetchTrashedClips()]);
    });
    // Native backends should deliver every clip-added event while Pasted is in
    // the background. Reconcile on focus as a safety net for compositors or
    // webviews that coalesce background delivery.
    const unlistenFocus = listen('tauri://focus', () => {
      void fetchClips();
    });

    return () => {
      if (ignoredStatusTimer) clearTimeout(ignoredStatusTimer);
      void unlistenClip.then((unlisten) => unlisten());
      void unlistenSequential.then((unlisten) => unlisten());
      void unlistenBlacklist.then((unlisten) => unlisten());
      void unlistenPause.then((unlisten) => unlisten());
      void unlistenLibraryChanged.then((unlisten) => unlisten());
      void unlistenFocus.then((unlisten) => unlisten());
    };
  }, [fetchClipCollectionSummary, fetchClips, fetchTrashedClips]);

  return {
    allClips,
    setAllClips,
    trashedClips,
    setTrashedClips,
    bins,
    setBins,
    pipelines,
    sequentialStatus,
    totalClipCount,
    totalTrashCount,
    clipCollectionSummary,
    setTotalClipCount,
    isClipboardPaused,
    ignoredAppStatus,
    initialDataLoaded,
    fetchClips,
    fetchTrashedClips,
    fetchClipCollectionSummary,
    loadMoreClips,
    loadMoreTrashedClips,
    isLoadingMoreClips,
    isLoadingMoreTrash,
    fetchBins,
    fetchPipelines,
    fetchSequentialStatus,
    toggleClipboardPause,
    restoreClip,
    purgeClipPermanently,
    emptyTrash,
  };
}
