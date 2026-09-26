import { useEffect, useLayoutEffect } from 'react';
import { useLocalization } from '../localization/LocalizationProvider';
import { safeInvoke as invoke } from '../utils/tauri';

export interface LibraryStartupStatus {
  ready: boolean;
  path: string | null;
  recoveryCreatedAt: string | null;
}

// Only reached when even the normal application folder cannot be opened (or
// another session currently owns it). There is no recovery decision to make.
export function LibraryStartupWaiting() {
  const { t, catalogReady } = useLocalization();
  useLayoutEffect(() => {
    if (catalogReady) document.getElementById('startup-splash')?.remove();
  }, [catalogReady]);
  useEffect(() => {
    let cancelled = false;
    let timer = 0;
    async function retry() {
      try {
        const status = await invoke<LibraryStartupStatus>('retry_library_startup');
        if (status.ready) return; // Native startup restarts once storage is ready.
      } catch { /* A transient filesystem or session failure retries below. */ }
      if (!cancelled) timer = window.setTimeout(() => void retry(), 5000);
    }
    timer = window.setTimeout(() => void retry(), 5000);
    return () => { cancelled = true; window.clearTimeout(timer); };
  }, []);
  if (!catalogReady) return null;
  return <main className="theme-text-main flex min-h-screen items-center justify-center p-8">
    <div className="max-w-sm text-center" role="status">
      <h1 className="text-base font-medium">{t('libraryRecovery.waitingTitle')}</h1>
      <p className="theme-text-muted mt-2 text-sm leading-relaxed">{t('libraryRecovery.waitingDescription')}</p>
    </div>
  </main>;
}
