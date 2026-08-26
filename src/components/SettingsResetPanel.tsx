import { useState } from 'react';
import { AlertTriangle } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { FactoryResetDialog } from './FactoryResetDialog';
import { useToast } from './ToastProvider';
import { ActionButton } from './AppDialogLayout';
import { collectBackupClientState } from '../utils/backupClientState';
import { translate } from '../localization/runtime';
import { backupApi } from '../api/backup';
import { resetPastedClientStorage } from '../utils/appUiState';
import { discardPendingScrollPositionPersistence } from '../utils/scrollPositionState';

interface SettingsResetPanelProps {
  onRefreshBins?: () => void;
  onRefreshManualTransforms?: () => void;
  onRefreshClips?: () => void;
  onRefreshTrashedClips?: () => void;
  onResetClientState?: (resetInPlace: boolean) => void;
}

export function SettingsResetPanel({
  onRefreshBins,
  onRefreshManualTransforms,
  onRefreshClips,
  onRefreshTrashedClips,
  onResetClientState,
}: SettingsResetPanelProps) {
  const { showToast } = useToast();
  const [isResetOpen, setIsResetOpen] = useState(false);

  const handleExport = async () => {
    await backupApi.exportFull(collectBackupClientState());
  };

  const handleFactoryReset = async () => {
    const isNative = Boolean((window as typeof window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
    await invoke('factory_reset_app');
    discardPendingScrollPositionPersistence();
    resetPastedClientStorage(localStorage);
    onResetClientState?.(!isNative);

    if (isNative && import.meta.env.DEV) {
      window.location.reload();
    } else if (!isNative) {
      // Keep browser previews usable; packaged Pasted restarts natively.
      onRefreshBins?.();
      onRefreshManualTransforms?.();
      onRefreshClips?.();
      onRefreshTrashedClips?.();
      setIsResetOpen(false);
      showToast({ tone: 'success', get message() { return translate('component.settingsResetPanel.pastedWasResetToItsFirstLaunchState'); } });
    }
  };

  return (
    <section className="theme-panel rounded-2xl border p-4">
      <div className="flex items-center justify-between gap-4">
        <div className="flex min-w-0 items-start gap-3">
          <div className="theme-status-danger shrink-0 rounded-lg border p-2">
            <AlertTriangle className="h-4 w-4" />
          </div>
          <div className="min-w-0 pt-0.5">
            <h3 className="theme-danger-text text-sm font-bold">{translate('component.settingsResetPanel.resetPasted')}</h3>
            <p className="mt-1 text-[11px] leading-relaxed theme-text-muted">
              {translate('component.settingsResetPanel.permanentlyEraseSavedDataAndReturnEverySettingToItsDefault')}
            </p>
          </div>
        </div>
        <ActionButton
          variant="solid-danger"
          onClick={() => setIsResetOpen(true)}
          className="shrink-0 cursor-pointer"
        >
          {translate('component.settingsResetPanel.resetPasted2')}
        </ActionButton>
      </div>

      <FactoryResetDialog
        isOpen={isResetOpen}
        onClose={() => setIsResetOpen(false)}
        onExport={handleExport}
        onReset={handleFactoryReset}
      />
    </section>
  );
}
