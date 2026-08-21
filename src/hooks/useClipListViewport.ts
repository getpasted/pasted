import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import type { ClipItem } from '../types';
import type { ClipCollectionMembership } from '../utils/clipCollections';
import { clipCollectionViewKey, pendingClipFocusId, type ClipFocusRequest } from '../utils/clipSelection';
import { clipCardScrollTop } from '../utils/clipListViewport';
import { useRememberedClipListScroll } from './useRememberedClipListScroll';

interface UseClipListViewportOptions {
  membership?: ClipCollectionMembership;
  currentTab: string;
  selectedBinId: number | null;
  displayedClips: ClipItem[];
  allClips: ClipItem[];
  trashedClips: ClipItem[];
  selectedClip: ClipItem | null;
  pinningEnabled: boolean;
  totalClipCount: number;
  totalTrashCount: number;
  searchTotalCount: number;
  isLoadingMoreClips: boolean;
  isLoadingMoreTrash: boolean;
  isSearching: boolean;
  loadMoreClips: () => Promise<unknown>;
  loadMoreTrashedClips: () => Promise<unknown>;
  loadMoreSearchResults: () => Promise<unknown>;
  focusRequest?: ClipFocusRequest | null;
}

export function useClipListViewport({
  membership,
  currentTab,
  selectedBinId,
  displayedClips,
  allClips,
  trashedClips,
  selectedClip,
  pinningEnabled,
  totalClipCount,
  totalTrashCount,
  searchTotalCount,
  isLoadingMoreClips,
  isLoadingMoreTrash,
  isSearching,
  loadMoreClips,
  loadMoreTrashedClips,
  loadMoreSearchResults,
  focusRequest,
}: UseClipListViewportOptions) {
  const clipListRef = useRef<HTMLDivElement | null>(null);
  const pendingRevealIdRef = useRef<number | null>(null);
  const revealAnimationFrameRef = useRef<number | null>(null);
  const handledFocusRequestIdRef = useRef<number | null>(null);
  const [stackedPinnedClipIds, setStackedPinnedClipIds] = useState<number[]>([]);
  const isBinCollection = membership === 'bin' && selectedBinId !== null;
  const isPinnedCollection = membership === 'pinned';
  const selectionViewKey = clipCollectionViewKey(currentTab, selectedBinId);
  const rememberScroll = useRememberedClipListScroll(selectionViewKey, clipListRef);
  const pinnedShelfClips = useMemo(
    () => pinningEnabled && (membership === 'all' || isPinnedCollection)
      ? displayedClips.filter((clip) => clip.is_pinned)
      : [],
    [displayedClips, isPinnedCollection, membership, pinningEnabled],
  );
  const pinnedShelfSignature = pinnedShelfClips
    .map((clip) => `${clip.id}:${clip.pin_order ?? 0}`)
    .join('|');

  const requestRepositionedClipReveal = useCallback((ids: number[]) => {
    if (membership !== 'all' && membership !== 'bin') return;
    pendingRevealIdRef.current = selectedClip && ids.includes(selectedClip.id)
      ? selectedClip.id
      : ids[0] ?? null;
  }, [membership, selectedClip]);

  useEffect(() => setStackedPinnedClipIds([]), [selectionViewKey]);

  const handleClipListScroll = useCallback((element: HTMLDivElement) => {
    rememberScroll(element);
    if (element.scrollHeight - element.scrollTop - element.clientHeight < 800) {
      if (membership === 'trash') void loadMoreTrashedClips();
      else if (membership === 'search') void loadMoreSearchResults();
      else if (membership !== 'queue') void loadMoreClips();
    }
    if (pinnedShelfClips.length === 0 || (membership !== 'all' && !isPinnedCollection)) {
      setStackedPinnedClipIds([]);
      return;
    }
    const pinnedCards = element.querySelectorAll<HTMLElement>('[data-pinned-clip="true"]');
    if (pinnedCards.length === 0) {
      setStackedPinnedClipIds([]);
      return;
    }
    const listTop = element.getBoundingClientRect().top;
    setStackedPinnedClipIds((previous) => {
      const previousIds = new Set(previous);
      const next = Array.from(pinnedCards).flatMap((card) => {
        const id = Number(card.dataset.clipId);
        if (!Number.isFinite(id)) return [];
        const bottom = card.getBoundingClientRect().bottom;
        return bottom <= listTop + (previousIds.has(id) ? 12 : 0) ? [id] : [];
      });
      return next.length === previous.length && next.every((id, index) => id === previous[index])
        ? previous
        : next;
    });
  }, [isPinnedCollection, loadMoreClips, loadMoreSearchResults, loadMoreTrashedClips, membership, pinnedShelfClips.length, rememberScroll]);

  useLayoutEffect(() => {
    const element = clipListRef.current;
    if (!element) return;
    const needsAnotherBatch = isBinCollection
      ? allClips.length < totalClipCount
      : element.scrollHeight - element.clientHeight < 800;
    if (!needsAnotherBatch) return;
    if (membership === 'trash') {
      if (!isLoadingMoreTrash && trashedClips.length < totalTrashCount) void loadMoreTrashedClips();
    } else if (membership === 'search') {
      if (!isSearching && displayedClips.length < searchTotalCount) void loadMoreSearchResults();
    } else if (membership !== 'queue' && !isLoadingMoreClips && allClips.length < totalClipCount) {
      void loadMoreClips();
    }
  }, [allClips.length, displayedClips.length, isBinCollection, isLoadingMoreClips, isLoadingMoreTrash, isSearching, loadMoreClips, loadMoreSearchResults, loadMoreTrashedClips, membership, searchTotalCount, totalClipCount, totalTrashCount, trashedClips.length]);

  const isLoadingCurrentCollection = membership === 'trash'
    ? isLoadingMoreTrash
    : membership === 'search'
      ? isSearching
      : membership !== 'queue' && isLoadingMoreClips;

  useLayoutEffect(() => {
    const element = clipListRef.current;
    if (!element) return undefined;
    let secondFrame = 0;
    const firstFrame = requestAnimationFrame(() => {
      secondFrame = requestAnimationFrame(() => handleClipListScroll(element));
    });
    return () => {
      cancelAnimationFrame(firstFrame);
      if (secondFrame) cancelAnimationFrame(secondFrame);
    };
  }, [handleClipListScroll, pinnedShelfSignature]);

  useLayoutEffect(() => {
    const requestedFocusId = pendingClipFocusId(focusRequest, selectionViewKey, handledFocusRequestIdRef.current);
    const clipId = requestedFocusId ?? pendingRevealIdRef.current;
    const element = clipListRef.current;
    if (clipId === null || !element) return;
    const card = element.querySelector<HTMLElement>(`[data-clip-id="${clipId}"]`);
    if (!card) return;
    if (requestedFocusId !== null) handledFocusRequestIdRef.current = focusRequest!.requestId;
    else pendingRevealIdRef.current = null;
    const targetScrollTop = () => clipCardScrollTop(element, card);
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      element.scrollTop = targetScrollTop();
      return;
    }
    if (revealAnimationFrameRef.current !== null) cancelAnimationFrame(revealAnimationFrameRef.current);
    const durationMs = 260;
    revealAnimationFrameRef.current = requestAnimationFrame((startedAt) => {
      const startScrollTop = element.scrollTop;
      const distance = targetScrollTop() - startScrollTop;
      if (distance === 0) {
        revealAnimationFrameRef.current = null;
        return;
      }
      const animate = (now: number) => {
        const progress = Math.min((now - startedAt) / durationMs, 1);
        element.scrollTop = startScrollTop + distance * (1 - Math.pow(1 - progress, 3));
        if (progress < 1) revealAnimationFrameRef.current = requestAnimationFrame(animate);
        else {
          element.scrollTop = targetScrollTop();
          revealAnimationFrameRef.current = null;
        }
      };
      animate(startedAt);
    });
  }, [displayedClips, focusRequest, selectionViewKey]);

  useEffect(() => () => {
    if (revealAnimationFrameRef.current !== null) cancelAnimationFrame(revealAnimationFrameRef.current);
  }, []);

  return {
    clipListRef,
    handleClipListScroll,
    isLoadingCurrentCollection,
    pinnedShelfClips,
    requestRepositionedClipReveal,
    stackedPinnedClipIds,
  };
}
