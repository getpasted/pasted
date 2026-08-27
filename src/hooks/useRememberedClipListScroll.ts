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
  if (position.anchorClipId !== null) {
    const anchor = element.querySelector<HTMLElement>(`[data-clip-id="${position.anchorClipId}"]`);
    if (anchor) {
      element.scrollTop += anchor.getBoundingClientRect().top
        - element.getBoundingClientRect().top - position.anchorOffset;
      return;
    }
  }
  element.scrollTop = position.scrollTop;
}

function reorderCommitInProgress(element: HTMLDivElement) {
  return document.documentElement.classList.contains('is-stable-reordering')
    || Boolean(element.closest('.is-settling-pinned-reorder'));
}

export function useRememberedClipListScroll(
  viewKey: string,
  listRef: RefObject<HTMLDivElement | null>,
) {
  const memoryRef = useRef<ClipListScrollMemory | null>(null);
  const restoreFrameRef = useRef<number | null>(null);
  if (memoryRef.current === null) memoryRef.current = new ClipListScrollMemory();

  useLayoutEffect(() => {
    const element = listRef.current;
    if (!element) return undefined;
    if (!memoryRef.current!.has(viewKey)) {
      const persisted = readPersistedScrollPosition(`clips:${viewKey}`);
      memoryRef.current!.remember(viewKey, {
        scrollTop: persisted.scrollTop,
        anchorClipId: persisted.anchorClipId ?? null,
        anchorOffset: persisted.anchorOffset ?? 0,
      });
    }
    const initialPosition = memoryRef.current!.recall(viewKey);
    let restoringInitialLayout = true;
    const finishInitialRestore = window.setTimeout(() => { restoringInitialLayout = false; }, 500);
    const scheduleRestore = () => {
      if (reorderCommitInProgress(element)) return;
      if (restoreFrameRef.current !== null) cancelAnimationFrame(restoreFrameRef.current);
      restoreFrameRef.current = requestAnimationFrame(() => {
        restoreFrameRef.current = null;
        restorePosition(
          element,
          restoringInitialLayout ? initialPosition : memoryRef.current!.recall(viewKey),
        );
      });
    };
    const mutationObserver = new MutationObserver(scheduleRestore);

    element.scrollTop = initialPosition.scrollTop;
    mutationObserver.observe(element, { childList: true, subtree: true });
    element.addEventListener('load', scheduleRestore, true);
    scheduleRestore();
    return () => {
      window.clearTimeout(finishInitialRestore);
      if (restoreFrameRef.current !== null) cancelAnimationFrame(restoreFrameRef.current);
      mutationObserver.disconnect();
      element.removeEventListener('load', scheduleRestore, true);
    };
  }, [listRef, viewKey]);

  return useCallback((element: HTMLDivElement) => {
    const position = capturePosition(element);
    memoryRef.current!.remember(viewKey, position);
    scheduleScrollPositionPersistence(`clips:${viewKey}`, position);
    scheduleBackupClientStatePersistence(750);
  }, [viewKey]);
}
