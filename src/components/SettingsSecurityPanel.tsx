import { FormEvent, useState } from 'react';
import { LockKeyhole } from 'lucide-react';
import { useAppLock } from '../hooks/useAppLock';
import { AppDialog } from './AppDialog';
import { ActionButton, AppDialogBody, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';
import { MenuSelect } from './MenuSelect';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
import { SettingsSwitch } from './SettingsSwitch';

const IDLE_OPTIONS = [
  { value: '0', label: 'Never' },
  { value: '1', label: '1 minute' },
  { value: '5', label: '5 minutes' },
  { value: '60', label: '1 hour' },
  { value: '480', label: '8 hours' },
];

function message(error: unknown) {
  return String(error).replace(/^Error:\s*/, '');
}

export function SettingsSecurityPanel() {
  const appLock = useAppLock();
  const [currentPassphrase, setCurrentPassphrase] = useState('');
  const [passphrase, setPassphrase] = useState('');
  const [confirmation, setConfirmation] = useState('');
  const [error, setError] = useState('');
  const [pending, setPending] = useState(false);
  const [credentialMode, setCredentialMode] = useState<'configure' | 'disable' | null>(null);
  const isMac = document.documentElement.dataset.platform === 'macos';

  const resetCredentials = () => {
    setCurrentPassphrase('');
    setPassphrase('');
    setConfirmation('');
    setError('');
  };

  const openCredentials = (mode: 'configure' | 'disable') => {
    resetCredentials();
    setCredentialMode(mode);
  };

  const closeCredentials = () => {
    if (pending) return;
    setCredentialMode(null);
    resetCredentials();
  };

  const configure = async (event: FormEvent) => {
    event.preventDefault();
    setError('');
    if (passphrase !== confirmation) {
      setError('The new passphrases do not match.');
      return;
    }
    setPending(true);
    try {
      await appLock.configure(passphrase, appLock.status.enabled ? currentPassphrase : undefined);
      setCredentialMode(null);
      resetCredentials();
    } catch (cause) {
      setError(message(cause));
    } finally {
      setPending(false);
    }
  };

  const disable = async (event: FormEvent) => {
    event.preventDefault();
    setPending(true);
    setError('');
    try {
      await appLock.disable(currentPassphrase);
      setCredentialMode(null);
      resetCredentials();
    } catch (cause) {
      setError(message(cause));
    } finally {
      setPending(false);
    }
  };

  return (
    <div className="space-y-5 text-xs">
      <SettingsPanelHeader
        icon={LockKeyhole}
        title="Security"
        description="Manage access and authentication."
        actions={appLock.status.enabled ? <ActionButton onClick={() => void appLock.lock()}><LockKeyhole className="h-3.5 w-3.5" /> Lock now</ActionButton> : undefined}
      />

      <div className="space-y-3">
        <SettingsSubsectionHeader
          title="App lock"
          description="Require authentication to interact with Pasted."
        />
        <div className="flex justify-end gap-2">
          {appLock.status.enabled && <ActionButton variant="danger" disabled={pending} onClick={() => openCredentials('disable')}>Disable app lock…</ActionButton>}
          <ActionButton variant="primary" disabled={pending} onClick={() => openCredentials('configure')}>
            {appLock.status.enabled ? 'Change passphrase…' : 'Enable app lock…'}
          </ActionButton>
        </div>
        {error && !credentialMode && <p role="alert" className="theme-danger-text">{error}</p>}
      </div>

      <div className="theme-divider border-t" />
      <div className="space-y-4">
        <SettingsSubsectionHeader title="Unlock" description="Biometric data stays with the operating system." />
          <div className={`flex items-start justify-between gap-4 ${!appLock.status.enabled || !appLock.status.systemAuthAvailable ? 'settings-disabled-row' : ''}`}>
            <div className="min-w-0 flex-1">
              <span className="theme-text-main block font-semibold">Unlock using {appLock.status.systemAuthLabel}</span>
              <p className="theme-text-muted mt-0.5 text-[11px] leading-normal">{appLock.status.systemAuthAvailable ? 'The operating system reports only whether authentication succeeded.' : 'Not available on this device or desktop session.'}</p>
            </div>
            <SettingsSwitch
              checked={appLock.status.systemAuthEnabled}
              disabled={pending || !appLock.status.enabled || !appLock.status.systemAuthAvailable}
              label={`unlock using ${appLock.status.systemAuthLabel}`}
              onClick={() => {
                setPending(true);
                setError('');
                void appLock.setSystemAuth(!appLock.status.systemAuthEnabled).catch((cause) => setError(message(cause))).finally(() => setPending(false));
              }}
            />
          </div>
          {isMac && <div className={`flex items-start justify-between gap-4 ${!appLock.status.enabled || !appLock.status.appleWatchAvailable ? 'settings-disabled-row' : ''}`}>
            <div className="min-w-0 flex-1">
              <span className="theme-text-main block font-semibold">Unlock using Apple Watch</span>
              <p className="theme-text-muted mt-0.5 text-[11px] leading-normal">{appLock.status.appleWatchAvailable ? 'Approve unlock from a nearby paired Apple Watch.' : 'Not available without a compatible paired Apple Watch.'}</p>
            </div>
            <SettingsSwitch
              checked={appLock.status.appleWatchEnabled}
              disabled={pending || !appLock.status.enabled || !appLock.status.appleWatchAvailable}
              label="unlock using Apple Watch"
              onClick={() => {
                setPending(true);
                setError('');
                void appLock.setAppleWatch(!appLock.status.appleWatchEnabled).catch((cause) => setError(message(cause))).finally(() => setPending(false));
              }}
            />
          </div>}
      </div>
      <div className="theme-divider border-t" />
      <div className="space-y-4">
          <SettingsSubsectionHeader title="Auto-lock" description="Choose when authentication is required again." />
          <div className={`flex items-start justify-between gap-4 ${!appLock.status.enabled ? 'settings-disabled-row' : ''}`}>
            <div className="min-w-0 flex-1">
              <span className="theme-text-main block font-semibold">Lock when the device sleeps</span>
              <p className="theme-text-muted mt-0.5 text-[11px] leading-normal">Require authentication after the device wakes.</p>
            </div>
            <SettingsSwitch checked={appLock.status.lockOnSleep} disabled={pending || !appLock.status.enabled} label="lock when the device sleeps" onClick={() => void appLock.setLockOnSleep(!appLock.status.lockOnSleep).catch((cause) => setError(message(cause)))} />
          </div>
          <div className={`flex items-center justify-between gap-4 ${!appLock.status.enabled ? 'settings-disabled-row' : ''}`}>
            <span className="theme-text-main font-semibold">Lock after inactivity</span>
            <MenuSelect value={String(appLock.status.idleMinutes)} options={IDLE_OPTIONS} label="Auto-lock delay" disabled={pending || !appLock.status.enabled} onChange={(value) => void appLock.setIdleMinutes(Number(value)).catch((cause) => setError(message(cause)))} />
          </div>
          <div className={`flex items-start justify-between gap-4 ${!appLock.status.enabled ? 'settings-disabled-row' : ''}`}>
            <div className="min-w-0 flex-1">
              <span className="theme-text-main block font-semibold">Capture while locked</span>
              <p className="theme-text-muted mt-0.5 text-[11px] leading-normal">Keep recording new clipboard items while the interface is locked.</p>
            </div>
            <SettingsSwitch checked={appLock.status.captureWhileLocked} disabled={pending || !appLock.status.enabled} label="capture while locked" onClick={() => void appLock.setCaptureWhileLocked(!appLock.status.captureWhileLocked).catch((cause) => setError(message(cause)))} />
          </div>
      </div>
      <AppDialog
        isOpen={credentialMode !== null}
        onClose={closeCredentials}
        labelledBy="app-lock-credentials-title"
        isDirty={Boolean(currentPassphrase || passphrase || confirmation)}
        discardMessage="Discard the entered passphrase?"
        panelClassName="theme-panel w-full max-w-md overflow-hidden border shadow-2xl"
      >
        {({ requestClose }) => (
          <form onSubmit={credentialMode === 'disable' ? disable : configure}>
            <AppDialogHeader onClose={requestClose} closeLabel="Close app lock dialog">
              <AppDialogHeading
                id="app-lock-credentials-title"
                title={credentialMode === 'disable' ? 'Disable app lock?' : appLock.status.enabled ? 'Change passphrase' : 'Enable app lock'}
                description={credentialMode === 'disable' ? 'Clipboard history will open without authentication. Saved unlock preferences will no longer protect access.' : 'Set a fallback passphrase for app unlock.'}
                icon={<LockKeyhole />}
                tone={credentialMode === 'disable' ? 'danger' : 'default'}
              />
            </AppDialogHeader>
            <AppDialogBody className="space-y-3 text-xs">
              {appLock.status.enabled && <label className="block space-y-1">
                <span className="theme-text-main font-semibold">Current passphrase</span>
                <input autoFocus type="password" autoComplete="current-password" value={currentPassphrase} onChange={(event) => setCurrentPassphrase(event.target.value)} className="theme-input w-full rounded-lg border px-3 py-2" />
              </label>}
              {credentialMode === 'configure' && <>
                <label className="block space-y-1">
                  <span className="theme-text-main font-semibold">New passphrase</span>
                  <input autoFocus={!appLock.status.enabled} type="password" autoComplete="new-password" value={passphrase} onChange={(event) => setPassphrase(event.target.value)} className="theme-input w-full rounded-lg border px-3 py-2" />
                </label>
                <label className="block space-y-1">
                  <span className="theme-text-main font-semibold">Confirm passphrase</span>
                  <input type="password" autoComplete="new-password" value={confirmation} onChange={(event) => setConfirmation(event.target.value)} className="theme-input w-full rounded-lg border px-3 py-2" />
                </label>
              </>}
              {error && <p role="alert" className="theme-danger-text">{error}</p>}
            </AppDialogBody>
            <AppDialogFooter>
              <ActionButton onClick={requestClose} disabled={pending}>Cancel</ActionButton>
              <ActionButton
                type="submit"
                variant={credentialMode === 'disable' ? 'danger' : 'primary'}
                disabled={pending || (!currentPassphrase && appLock.status.enabled) || (credentialMode === 'configure' && (passphrase.length < 1 || passphrase !== confirmation))}
              >
                {credentialMode === 'disable' ? 'Disable app lock' : appLock.status.enabled ? 'Change passphrase' : 'Enable app lock'}
              </ActionButton>
            </AppDialogFooter>
          </form>
        )}
      </AppDialog>
    </div>
  );
}
