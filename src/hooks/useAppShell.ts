import { useEffect, useRef, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { consumePendingBackupClientState } from '../utils/backupClientState';
import { dismissStartupSplash } from '../utils/startupSplash';
import { safeInvoke as invoke } from '../utils/tauri';

const TRANSIENT_SCROLL_SURFACE_SELECTOR = [
  '.surface-scroll-region',
  '.theme-menu',
  '.theme-panel',
  '.theme-surface',
  '.theme-card-idle',
  '.theme-code-surface',
  '.app-dialog-panel',
  '.settings-panel',
  '.tools-scroll-region',
  '.overlay-scroll-region',
  '.custom-scrollbar',
].join(', ');

interface UseAppShellOptions {
  catalogReady: boolean;
  direction: 'ltr' | 'rtl';
  settingsHydrated: boolean;
  initialDataLoaded: boolean;
}

export function useAppShell({
  catalogReady,
  direction,
  settingsHydrated,
  initialDataLoaded,
}: UseAppShellOptions) {
  const previousTitlebarDirectionRef = useRef(direction);
  const [isHudView, setIsHudView] = useState(false);

  useEffect(() => {
    void consumePendingBackupClientState()
      .then((restored) => {
        if (restored) window.location.reload();
      })
      .catch((error) => console.error('Failed to restore backed-up interface state:', error));
  }, []);

  useEffect(() => {
    if (document.documentElement.dataset.platform !== 'macos') return undefined;
    const previousDirection = previousTitlebarDirectionRef.current;
    previousTitlebarDirectionRef.current = direction;
    if (direction === 'ltr' && previousDirection !== 'rtl') return undefined;
    void invoke('set_titlebar_direction', { rtl: direction === 'rtl' })
      .catch((error) => console.error('Failed to update titlebar direction:', error));
    return undefined;
  }, [direction]);

  useEffect(() => {
    const hideTimers = new Map<HTMLElement, number>();
    const markSurfaceScrolling = (target: HTMLElement) => {
      target.classList.add('is-scrolling');
      const previousTimer = hideTimers.get(target);
      if (previousTimer) window.clearTimeout(previousTimer);
      hideTimers.set(target, window.setTimeout(() => {
        target.classList.remove('is-scrolling');
        hideTimers.delete(target);
      }, 700));
    };
    const findScrollSurface = (event: Event) => event.composedPath().find(
      (candidate): candidate is HTMLElement => candidate instanceof HTMLElement
        && candidate.matches(TRANSIENT_SCROLL_SURFACE_SELECTOR),
    );
    const handleSurfaceScroll = (event: Event) => {
      const target = findScrollSurface(event);
      if (target) markSurfaceScrolling(target);
    };
    const handleSurfaceWheel = (event: WheelEvent) => {
      const target = findScrollSurface(event);
      if (target && target.scrollHeight > target.clientHeight) markSurfaceScrolling(target);
    };

    document.addEventListener('scroll', handleSurfaceScroll, true);
    document.addEventListener('wheel', handleSurfaceWheel, { capture: true, passive: true });
    return () => {
      document.removeEventListener('scroll', handleSurfaceScroll, true);
      document.removeEventListener('wheel', handleSurfaceWheel, true);
      hideTimers.forEach((timer) => window.clearTimeout(timer));
    };
  }, []);

  useEffect(() => {
    const enableHudMode = () => {
      setIsHudView(true);
      document.documentElement.classList.add('hud-mode');
      document.body.classList.add('hud-mode');
      document.getElementById('root')?.classList.add('hud-mode');
    };

    try {
      const win = getCurrentWindow();
      if (win.label === 'hud' || window.location.search.includes('view=hud')) enableHudMode();
    } catch {
      if (window.location.search.includes('view=hud')) enableHudMode();
    }
  }, []);

  useEffect(() => {
    const splash = document.getElementById('startup-splash');
    if (!splash) return;
    if (isHudView && catalogReady) {
      splash.remove();
      return;
    }
    if (!catalogReady || !settingsHydrated || !initialDataLoaded) return;
    return dismissStartupSplash(splash);
  }, [catalogReady, initialDataLoaded, isHudView, settingsHydrated]);

  useEffect(() => {
    if (!catalogReady || !settingsHydrated || !initialDataLoaded) return undefined;
    document.documentElement.dataset.pastedContentReady = 'true';
    window.dispatchEvent(new Event('pasted-app-content-ready'));
    return () => {
      delete document.documentElement.dataset.pastedContentReady;
    };
  }, [catalogReady, initialDataLoaded, settingsHydrated]);

  return { isHudView };
}
