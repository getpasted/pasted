import { CheckCircle2, Download, Loader2, RefreshCw } from 'lucide-react';
import { useEffect, useState } from 'react';

import { translate } from '../localization/runtime';
import type { AppUpdateStatus, AvailableAppUpdate } from '../updateTypes';
import { safeInvoke as invoke } from '../utils/tauri';
import { ActionButton } from './AppDialogLayout';
import { ConfirmationDialog, type ConfirmationDialogRequest } from './ConfirmationDialog';

export function SettingsUpdateSection({
  enabled,
  versionLabel,
}: {
  enabled: boolean;
  versionLabel: string;
}) {
  const [status, setStatus] = useState<AppUpdateStatus | null>(null);
  const [update, setUpdate] = useState<AvailableAppUpdate | null>(null);
  const [checking, setChecking] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState('');
  const [confirmation, setConfirmation] = useState<ConfirmationDialogRequest | null>(null);

  useEffect(() => {
    invoke<AppUpdateStatus>('get_app_update_status')
      .then((nextStatus) => {
        setStatus(nextStatus);
        if (!enabled || !nextStatus.enabled || !nextStatus.configured) return;
        setChecking(true);
        return invoke<AvailableAppUpdate>('check_for_app_update')
          .then(setUpdate)
          .catch((reason) => setError(String(reason)))
          .finally(() => setChecking(false));
      })
      .catch((reason) => setError(String(reason)));
  }, [enabled]);

  const checkForUpdate = async () => {
    if (!enabled || !status?.enabled) return;
    setChecking(true);
    setError('');
    try {
      setUpdate(await invoke<AvailableAppUpdate>('check_for_app_update'));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setChecking(false);
    }
  };

  const installUpdate = async () => {
    setConfirmation(null);
    setInstalling(true);
    setError('');
    try {
      await invoke('install_app_update');
    } catch (reason) {
      setInstalling(false);
      setError(String(reason));
    }
  };

  const requestInstall = () => {
    if (!update?.version) return;
    setConfirmation({
      title: translate('component.settingsUpdateSection.installVersion', { version: update.version }),
      description: translate('component.settingsUpdateSection.installDescription'),
      details: translate('component.settingsUpdateSection.libraryPreserved'),
      confirmLabel: translate('component.settingsUpdateSection.installAndRestart'),
      onConfirm: installUpdate,
    });
  };

  if (!enabled || status?.enabled === false) {
    return (
      <span className="theme-badge mt-4 rounded-full border px-3 py-1 font-mono text-[10px] font-semibold">
        {versionLabel}
      </span>
    );
  }

  return <>
    <div className="mt-4 flex w-full max-w-lg flex-col items-center gap-2">
      <ActionButton
        aria-label={translate('component.settingsUpdateSection.checkForUpdates')}
        title={translate('component.settingsUpdateSection.checkForUpdates')}
        disabled={!status?.configured || checking || installing}
        onClick={() => void checkForUpdate()}
        className="theme-badge rounded-full px-3 font-mono text-[10px] disabled:opacity-40"
      >
        {checking ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <RefreshCw className="h-3.5 w-3.5" />}
        {versionLabel}
      </ActionButton>

      {status && !status.configured && (
        <div className="theme-text-muted text-[10px] leading-relaxed">
          {translate('component.settingsUpdateSection.unavailableInThisBuild')}
        </div>
      )}

      {update && !update.available && (
        <div className="theme-status-success flex items-center gap-2 rounded-xl border px-3 py-2 text-xs">
          <CheckCircle2 className="h-4 w-4 shrink-0" />
          {translate('component.settingsUpdateSection.upToDate', { version: update.currentVersion })}
        </div>
      )}

      {update?.available && update.version && (
        <div className="theme-card-idle w-full border p-4 text-start">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-start">
            <Download className="mt-0.5 h-4 w-4 shrink-0 text-[var(--accent-primary)]" />
            <div className="min-w-0 flex-1">
              <div className="theme-title text-sm font-bold">
                {translate('component.settingsUpdateSection.versionAvailable', { version: update.version })}
              </div>
              {update.notes && <p className="theme-text-muted mt-1 whitespace-pre-wrap text-xs leading-relaxed">{update.notes}</p>}
            </div>
            <ActionButton variant="solid-primary" disabled={installing} onClick={requestInstall} className="shrink-0">
              {installing && <Loader2 className="h-3.5 w-3.5 animate-spin" />}
              {installing
                ? translate('component.settingsUpdateSection.installing')
                : translate('component.settingsUpdateSection.installAndRestart')}
            </ActionButton>
          </div>
        </div>
      )}

      {error && <div role="alert" className="theme-status-danger w-full rounded-xl border px-3 py-2 text-xs">{error}</div>}
    </div>

    <ConfirmationDialog request={confirmation} onCancel={() => setConfirmation(null)} />
  </>;
}
