import { scheduleBackupClientStatePersistence } from './backupClientState';
import { readPersistedScrollPosition, scheduleScrollPositionPersistence } from './scrollPositionState';

const SCROLL_SURFACE_SELECTOR = '[data-pasted-scroll-key]';

function attachScrollSurface(element: HTMLElement, key: string) {
  const position = readPersistedScrollPosition(key);
  let restoring = true;
  let restoreFrame = 0;

  const remember = () => {
    scheduleScrollPositionPersistence(key, { scrollTop: element.scrollTop });
    scheduleBackupClientStatePersistence(750);
  };
  const finishRestore = () => { restoring = false; };
  const restore = () => {
    restoreFrame = 0;
    if (!restoring) return;
    element.scrollTop = Math.min(position.scrollTop, Math.max(0, element.scrollHeight - element.clientHeight));
    if (element.scrollHeight - element.clientHeight >= position.scrollTop - 1) {
      restoreFrame = requestAnimationFrame(finishRestore);
    }
  };
  const scheduleRestore = () => {
    if (restoreFrame) cancelAnimationFrame(restoreFrame);
    restoreFrame = requestAnimationFrame(restore);
  };
  const handleScroll = () => {
    if (!restoring) remember();
  };
  const interruptRestore = () => {
    if (!restoring) return;
    restoring = false;
    if (restoreFrame) cancelAnimationFrame(restoreFrame);
  };
  const mutationObserver = new MutationObserver(scheduleRestore);
  mutationObserver.observe(element, { childList: true, subtree: true });
  element.addEventListener('scroll', handleScroll, { passive: true });
  element.addEventListener('wheel', interruptRestore, { passive: true });
  element.addEventListener('pointerdown', interruptRestore, { passive: true });
  element.addEventListener('touchstart', interruptRestore, { passive: true });
  element.addEventListener('load', scheduleRestore, true);
  const restoreTimeout = window.setTimeout(finishRestore, 2000);
  scheduleRestore();

  return () => {
    window.clearTimeout(restoreTimeout);
    if (restoreFrame) cancelAnimationFrame(restoreFrame);
    mutationObserver.disconnect();
    element.removeEventListener('scroll', handleScroll);
    element.removeEventListener('wheel', interruptRestore);
    element.removeEventListener('pointerdown', interruptRestore);
    element.removeEventListener('touchstart', interruptRestore);
    element.removeEventListener('load', scheduleRestore, true);
  };
}

export function installPersistedScrollSurfaces() {
  const attached = new Map<HTMLElement, { key: string; cleanup: () => void }>();
  const synchronize = () => {
    for (const element of document.querySelectorAll<HTMLElement>(SCROLL_SURFACE_SELECTOR)) {
      const key = element.dataset.pastedScrollKey;
      const existing = attached.get(element);
      if (!key || existing?.key === key) continue;
      existing?.cleanup();
      attached.set(element, { key, cleanup: attachScrollSurface(element, key) });
    }
    for (const [element, state] of attached) {
      if (element.isConnected && element.dataset.pastedScrollKey === state.key) continue;
      state.cleanup();
      attached.delete(element);
    }
  };
  const observer = new MutationObserver(synchronize);
  observer.observe(document.body, {
    attributes: true,
    attributeFilter: ['data-pasted-scroll-key'],
    childList: true,
    subtree: true,
  });
  synchronize();
  return () => {
    observer.disconnect();
    for (const state of attached.values()) state.cleanup();
    attached.clear();
  };
}
