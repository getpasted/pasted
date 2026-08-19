import { FormEvent, useRef, useState } from 'react';
import { Fingerprint, LockKeyhole, Watch } from 'lucide-react';
import type { AppLockStatus } from '../hooks/useAppLock';
import { ActionButton } from './AppDialogLayout';
import { safeInvoke as invoke } from '../utils/tauri';
import { translate } from '../localization/runtime';
import { appLockAuthErrorMessage } from '../utils/appLockAuth';

export function AppLockScreen({
  status,
  onUnlockWithPassphrase,
  onUnlockWithSystemAuth,
  onUnlockWithAppleWatch,
  unlocking = false,
}: {
  status: AppLockStatus;
  onUnlockWithPassphrase: (passphrase: string) => Promise<unknown>;
  onUnlockWithSystemAuth: () => Promise<unknown>;
  onUnlockWithAppleWatch: () => Promise<unknown>;
  unlocking?: boolean;
}) {
  const [passphrase, setPassphrase] = useState('');
  const [error, setError] = useState('');
  const [pending, setPending] = useState(false);
  const authenticationInFlight = useRef(false);
  const passphraseInput = useRef<HTMLInputElement>(null);
  const showSystemAuth = status.systemAuthEnabled;
  // Availability is a live reachability signal: a configured Watch may be
  // temporarily locked or out of range. Keep the action available for retry.
  const showAppleWatch = status.appleWatchEnabled;

  const run = async (action: () => Promise<unknown>) => {
    if (authenticationInFlight.current) return;
    authenticationInFlight.current = true;
    setPending(true);
    setError('');
    try {
      await action();
      setPassphrase('');
    } catch (cause) {
      const detail = appLockAuthErrorMessage(cause);
      // Canceling an operating-system authentication prompt is a normal way
      // to return to the passphrase form, not an error that needs repeating.
      if (detail !== 'Authentication canceled.') setError(detail);
    } finally {
      authenticationInFlight.current = false;
      setPending(false);
    }
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (passphrase) void run(() => onUnlockWithPassphrase(passphrase));
  };

  return (
    <main className={`app-lock-screen theme-app fixed inset-0 z-[10000] flex items-center justify-center p-6 font-sans select-none ${unlocking ? 'is-unlocking' : ''}`}>
      <form onSubmit={submit} aria-busy={pending} className="theme-panel w-full max-w-sm space-y-5 rounded-2xl border p-6 shadow-2xl">
        <div className="space-y-2 text-center">
          <span className={`app-lock-mark theme-surface mx-auto flex h-12 w-12 items-center justify-center rounded-xl border ${pending ? 'is-authenticating' : ''}`}>
            <LockKeyhole className="theme-text-main h-5 w-5" />
          </span>
          <h1 className="theme-title text-lg font-bold">{translate('component.appLockScreen.pastedIsLocked')}</h1>
          <p className="theme-text-muted text-xs" role="status">
            {status.captureWhileLocked
              ? translate('component.appLockScreen.newClipsAreStillBeingCaptured')
              : translate('component.appLockScreen.newClipsAreNotBeingCapturedWhileLocked')}
          </p>
        </div>
        <div className={showSystemAuth || showAppleWatch ? 'grid grid-cols-[minmax(0,1fr)_auto] gap-x-2 gap-y-1.5' : 'space-y-1.5'}>
          <label htmlFor="app-lock-passphrase" className="theme-text-main block text-xs font-semibold">{translate('component.appLockScreen.passphrase')}</label>
          {(showSystemAuth || showAppleWatch) && <span className="theme-text-main block text-xs font-semibold">{translate('component.appLockScreen.bio')}</span>}
          <input
            id="app-lock-passphrase"
            ref={passphraseInput}
            autoFocus
            type="password"
            autoComplete="current-password"
            value={passphrase}
            onChange={(event) => setPassphrase(event.target.value)}
            className="theme-input min-w-0 w-full rounded-lg border px-3 py-2.5 text-sm"
          />
          {(showSystemAuth || showAppleWatch) && (
            <div className="app-lock-auth-group flex shrink-0" role="group" aria-label={translate('component.appLockScreen.biometricUnlockMethods')}>
              {showAppleWatch && (
                <ActionButton
                  aria-label={translate('component.appLockScreen.unlockWithAppleWatch')}
                  title={translate('component.appLockScreen.unlockWithAppleWatch')}
                  className="app-lock-auth-button"
                  disabled={pending}
                  onClick={() => void run(onUnlockWithAppleWatch)}
                >
                  <Watch className="app-lock-watch-icon" />
                </ActionButton>
              )}
              {showSystemAuth && (
                <ActionButton
                  aria-label={translate('component.appLockScreen.unlockWithSystemauthlabel', { systemAuthLabel: status.systemAuthLabel })}
                  title={translate('component.appLockScreen.unlockWithSystemauthlabel', { systemAuthLabel: status.systemAuthLabel })}
                  className="app-lock-auth-button"
                  disabled={pending}
                  onClick={() => void run(onUnlockWithSystemAuth)}
                >
                  <Fingerprint className="app-lock-fingerprint-icon" />
                </ActionButton>
              )}
            </div>
          )}
        </div>
        {error && <p role="alert" className="theme-danger-text text-xs">{error}</p>}
        <div className="flex items-center justify-between gap-3">
          <ActionButton disabled={pending} onClick={() => void run(() => invoke('quit_app'))}>{translate('component.appLockScreen.quit')}</ActionButton>
          <ActionButton type="submit" variant="primary" disabled={pending || !passphrase}>
            {pending ? translate('component.appLockScreen.unlocking') : translate('component.appLockScreen.unlock')}
          </ActionButton>
        </div>
      </form>
    </main>
  );
}
