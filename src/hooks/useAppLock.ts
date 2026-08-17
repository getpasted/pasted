import { useCallback, useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { safeInvoke as invoke } from '../utils/tauri';

export interface AppLockStatus {
  enabled: boolean;
  locked: boolean;
  systemAuthEnabled: boolean;
  systemAuthAvailable: boolean;
  systemAuthLabel: string;
  appleWatchEnabled: boolean;
  appleWatchAvailable: boolean;
  idleMinutes: number;
  lockOnSleep: boolean;
  lockOnRestart: boolean;
  captureWhileLocked: boolean;
}

const DEFAULT_STATUS: AppLockStatus = {
  enabled: false,
  locked: false,
  systemAuthEnabled: false,
  systemAuthAvailable: false,
  systemAuthLabel: 'System authentication',
  appleWatchEnabled: false,
  appleWatchAvailable: false,
  idleMinutes: 5,
  lockOnSleep: true,
  lockOnRestart: true,
  captureWhileLocked: true,
};

let cachedStatus = DEFAULT_STATUS;
let cachedHydrated = false;
const statusSubscribers = new Set<(status: AppLockStatus) => void>();

function publishStatus(next: AppLockStatus) {
  cachedStatus = next;
  statusSubscribers.forEach((subscriber) => subscriber(next));
}

async function waitForUnlockAnimation() {
  const duration = window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 180 : 520;
  await new Promise<void>((resolve) => {
    window.requestAnimationFrame(() => {
      window.requestAnimationFrame(() => {
        const screen = document.querySelector<HTMLElement>('.app-lock-screen.is-unlocking');
        if (!screen) {
          resolve();
          return;
        }
        let settled = false;
        const finish = () => {
          if (settled) return;
          settled = true;
          screen.removeEventListener('animationend', onAnimationEnd);
          window.clearTimeout(timeout);
          resolve();
        };
        const onAnimationEnd = (event: AnimationEvent) => {
          if (event.target === screen) finish();
        };
        const timeout = window.setTimeout(finish, duration + 100);
        screen.addEventListener('animationend', onAnimationEnd);
      });
    });
  });
}

function waitForAppContentReady() {
  if (document.documentElement.dataset.pastedContentReady === 'true') {
    return Promise.resolve();
  }
  return new Promise<void>((resolve) => {
    let settled = false;
    const finish = () => {
      if (settled) return;
      settled = true;
      window.removeEventListener('pasted-app-content-ready', finish);
      window.clearTimeout(timeout);
      resolve();
    };
    const timeout = window.setTimeout(finish, 2000);
    window.addEventListener('pasted-app-content-ready', finish, { once: true });
  });
}

export function useAppLock() {
  const [status, setStatus] = useState(cachedStatus);
  const [hydrated, setHydrated] = useState(cachedHydrated);
  const [unlockingSuccess, setUnlockingSuccess] = useState(false);
  const [unlockAnimationActive, setUnlockAnimationActive] = useState(false);
  const statusRef = useRef(status);
  const unlockTransitionRef = useRef<Promise<void> | null>(null);
  statusRef.current = status;

  useEffect(() => {
    const update = (next: AppLockStatus) => setStatus(next);
    statusSubscribers.add(update);
    return () => {
      statusSubscribers.delete(update);
    };
  }, []);

  const acceptStatus = useCallback((next: AppLockStatus) => {
    publishStatus(next);
  }, []);

  const transitionToUnlocked = useCallback((next: AppLockStatus) => {
    if (unlockTransitionRef.current) return unlockTransitionRef.current;
    const transition = (async () => {
      const contentReady = waitForAppContentReady();
      setUnlockingSuccess(true);
      await contentReady;
      setUnlockAnimationActive(true);
      await waitForUnlockAnimation();
      acceptStatus(next);
      setUnlockAnimationActive(false);
      setUnlockingSuccess(false);
    })();
    unlockTransitionRef.current = transition.finally(() => {
      unlockTransitionRef.current = null;
    });
    return unlockTransitionRef.current;
  }, [acceptStatus]);

  const refresh = useCallback(async () => {
    const next = await invoke<AppLockStatus>('get_app_lock_status');
    if (next) acceptStatus(next);
    cachedHydrated = true;
    setHydrated(true);
    return next ?? statusRef.current;
  }, [acceptStatus]);

  useEffect(() => {
    void refresh().catch((error) => {
      console.error('Could not read app-lock status:', error);
      cachedHydrated = true;
      setHydrated(true);
    });
    const refreshOnFocus = () => void refresh().catch((error) => {
      console.error('Could not refresh app-lock status:', error);
    });
    window.addEventListener('focus', refreshOnFocus);
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<AppLockStatus>('app-lock-changed', ({ payload }) => {
      if (!disposed && payload) {
        if (statusRef.current.locked && !payload.locked) void transitionToUnlocked(payload);
        else acceptStatus(payload);
      }
    }).then((dispose) => {
      if (disposed) dispose();
      else unlisten = dispose;
    }).catch(console.error);
    return () => {
      disposed = true;
      unlisten?.();
      window.removeEventListener('focus', refreshOnFocus);
    };
  }, [acceptStatus, refresh, transitionToUnlocked]);

  const lock = useCallback(async () => {
    const next = await invoke<AppLockStatus>('lock_app');
    if (next) acceptStatus(next);
  }, [acceptStatus]);

  useEffect(() => {
    if (!hydrated || !status.enabled || status.locked || status.idleMinutes === 0) return undefined;
    let timer = 0;
    const reset = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => void lock().catch(console.error), status.idleMinutes * 60_000);
    };
    const events: Array<keyof WindowEventMap> = ['pointerdown', 'keydown', 'wheel', 'touchstart'];
    events.forEach((event) => window.addEventListener(event, reset, { passive: true, capture: true }));
    reset();
    return () => {
      window.clearTimeout(timer);
      events.forEach((event) => window.removeEventListener(event, reset, true));
    };
  }, [hydrated, lock, status.enabled, status.idleMinutes, status.locked]);

  const apply = useCallback(async (request: Promise<AppLockStatus>) => {
    const next = await request;
    if (next) acceptStatus(next);
    return next ?? statusRef.current;
  }, [acceptStatus]);

  const unlock = useCallback(async (request: Promise<AppLockStatus>) => {
    const next = await request;
    if (next) await transitionToUnlocked(next);
    return next ?? statusRef.current;
  }, [transitionToUnlocked]);

  return {
    status,
    hydrated,
    unlockingSuccess,
    unlockAnimationActive,
    refresh,
    lock,
    configure: (passphrase: string, currentPassphrase?: string) => apply(invoke<AppLockStatus>('configure_app_lock', { passphrase, currentPassphrase })),
    disable: (passphrase: string) => apply(invoke<AppLockStatus>('disable_app_lock', { passphrase })),
    unlockWithPassphrase: (passphrase: string) => unlock(invoke<AppLockStatus>('unlock_app', { passphrase, authMethod: null })),
    unlockWithSystemAuth: () => unlock(invoke<AppLockStatus>('unlock_app', { passphrase: null, authMethod: 'system' })),
    unlockWithAppleWatch: () => unlock(invoke<AppLockStatus>('unlock_app', { passphrase: null, authMethod: 'apple_watch' })),
    setSystemAuth: (enabled: boolean) => apply(invoke<AppLockStatus>('set_app_lock_system_auth', { enabled })),
    setAppleWatch: (enabled: boolean) => apply(invoke<AppLockStatus>('set_app_lock_apple_watch', { enabled })),
    setIdleMinutes: (minutes: number) => apply(invoke<AppLockStatus>('set_app_lock_idle_minutes', { minutes })),
    setLockOnSleep: (enabled: boolean) => apply(invoke<AppLockStatus>('set_app_lock_lock_on_sleep', { enabled })),
    setLockOnRestart: (enabled: boolean) => apply(invoke<AppLockStatus>('set_app_lock_lock_on_restart', { enabled })),
    setCaptureWhileLocked: (enabled: boolean) => apply(invoke<AppLockStatus>('set_app_lock_capture_while_locked', { enabled })),
  };
}
