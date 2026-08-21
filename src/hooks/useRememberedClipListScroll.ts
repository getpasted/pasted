import { useCallback, useLayoutEffect, useRef, type RefObject } from 'react';
import {
  ClipListScrollMemory,
  type ClipListScrollPosition,
} from '../utils/clipListScrollMemory';

function capturePosition(element: HTMLDivElement): ClipListScrollPosition {
  const listTop = element.getBoundingClientRect().top;
  const anchor = Array.from(element.querySelectorAll<HTMLElement>('[data-clip-id]'))
    .find((card) => card.getBoundingClientRect().bottom > listTop);
  return {
    scrollTop: element.scrollTop,
    anchorClipId: anchor ? Number(anchor.dataset.clipId) : null,
    anchorOffset: anchor ? anchor.getBoundingClientRect().top - listTop : 0,
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
    const initialPosition = memoryRef.current!.recall(viewKey);
    let restoringInitialLayout = true;
    const finishInitialRestore = window.setTimeout(() => { restoringInitialLayout = false; }, 500);
    const scheduleRestore = () => {
      if (restoreFrameRef.current !== null) cancelAnimationFrame(restoreFrameRef.current);
      restoreFrameRef.current = requestAnimationFrame(() => {
        restoreFrameRef.current = null;
        restorePosition(
          element,
          restoringInitialLayout ? initialPosition : memoryRef.current!.recall(viewKey),
        );
      });
    };
    const resizeObserver = typeof ResizeObserver === 'undefined'
      ? null
      : new ResizeObserver(scheduleRestore);
    const observeCards = () => element.querySelectorAll<HTMLElement>('[data-clip-id]')
      .forEach((card) => resizeObserver?.observe(card));
    const mutationObserver = new MutationObserver(() => {
      observeCards();
      scheduleRestore();
    });

    element.scrollTop = initialPosition.scrollTop;
    observeCards();
    resizeObserver?.observe(element);
    mutationObserver.observe(element, { childList: true, subtree: true });
    element.addEventListener('load', scheduleRestore, true);
    scheduleRestore();
    return () => {
      window.clearTimeout(finishInitialRestore);
      if (restoreFrameRef.current !== null) cancelAnimationFrame(restoreFrameRef.current);
      resizeObserver?.disconnect();
      mutationObserver.disconnect();
      element.removeEventListener('load', scheduleRestore, true);
    };
  }, [listRef, viewKey]);

  return useCallback((element: HTMLDivElement) => {
    memoryRef.current!.remember(viewKey, capturePosition(element));
  }, [viewKey]);
}
