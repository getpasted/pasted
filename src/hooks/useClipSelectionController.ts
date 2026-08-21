import {
  useCallback,
  useLayoutEffect,
  useRef,
  useState,
  type Dispatch,
  type MouseEvent,
  type SetStateAction,
} from 'react';
import type { ClipItem } from '../types';
import type { AppUiState } from '../utils/appUiState';
import { clipCollectionViewKey, pendingClipFocusId, selectionIdsForContextMenu, type ClipFocusRequest } from '../utils/clipSelection';
import { useClipSelectionKeyboard } from './useClipSelectionKeyboard';

interface UseClipSelectionControllerOptions {
  displayedClips: ClipItem[];
  initialDataLoaded: boolean;
  currentTab: string;
  selectedBinId: number | null;
  restoredUiState: AppUiState;
  selectedClip: ClipItem | null;
  setSelectedClip: Dispatch<SetStateAction<ClipItem | null>>;
  selectedClipIds: Set<number>;
  setSelectedClipIds: Dispatch<SetStateAction<Set<number>>>;
  setIsSidebarCollapsed: Dispatch<SetStateAction<boolean>>;
  copyClip: (clip: ClipItem) => unknown;
  deleteClip: (clipId: number) => unknown;
  purgeClipPermanently: (clipId: number) => unknown;
  focusRequest?: ClipFocusRequest | null;
}

export function useClipSelectionController({
  displayedClips,
  initialDataLoaded,
  currentTab,
  selectedBinId,
  restoredUiState,
  selectedClip,
  setSelectedClip,
  selectedClipIds,
  setSelectedClipIds,
  setIsSidebarCollapsed,
  copyClip,
  deleteClip,
  purgeClipPermanently,
  focusRequest,
}: UseClipSelectionControllerOptions) {
  const [, setSelectedIndex] = useState(-1);
  const selectionViewKey = clipCollectionViewKey(currentTab, selectedBinId);
  const selectedClipByViewRef = useRef(new Map<string, number | null>([
    [
      restoredUiState.currentTab === 'bin'
        ? `bin:${restoredUiState.selectedBinId ?? 'none'}`
        : `section:${restoredUiState.currentTab}`,
      restoredUiState.selectedClipId,
    ],
  ]));
  const activeSelectionViewRef = useRef<string | null>(null);
  const handledFocusRequestIdRef = useRef<number | null>(null);

  const clearClipSelection = useCallback(() => {
    setSelectedClip(null);
    setSelectedClipIds(new Set());
    setSelectedIndex(-1);
  }, [setSelectedClip, setSelectedClipIds]);

  const selectPinnedShelfClip = useCallback((clip: ClipItem) => {
    const index = displayedClips.findIndex((item) => item.id === clip.id);
    setSelectedIndex(index);
    setSelectedClip(clip);
    setSelectedClipIds(new Set([clip.id]));
    selectedClipByViewRef.current.set(selectionViewKey, clip.id);
  }, [displayedClips, selectionViewKey, setSelectedClip, setSelectedClipIds]);

  const selectClipForContextMenu = useCallback((clip: ClipItem) => {
    setSelectedIndex(displayedClips.findIndex((candidate) => candidate.id === clip.id));
    setSelectedClip(clip);
    setSelectedClipIds((previous) => selectionIdsForContextMenu(previous, clip.id));
  }, [displayedClips, setSelectedClip, setSelectedClipIds]);

  useLayoutEffect(() => {
    if (!initialDataLoaded) return;
    const displayedIds = new Set(displayedClips.map((clip) => clip.id));
    const viewChanged = activeSelectionViewRef.current !== selectionViewKey;
    const rememberedId = selectedClipByViewRef.current.get(selectionViewKey);
    activeSelectionViewRef.current = selectionViewKey;

    const selectFallback = () => {
      const fallback = displayedClips[0] ?? null;
      selectedClipByViewRef.current.set(selectionViewKey, fallback?.id ?? null);
      setSelectedClip(fallback);
      setSelectedClipIds(fallback ? new Set([fallback.id]) : new Set());
      setSelectedIndex(fallback ? 0 : -1);
    };

    if (displayedClips.length === 0) {
      selectedClipByViewRef.current.set(selectionViewKey, null);
      setSelectedClip(null);
      setSelectedClipIds(new Set());
      setSelectedIndex(-1);
      return;
    }

    const requestedClipId = pendingClipFocusId(
      focusRequest,
      selectionViewKey,
      handledFocusRequestIdRef.current,
    );
    const requestedClip = requestedClipId === null
      ? undefined
      : displayedClips.find((clip) => clip.id === requestedClipId);
    if (requestedClip) {
      const requestedIndex = displayedClips.findIndex((clip) => clip.id === requestedClip.id);
      handledFocusRequestIdRef.current = focusRequest!.requestId;
      selectedClipByViewRef.current.set(selectionViewKey, requestedClip.id);
      setSelectedClip(requestedClip);
      setSelectedClipIds(new Set([requestedClip.id]));
      setSelectedIndex(requestedIndex);
      return;
    }

    if (viewChanged) {
      const rememberedClip = typeof rememberedId === 'number'
        ? displayedClips.find((clip) => clip.id === rememberedId)
        : null;
      const nextClip = rememberedClip ?? displayedClips[0];
      const nextIndex = displayedClips.findIndex((clip) => clip.id === nextClip.id);
      selectedClipByViewRef.current.set(selectionViewKey, nextClip.id);
      setSelectedClip(nextClip);
      setSelectedClipIds(new Set([nextClip.id]));
      setSelectedIndex(nextIndex);
      return;
    }

    if (selectedClip) {
      const currentIndex = displayedClips.findIndex((clip) => clip.id === selectedClip.id);
      if (currentIndex === -1) {
        selectFallback();
        return;
      }
      const currentClip = displayedClips[currentIndex];
      selectedClipByViewRef.current.set(selectionViewKey, currentClip.id);
      setSelectedClip(currentClip);
      setSelectedIndex(currentIndex);
    } else if (typeof rememberedId === 'number' && !displayedIds.has(rememberedId)) {
      selectFallback();
      return;
    } else {
      selectedClipByViewRef.current.set(selectionViewKey, null);
      setSelectedClipIds(new Set());
      setSelectedIndex(-1);
      return;
    }

    setSelectedClipIds((previous) => {
      const next = new Set(Array.from(previous).filter((id) => displayedIds.has(id)));
      return next.size === previous.size && Array.from(next).every((id) => previous.has(id))
        ? previous
        : next;
    });
  }, [displayedClips, focusRequest, initialDataLoaded, selectedClip?.id, selectionViewKey, setSelectedClip, setSelectedClipIds]);

  useClipSelectionKeyboard({ currentTab, displayedClips, selectedClip, setSelectedClip, setSelectedClipIds, setSelectedIndex, setIsSidebarCollapsed, copyClip, deleteClip, purgeClipPermanently });

  const selectedClipRef = useRef(selectedClip);
  const selectedClipIdsRef = useRef(selectedClipIds);
  const displayedClipsRef = useRef(displayedClips);
  selectedClipRef.current = selectedClip;
  selectedClipIdsRef.current = selectedClipIds;
  displayedClipsRef.current = displayedClips;

  const handleClipSelect = useCallback((clip: ClipItem, event: MouseEvent) => {
    const currentSelectedClip = selectedClipRef.current;
    const currentSelectedClipIds = selectedClipIdsRef.current;
    const currentDisplayedClips = displayedClipsRef.current;
    setSelectedIndex(currentDisplayedClips.findIndex((candidate) => candidate.id === clip.id));

    if (event.metaKey || event.ctrlKey) {
      setSelectedClipIds((previous) => {
        const next = new Set(previous);
        if (next.has(clip.id)) {
          next.delete(clip.id);
          if (currentSelectedClip?.id === clip.id) {
            const remaining = Array.from(next);
            const lastId = remaining[remaining.length - 1];
            setSelectedClip(currentDisplayedClips.find((candidate) => candidate.id === lastId) ?? null);
          }
        } else {
          next.add(clip.id);
          setSelectedClip(clip);
        }
        return next;
      });
      return;
    }

    if (event.shiftKey && currentSelectedClip) {
      const currentIndex = currentDisplayedClips.findIndex((candidate) => candidate.id === clip.id);
      const anchorIndex = currentDisplayedClips.findIndex((candidate) => candidate.id === currentSelectedClip.id);
      if (currentIndex !== -1 && anchorIndex !== -1) {
        const start = Math.min(currentIndex, anchorIndex);
        const end = Math.max(currentIndex, anchorIndex);
        setSelectedClipIds(new Set(currentDisplayedClips.slice(start, end + 1).map((candidate) => candidate.id)));
      }
      return;
    }

    if (currentSelectedClip?.id === clip.id && currentSelectedClipIds.size <= 1) clearClipSelection();
    else {
      setSelectedClip(clip);
      setSelectedClipIds(new Set([clip.id]));
    }
  }, [clearClipSelection, setSelectedClip, setSelectedClipIds]);

  return { clearClipSelection, handleClipSelect, selectClipForContextMenu, selectPinnedShelfClip };
}
