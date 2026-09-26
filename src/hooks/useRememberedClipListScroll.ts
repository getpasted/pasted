import { useCallback, useLayoutEffect, useRef, type RefObject } from 'react';
import {
  ClipListScrollMemory,
  type ClipListScrollPosition,
} from '../utils/clipListScrollMemory';
import { scheduleBackupClientStatePersistence } from '../utils/backupClientState';
import { readPersistedScrollPosition, scheduleScrollPositionPersistence } from '../utils/scrollPositionState';

function capturePosition(element: HTMLDivElement): ClipListScrollPosition {
  const listRect = element.getBoundingClientRect();
  const x = listRect.left + listRect.width / 2;
  let anchor: HTMLElement | null = null;
  for (let y = listRect.top + 4; y < Math.min(listRect.bottom, listRect.top + 164); y += 16) {
    const candidate = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-clip-id]') ?? null;
    if (candidate && element.contains(candidate)) {
      anchor = candidate;
      break;
    }
  }
  if (!anchor) {
    for (const card of element.querySelectorAll<HTMLElement>('[data-clip-id]')) {
      if (card.getBoundingClientRect().bottom > listRect.top) {
        anchor = card;
        break;
      }
    }
  }
  return {
    scrollTop: element.scrollTop,
    anchorClipId: anchor ? Number(anchor.dataset.clipId) : null,
    anchorOffset: anchor ? anchor.getBoundingClientRect().top - listRect.top : 0,
  };
}

function restorePosition(element: HTMLDivElement, position: ClipListScrollPosition) {
  const maxScrollTop = element.scrollHeight - element.clientHeight + 1;
  if (position.anchorClipId !== null) {
    const anchor = element.querySelector<HTMLElement>(`[data-clip-id="${position.anchorClipId}"]`);
    if (anchor) {
      element.scrollTop += anchor.getBoundingClientRect().top
        - element.getBoundingClientRect().top - position.anchorOffset;
      return true;
    }
    if (position.scrollTop <= maxScrollTop) element.scrollTop = position.scrollTop;
    return false;
  }
  if (position.scrollTop > maxScrollTop) return false;
  element.scrollTop = position.scrollTop;
  return true;
}

function reorderCommitInProgress(element: HTMLDivElement) {
  return document.documentElement.classList.contains('is-stable-reordering')
    || Boolean(element.closest('.is-settling-pinned-reorder'));
}

export function useRememberedClipListScroll(
  viewKey: string,
  listRef: RefObject<HTMLDivElement | null>,
  ready: boolean,
) {
  const memoryRef = useRef<ClipListScrollMemory | null>(null);
  const restoreFrameRef = useRef<number | null>(null);
  const restoringRef = useRef(false);
  const transitionRef = useRef<{
    key: string;
    position: ClipListScrollPosition;
    complete: boolean;
  } | null>(null);
  if (memoryRef.current === null) memoryRef.current = new ClipListScrollMemory();

  useLayoutEffect(() => {
    const element = listRef.current;
    if (!element) return undefined;
    if (transitionRef.current?.key !== viewKey) {
      if (!memoryRef.current!.has(viewKey)) {
        const persisted = readPersistedScrollPosition(`clips:${viewKey}`);
        memoryRef.current!.remember(viewKey, {
          scrollTop: persisted.scrollTop,
          anchorClipId: persisted.anchorClipId ?? null,
          anchorOffset: persisted.anchorOffset ?? 0,
        });
      }
      transitionRef.current = {
        key: viewKey,
        position: memoryRef.current!.recall(viewKey),
        complete: false,
      };
    }
    const transition = transitionRef.current;
    if (transition.complete) return undefined;
    let attempts = 0;
    restoringRef.current = true;
    element.style.visibility = 'hidden';
    const reveal = () => {
      transition.complete = true;
      restoringRef.current = false;
      element.style.visibility = '';
    };
    const restore = () => {
      restoreFrameRef.current = null;
      if (!ready) return;
      if (reorderCommitInProgress(element)) {
        restoreFrameRef.current = requestAnimationFrame(restore);
        return;
      }
      attempts += 1;
      if (restorePosition(element, transition.position) || attempts >= 12) {
        restoreFrameRef.current = requestAnimationFrame(reveal);
        return;
      }
      restoreFrameRef.current = requestAnimationFrame(restore);
    };
    restoreFrameRef.current = requestAnimationFrame(restore);
    const fallback = window.setTimeout(reveal, 500);
    return () => {
      window.clearTimeout(fallback);
      if (restoreFrameRef.current !== null) cancelAnimationFrame(restoreFrameRef.current);
      restoringRef.current = false;
      element.style.visibility = '';
    };
  }, [listRef, ready, viewKey]);

  return useCallback((element: HTMLDivElement) => {
    if (restoringRef.current) return;
    const position = capturePosition(element);
    memoryRef.current!.remember(viewKey, position);
    scheduleScrollPositionPersistence(`clips:${viewKey}`, position);
    scheduleBackupClientStatePersistence(750);
  }, [viewKey]);
}
