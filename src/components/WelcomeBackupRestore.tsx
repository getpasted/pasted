import { ArchiveRestore, LoaderCircle } from 'lucide-react';
import { useState } from 'react';
import { backupApi } from '../api/backup';
import { translate } from '../localization/runtime';
import { ActionButton } from './AppDialogLayout';
import { ConfirmationDialog, type ConfirmationDialogRequest } from './ConfirmationDialog';
import { restoreFullBackupWithTransition } from './fullBackupRestore';
import { LibraryTransitionDialog } from './LibraryTransitionDialog';
import type { ImportFileInspection } from './settingsSyncModel';
import { useToast } from './ToastProvider';

export function WelcomeBackupRestore() {
  const { showToast } = useToast();
  const [isChoosing, setIsChoosing] = useState(false);
  const [isRestoring, setIsRestoring] = useState(false);
  const [confirmation, setConfirmation] = useState<ConfirmationDialogRequest | null>(null);

  const restore = async (path: string) => {
    setConfirmation(null);
    setIsRestoring(true);
    try {
      const outcome = await restoreFullBackupWithTransition(path);
      if (outcome === 'cancelled') {
        setIsRestoring(false);
        return;
      }
      if (outcome === 'restarting') return;
      showToast({ tone: 'success', message: translate('component.settingsSyncPanel.fullBackupRestoredRestartingWithTheRestoredState') });
      setIsRestoring(false);
    } catch (error) {
      console.error('Welcome Full Backup restore failed:', error);
      showToast({ tone: 'error', message: translate('component.settingsSyncPanel.fullBackupRestoreFailedValue', { value: String(error) }), durationMs: 8000 });
      setIsRestoring(false);
    }
  };

  const chooseBackup = async () => {
    setIsChoosing(true);
    try {
      const inspection = await backupApi.chooseImport<ImportFileInspection>();
      if (!inspection) return;
      if (inspection.kind !== 'backup') {
        showToast({ tone: 'error', message: translate('component.welcomeBackupRestore.chooseAPastedFullBackupFile') });
        return;
      }
      setConfirmation({
        title: translate('component.settingsSyncPanel.recoverFromBackup'),
        description: inspection.name,
        details: translate('component.settingsSyncPanel.theBackupIsValidatedBeforeReplacementAFullRecoveryBackupOfThe'),
        confirmLabel: translate('component.settingsSyncPanel.recover'),
        tone: 'danger',
        onConfirm: () => restore(inspection.path),
      });
    } catch (error) {
      console.error('Welcome Full Backup inspection failed:', error);
      showToast({ tone: 'error', message: String(error), durationMs: 8000 });
    } finally {
      setIsChoosing(false);
    }
  };

  return <>
    <section className="theme-status-info flex items-center gap-3 rounded-xl border p-3">
      <span className="grid h-10 w-10 shrink-0 place-items-center rounded-xl border" aria-hidden="true">
        <ArchiveRestore className="h-5 w-5" />
      </span>
      <span className="min-w-0 flex-1">
        <strong className="theme-title block text-xs">{translate('component.welcomeBackupRestore.pastedFullBackup')}</strong>
        <span className="theme-text-muted mt-0.5 block text-[10px] leading-relaxed">
          {translate('component.welcomeBackupRestore.restoreTheCompleteWorkspaceFromAPastedbackupFile')}
        </span>
      </span>
      <ActionButton variant="primary" disabled={isChoosing || isRestoring} onClick={() => void chooseBackup()} className="shrink-0 disabled:opacity-50">
        {isChoosing ? <LoaderCircle className="h-4 w-4 animate-spin" /> : <ArchiveRestore className="h-4 w-4" />}
        {translate('component.welcomeBackupRestore.chooseBackup')}
      </ActionButton>
    </section>
    <ConfirmationDialog request={confirmation} onCancel={() => setConfirmation(null)} />
    <LibraryTransitionDialog
      isOpen={isRestoring}
      variant="import"
      title={translate('component.settingsSyncPanel.recoveringFromBackup')}
      description={translate('component.settingsSyncPanel.validatingAndReplacingTheCompleteLocalState')}
    />
  </>;
}
