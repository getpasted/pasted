import { CheckCircle2, Download, Loader2, RefreshCw, ShieldCheck } from 'lucide-react';
import { lazy, Suspense, useEffect, useState } from 'react';

import { translate } from '../localization/runtime';
import type { AppUpdateStatus, AvailableAppUpdate } from '../updateTypes';
import { safeInvoke as invoke } from '../utils/tauri';
import { ActionButton } from './AppDialogLayout';
import { ConfirmationDialog, type ConfirmationDialogRequest } from './ConfirmationDialog';

const ReleaseNotesDialog = lazy(() => import('./ReleaseNotesDialog').then((module) => ({
  default: module.ReleaseNotesDialog,
})));

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
  const [showReleaseNotes, setShowReleaseNotes] = useState(false);

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
    setUpdate(null);
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
      details: (
        <div className="theme-surface space-y-2 rounded-xl border p-3">
          <ul className="list-disc space-y-1 ps-4">
            <li>{translate('component.settingsUpdateSection.libraryPreserved')}</li>
            <li>{translate('component.settingsUpdateSection.updateDownloadedAndSignatureVerified')}</li>
            <li>{translate('component.settingsUpdateSection.signatureMismatchRejected')}</li>
            <li>{translate('component.settingsUpdateSection.installationCompletesAndRestarts')}</li>
          </ul>
        </div>
      ),
      confirmLabel: translate('component.settingsUpdateSection.installAndRestart'),
      icon: <ShieldCheck />,
      tone: 'info',
      onConfirm: installUpdate,
    });
  };

  const isUpToDate = !checking && update?.available === false;

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
        variant={isUpToDate ? 'success' : 'secondary'}
        aria-label={translate('component.settingsUpdateSection.checkForUpdates')}
        title={isUpToDate
          ? translate('component.settingsUpdateSection.upToDate', { version: update.currentVersion })
          : translate('component.settingsUpdateSection.checkForUpdates')}
        disabled={!status?.configured || checking || installing}
        onClick={() => void checkForUpdate()}
        className="theme-badge rounded-full px-3 font-mono text-[10px] disabled:opacity-40"
      >
        {checking
          ? <Loader2 className="h-3.5 w-3.5 animate-spin" />
          : isUpToDate
            ? <CheckCircle2 className="h-3.5 w-3.5" />
            : <RefreshCw className="h-3.5 w-3.5" />}
        {versionLabel}
      </ActionButton>

      {status && !status.configured && (
        <div className="theme-text-muted text-[10px] leading-relaxed">
          {translate('component.settingsUpdateSection.unavailableInThisBuild')}
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
              {update.notes && (
                <button
                  type="button"
                  className="theme-inline-action theme-text-muted mt-1 cursor-pointer text-xs font-semibold underline decoration-transparent underline-offset-2"
                  onClick={() => setShowReleaseNotes(true)}
                >
                  {translate('component.settingsUpdateSection.viewChanges')}
                </button>
              )}
            </div>
            <ActionButton variant="solid-primary" disabled={installing} onClick={requestInstall} className="shrink-0">
              {installing && <Loader2 className="h-3.5 w-3.5 animate-spin" />}
              {installing
                ? translate('component.settingsUpdateSection.installing')
                : translate('component.settingsUpdateSection.installAndRestartPrompt')}
            </ActionButton>
          </div>
        </div>
      )}

      {error && <div role="alert" className="theme-status-danger w-full rounded-xl border px-3 py-2 text-xs">{error}</div>}
    </div>

    <ConfirmationDialog request={confirmation} onCancel={() => setConfirmation(null)} />
    <Suspense fallback={null}>
      <ReleaseNotesDialog
        isOpen={showReleaseNotes && Boolean(update?.notes && update.version)}
        notes={update?.notes ?? ''}
        version={update?.version ?? ''}
        onClose={() => setShowReleaseNotes(false)}
      />
    </Suspense>
  </>;
}
