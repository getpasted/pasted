import { FormEvent, useState } from 'react';
import { LockKeyhole } from 'lucide-react';
import { useAppLock } from '../hooks/useAppLock';
import { AppDialog } from './AppDialog';
import { ActionButton, AppDialogBody, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';
import { MenuSelect } from './MenuSelect';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
import { SettingsSwitch } from './SettingsSwitch';
import { translate } from '../localization/runtime';

const IDLE_OPTIONS = [
  { value: '0', get label() { return translate('component.settingsSecurityPanel.never'); } },
  { value: '1', get label() { return translate('component.settingsSecurityPanel.value1Minute'); } },
  { value: '5', get label() { return translate('component.settingsSecurityPanel.value5Minutes'); } },
  { value: '60', get label() { return translate('component.settingsSecurityPanel.value1Hour'); } },
  { value: '480', get label() { return translate('component.settingsSecurityPanel.value8Hours'); } },
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
      setError(translate('component.settingsSecurityPanel.newPassphrasesDoNotMatch'));
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
        title={translate('component.settingsSecurityPanel.security')}
        description={translate('component.settingsSecurityPanel.manageAccessAndAuthentication')}
        actions={appLock.status.enabled ? <ActionButton onClick={() => void appLock.lock()}><LockKeyhole className="h-3.5 w-3.5" /> {translate('component.settingsSecurityPanel.lockNow')}</ActionButton> : undefined}
      />

      <div className="space-y-3">
        <SettingsSubsectionHeader
          title={translate('component.settingsSecurityPanel.appLock')}
          description={translate('component.settingsSecurityPanel.requireAuthenticationToInteractWithPasted')}
        />
        <div className="flex justify-end gap-2">
          {appLock.status.enabled && <ActionButton variant="danger" disabled={pending} onClick={() => openCredentials('disable')}>{translate('component.settingsSecurityPanel.disableAppLock')}</ActionButton>}
          <ActionButton variant="primary" disabled={pending} onClick={() => openCredentials('configure')}>
            {appLock.status.enabled ? translate('component.settingsSecurityPanel.changePassphrase') : translate('component.settingsSecurityPanel.enableAppLock')}
          </ActionButton>
        </div>
        {error && !credentialMode && <p role="alert" className="theme-danger-text">{error}</p>}
      </div>

      <div className="theme-divider border-t" />
      <div className="space-y-4">
        <SettingsSubsectionHeader title={translate('component.settingsSecurityPanel.unlock')} description={translate('component.settingsSecurityPanel.biometricDataStaysWithTheOperatingSystem')} />
          <div className={`flex items-start justify-between gap-4 ${!appLock.status.enabled || !appLock.status.systemAuthAvailable ? 'settings-disabled-row' : ''}`}>
            <div className="min-w-0 flex-1">
              <span className="theme-text-main block font-semibold">{translate('component.settingsSecurityPanel.unlockUsingMethod', { method: appLock.status.systemAuthLabel })}</span>
              <p className="theme-text-muted mt-0.5 text-[11px] leading-normal">{appLock.status.systemAuthAvailable ? translate('component.settingsSecurityPanel.theOperatingSystemReportsOnlyWhetherAuthenticationSucceeded') : translate('component.settingsSecurityPanel.notAvailableOnThisDeviceOrDesktopSession')}</p>
            </div>
            <SettingsSwitch
              checked={appLock.status.systemAuthEnabled}
              disabled={pending || !appLock.status.enabled || !appLock.status.systemAuthAvailable}
              label={translate('component.settingsSecurityPanel.unlockUsingMethod', { method: appLock.status.systemAuthLabel })}
              onClick={() => {
                setPending(true);
                setError('');
                void appLock.setSystemAuth(!appLock.status.systemAuthEnabled).catch((cause) => setError(message(cause))).finally(() => setPending(false));
              }}
            />
          </div>
          {isMac && <div className={`flex items-start justify-between gap-4 ${!appLock.status.enabled || !appLock.status.appleWatchAvailable ? 'settings-disabled-row' : ''}`}>
            <div className="min-w-0 flex-1">
              <span className="theme-text-main block font-semibold">{translate('component.settingsSecurityPanel.unlockUsingAppleWatch')}</span>
              <p className="theme-text-muted mt-0.5 text-[11px] leading-normal">{appLock.status.appleWatchAvailable ? translate('component.settingsSecurityPanel.approveUnlockFromANearbyPairedAppleWatch') : translate('component.settingsSecurityPanel.notAvailableWithoutACompatiblePairedAppleWatch')}</p>
            </div>
            <SettingsSwitch
              checked={appLock.status.appleWatchEnabled}
              disabled={pending || !appLock.status.enabled || !appLock.status.appleWatchAvailable}
              label={translate('component.settingsSecurityPanel.unlockUsingAppleWatch2')}
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
          <SettingsSubsectionHeader title={translate('component.settingsSecurityPanel.autoLock')} description={translate('component.settingsSecurityPanel.chooseWhenAuthenticationIsRequiredAgain')} />
          <div className={`flex items-start justify-between gap-4 ${!appLock.status.enabled ? 'settings-disabled-row' : ''}`}>
            <div className="min-w-0 flex-1">
              <span className="theme-text-main block font-semibold">{translate('component.settingsSecurityPanel.lockAfterRestart')}</span>
              <p className="theme-text-muted mt-0.5 text-[11px] leading-normal">{translate('component.settingsSecurityPanel.requireAuthenticationAfterClosingAndReopeningTheApp')}</p>
            </div>
            <SettingsSwitch checked={appLock.status.lockOnRestart} disabled={pending || !appLock.status.enabled} label={translate('component.settingsSecurityPanel.lockAfterRestart2')} onClick={() => void appLock.setLockOnRestart(!appLock.status.lockOnRestart).catch((cause) => setError(message(cause)))} />
          </div>
          <div className={`flex items-start justify-between gap-4 ${!appLock.status.enabled ? 'settings-disabled-row' : ''}`}>
            <div className="min-w-0 flex-1">
              <span className="theme-text-main block font-semibold">{translate('component.settingsSecurityPanel.lockWhenTheDeviceSleeps')}</span>
              <p className="theme-text-muted mt-0.5 text-[11px] leading-normal">{translate('component.settingsSecurityPanel.requireAuthenticationAfterTheDeviceWakes')}</p>
            </div>
            <SettingsSwitch checked={appLock.status.lockOnSleep} disabled={pending || !appLock.status.enabled} label={translate('component.settingsSecurityPanel.lockWhenTheDeviceSleeps2')} onClick={() => void appLock.setLockOnSleep(!appLock.status.lockOnSleep).catch((cause) => setError(message(cause)))} />
          </div>
          <div className={`flex items-center justify-between gap-4 ${!appLock.status.enabled ? 'settings-disabled-row' : ''}`}>
            <span className="theme-text-main font-semibold">{translate('component.settingsSecurityPanel.lockAfterInactivity')}</span>
            <MenuSelect value={String(appLock.status.idleMinutes)} options={IDLE_OPTIONS} label={translate('component.settingsSecurityPanel.autoLockDelay')} disabled={pending || !appLock.status.enabled} onChange={(value) => void appLock.setIdleMinutes(Number(value)).catch((cause) => setError(message(cause)))} />
          </div>
      </div>
      <div className="theme-divider border-t" />
      <div className={`flex items-start justify-between gap-4 ${!appLock.status.enabled ? 'settings-disabled-row' : ''}`}>
        <div className="min-w-0 flex-1">
          <span className="theme-text-main block font-semibold">{translate('component.settingsSecurityPanel.captureWhileLocked')}</span>
          <p className="theme-text-muted mt-0.5 text-[11px] leading-normal">{translate('component.settingsSecurityPanel.keepCapturingNewClipsWhileTheInterfaceIsLocked')}</p>
        </div>
        <SettingsSwitch checked={appLock.status.captureWhileLocked} disabled={pending || !appLock.status.enabled} label={translate('component.settingsSecurityPanel.captureWhileLocked2')} onClick={() => void appLock.setCaptureWhileLocked(!appLock.status.captureWhileLocked).catch((cause) => setError(message(cause)))} />
      </div>
      <AppDialog
        isOpen={credentialMode !== null}
        onClose={closeCredentials}
        labelledBy="app-lock-credentials-title"
        isDirty={Boolean(currentPassphrase || passphrase || confirmation)}
        discardMessage={translate('component.settingsSecurityPanel.discardEnteredPassphrase')}
        panelClassName="theme-panel w-full max-w-md overflow-hidden border shadow-2xl"
      >
        {({ requestClose }) => (
          <form onSubmit={credentialMode === 'disable' ? disable : configure}>
            <AppDialogHeader onClose={requestClose} closeLabel={translate('component.settingsSecurityPanel.closeAppLockDialog')}>
              <AppDialogHeading
                id="app-lock-credentials-title"
                title={credentialMode === 'disable' ? translate('component.settingsSecurityPanel.disableAppLock2') : appLock.status.enabled ? translate('component.settingsSecurityPanel.changePassphrase2') : translate('component.settingsSecurityPanel.enableAppLock2')}
                description={credentialMode === 'disable' ? translate('component.settingsSecurityPanel.clipboardHistoryWillOpenWithoutAuthenticationSavedUnlockPreferencesWillNoLonger') : translate('component.settingsSecurityPanel.setAFallbackPassphraseForAppUnlock')}
                icon={<LockKeyhole />}
                tone={credentialMode === 'disable' ? 'danger' : 'default'}
              />
            </AppDialogHeader>
            <AppDialogBody className="space-y-3 text-xs">
              {appLock.status.enabled && <label className="block space-y-1">
                <span className="theme-text-main font-semibold">{translate('component.settingsSecurityPanel.currentPassphrase')}</span>
                <input autoFocus type="password" autoComplete="current-password" value={currentPassphrase} onChange={(event) => setCurrentPassphrase(event.target.value)} className="theme-input w-full rounded-lg border px-3 py-2" />
              </label>}
              {credentialMode === 'configure' && <>
                <label className="block space-y-1">
                  <span className="theme-text-main font-semibold">{translate('component.settingsSecurityPanel.newPassphrase')}</span>
                  <input autoFocus={!appLock.status.enabled} type="password" autoComplete="new-password" value={passphrase} onChange={(event) => setPassphrase(event.target.value)} className="theme-input w-full rounded-lg border px-3 py-2" />
                </label>
                <label className="block space-y-1">
                  <span className="theme-text-main font-semibold">{translate('component.settingsSecurityPanel.confirmPassphrase')}</span>
                  <input type="password" autoComplete="new-password" value={confirmation} onChange={(event) => setConfirmation(event.target.value)} className="theme-input w-full rounded-lg border px-3 py-2" />
                </label>
              </>}
              {error && <p role="alert" className="theme-danger-text">{error}</p>}
            </AppDialogBody>
            <AppDialogFooter>
              <ActionButton onClick={requestClose} disabled={pending}>{translate('common.cancel')}</ActionButton>
              <ActionButton
                type="submit"
                variant={credentialMode === 'disable' ? 'danger' : 'primary'}
                disabled={pending || (!currentPassphrase && appLock.status.enabled) || (credentialMode === 'configure' && (passphrase.length < 1 || passphrase !== confirmation))}
              >
                {credentialMode === 'disable' ? translate('component.settingsSecurityPanel.disableAppLock3') : appLock.status.enabled ? translate('component.settingsSecurityPanel.changePassphrase2') : translate('component.settingsSecurityPanel.enableAppLock2')}
              </ActionButton>
            </AppDialogFooter>
          </form>
        )}
      </AppDialog>
    </div>
  );
}
