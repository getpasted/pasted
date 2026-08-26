import { useEffect, useState } from 'react';
import { ArrowRight, Database, RotateCcw } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
import {
  LibraryTransitionDialog,
  waitForMinimumLibraryTransition,
} from './LibraryTransitionDialog';
import { useToast } from './ToastProvider';
import { ExternalHistoryImport } from './ExternalHistoryImport';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';
import { collectBackupClientState } from '../utils/backupClientState';
import { useLocalization } from '../localization/LocalizationProvider';
import { translate } from '../localization/runtime';
import { activityApi } from '../api/activity';
import { backupApi } from '../api/backup';
import { SettingsSyncLibrarySection } from './SettingsSyncLibrarySection';
import { SettingsSyncExportSection } from './SettingsSyncExportSection';
import { SettingsSyncImportSection } from './SettingsSyncImportSection';
import { restoreFullBackupWithTransition } from './fullBackupRestore';
import { cacheStorageProtection, getCachedStorageProtection, loadStorageProtection } from './settingsSyncStorageProtection';
import type {
  ExportDataId,
  ExportFormat,
  ExportMode,
  ImportFileInspection,
  LibraryLocationInfo,
  LibraryMoveReport,
  StorageProtectionInfo,
} from './settingsSyncModel';

interface SettingsSyncPanelProps {
  onRefreshBins?: () => void;
  onRefreshManualTransforms?: () => void;
  onRefreshClips?: () => void;
  onRefreshTrashedClips?: () => void;
  analyticsEnabled?: boolean;
  activityEnabled?: boolean;
  onOpenAnalytics?: () => void;
}

export function SettingsSyncPanel({
  onRefreshBins,
  onRefreshManualTransforms,
  onRefreshClips,
  onRefreshTrashedClips,
  analyticsEnabled = false,
  activityEnabled = false,
  onOpenAnalytics,
}: SettingsSyncPanelProps) {
  const { formatDateTime, formatNumber } = useLocalization();
  const { showToast } = useToast();
  const [isImporting, setIsImporting] = useState(false);
  const [isInspectingImport, setIsInspectingImport] = useState(false);
  const [importInspection, setImportInspection] = useState<ImportFileInspection | null>(null);
  const [importInspectionError, setImportInspectionError] = useState<string | null>(null);
  const [isMoving, setIsMoving] = useState(false);
  const [isCreatingFullBackup, setIsCreatingFullBackup] = useState(false);
  const [isRestoringFullBackup, setIsRestoringFullBackup] = useState(false);
  const [isRestoreConfirmOpen, setIsRestoreConfirmOpen] = useState(false);
  const [location, setLocation] = useState<LibraryLocationInfo | null>(null);
  const [storageProtection, setStorageProtection] = useState<StorageProtectionInfo | null>(getCachedStorageProtection);
  const [exportMode, setExportMode] = useState<ExportMode>('full');
  const [exportFormat, setExportFormat] = useState<ExportFormat>('json');
  const [exportData, setExportData] = useState<Record<ExportDataId, boolean>>({
    clips: true,
    organization: true,
    activity: false,
    settings: false,
    recovery: false,
    interface: false,
  });

  const refreshLocation = async () => {
    try {
      setLocation(await invoke<LibraryLocationInfo>('get_library_location'));
    } catch (error) {
      console.error('Could not load database location:', error);
    }
  };

  const refreshStorageProtection = async (force = false) => {
    try {
      setStorageProtection(await loadStorageProtection(force));
    } catch (error) {
      console.error('Could not inspect storage protection:', error);
      const unavailable: StorageProtectionInfo = {
        status: 'unknown',
        technology: null,
        get summary() { return translate('component.settingsSyncPanel.volumeEncryptionCouldNotBeDetermined'); },
        get detail() { return translate('component.settingsSyncPanel.checkTheOperatingSystemSStorageSecuritySettings'); },
      };
      cacheStorageProtection(unavailable);
      setStorageProtection(unavailable);
    }
  };

  useEffect(() => {
    void refreshLocation();
    void refreshStorageProtection();
  }, []);

  const handleMoveLibrary = async () => {
    const transitionStartedAt = performance.now();
    setIsMoving(true);
    try {
      const report = await invoke<LibraryMoveReport | null>('move_library');
      if (!report) return;
      setLocation(report.location);
      await refreshStorageProtection(true);
      await waitForMinimumLibraryTransition(transitionStartedAt);
      showToast({
        tone: 'success',
        get message() { return translate('component.settingsSyncPanel.databaseMovedThePreviousDatabaseWasKeptAsARecoveryCopy'); },
      });
    } catch (error) {
      console.error('Library move failed:', error);
      await waitForMinimumLibraryTransition(transitionStartedAt);
      showToast({ tone: 'error', message: String(error), durationMs: 8000 });
    } finally {
      setIsMoving(false);
    }
  };

  const handleRestoreDefault = async () => {
    const transitionStartedAt = performance.now();
    setIsMoving(true);
    try {
      const report = await invoke<LibraryMoveReport>('restore_default_library_location');
      setLocation(report.location);
      await refreshStorageProtection(true);
      await waitForMinimumLibraryTransition(transitionStartedAt);
      showToast({
        tone: 'success',
        get message() { return translate('component.settingsSyncPanel.databaseReturnedToItsDefaultLocationTheCustomDatabaseWasKeptAs'); },
      });
    } catch (error) {
      console.error('Default library restore failed:', error);
      await waitForMinimumLibraryTransition(transitionStartedAt);
      showToast({ tone: 'error', message: String(error), durationMs: 8000 });
    } finally {
      setIsMoving(false);
    }
  };

  const handleExport = async () => {
    try {
      const savedPath = await backupApi.exportTransfer();
      if (savedPath) showToast({ tone: 'success', get message() { return translate('component.settingsSyncPanel.historyAndOrganizationDataExportedSuccessfully'); } });
    } catch (error) {
      console.error('History and organization export failed:', error);
      showToast({ tone: 'error', get message() { return translate('component.settingsSyncPanel.historyAndOrganizationExportFailed'); } });
    }
  };

  const downloadActivityExport = async (format: 'json' | 'csv') => {
    try {
      const contents = format === 'json'
        ? await activityApi.exportJson()
        : await activityApi.exportCsv();
      const url = URL.createObjectURL(new Blob([contents], {
        type: format === 'json' ? 'application/json' : 'text/csv',
      }));
      const link = document.createElement('a');
      link.href = url;
      link.download = `pasted_activity_${Date.now()}.${format}`;
      link.click();
      URL.revokeObjectURL(url);
      showToast({ tone: 'success', message: translate('component.settingsSyncPanel.valueActivityExportDownloaded', { value: format.toUpperCase() }) });
    } catch (error) {
      console.error(`Failed to export Activity as ${format}:`, error);
      showToast({ tone: 'error', message: translate('component.settingsSyncPanel.valueActivityExportFailed', { value: format.toUpperCase() }) });
    }
  };

  const downloadClipExport = async (format: 'json' | 'csv') => {
    try {
      const contents = format === 'json'
        ? await invoke<string>('export_clips_json')
        : await invoke<string>('export_clips_csv');
      const url = URL.createObjectURL(new Blob([contents], {
        type: format === 'json' ? 'application/json' : 'text/csv',
      }));
      const link = document.createElement('a');
      link.href = url;
      link.download = `pasted_clips_${Date.now()}.${format}`;
      link.click();
      URL.revokeObjectURL(url);
      showToast({ tone: 'success', message: translate('component.settingsSyncPanel.valueClipExportDownloaded', { value: format.toUpperCase() }) });
    } catch (error) {
      console.error(`Failed to export clips as ${format}:`, error);
      showToast({ tone: 'error', message: translate('component.settingsSyncPanel.valueClipExportFailed', { value: format.toUpperCase() }) });
    }
  };

  const handleChooseImportFile = async () => {
    setIsInspectingImport(true);
    setImportInspectionError(null);
    try {
      const inspection = await backupApi.chooseImport<ImportFileInspection>();
      if (inspection) setImportInspection(inspection);
    } catch (error) {
      console.error('Import file inspection failed:', error);
      setImportInspection(null);
      setImportInspectionError(String(error));
    } finally {
      setIsInspectingImport(false);
    }
  };

  const handleMergeImport = async () => {
    if (!importInspection || importInspection.kind === 'backup') return;
    const selected = importInspection;
    const transitionStartedAt = performance.now();
    setIsImporting(true);
    try {
      const report = await backupApi.importInspected<Record<string, number>>(
        selected.path,
        selected.kind,
        selected.format,
      );
      if (selected.kind === 'organization') {
        await Promise.all([
          Promise.resolve(onRefreshBins?.()),
          Promise.resolve(onRefreshManualTransforms?.()),
          Promise.resolve(onRefreshClips?.()),
          Promise.resolve(onRefreshTrashedClips?.()),
        ]);
      } else if (selected.kind === 'clips') {
        await Promise.resolve(onRefreshClips?.());
      }
      await waitForMinimumLibraryTransition(transitionStartedAt);
      const importedCount = report.importedCount ?? 0;
      const duplicateCount = report.duplicateCount ?? 0;
      showToast({
        tone: 'success',
        message: selected.kind === 'organization'
          ? translate('component.settingsSyncPanel.historyAndOrganizationMergedProcessedCountClips', { count: importedCount })
          : selected.kind === 'activity'
            ? translate('component.settingsSyncPanel.mergedActivityEntries', { count: importedCount, duplicateCount })
            : translate('component.settingsSyncPanel.mergedClips', { count: importedCount, duplicateCount }),
      });
      setImportInspection(null);
    } catch (error) {
      console.error('Import failed after inspection:', error);
      showToast({ tone: 'error', message: translate('component.settingsSyncPanel.theSelectedFileCouldNotBeMergedValue', { value: String(error) }), durationMs: 8000 });
    } finally {
      setIsImporting(false);
    }
  };

  const toggleExportData = (id: ExportDataId) => {
    setExportData((current) => {
      const next = { ...current, [id]: !current[id] };
      if (id === 'organization' && next.organization) next.clips = true;
      if (id === 'clips' && !next.clips) next.organization = false;
      return next;
    });
  };

  const chooseExportFormat = (format: ExportFormat) => {
    setExportFormat(format);
  };

  const handleSelectedExport = async () => {
    if (exportMode === 'full') {
      await handleCreateFullBackup();
      return;
    }
    const tasks: Array<Promise<void>> = [];
    const organizationSelected = exportFormat === 'json' && exportData.organization;
    if (organizationSelected) {
      tasks.push(handleExport());
    } else if (exportData.clips) {
      tasks.push(downloadClipExport(exportFormat));
    }
    if (exportData.activity) tasks.push(downloadActivityExport(exportFormat));
    await Promise.all(tasks);
  };

  const handleCreateFullBackup = async () => {
    setIsCreatingFullBackup(true);
    try {
      const report = await backupApi.exportFull(collectBackupClientState());
      if (report) showToast({ tone: 'success', get message() { return translate('component.settingsSyncPanel.fullBackupCreatedSuccessfully'); } });
    } catch (error) {
      console.error('Full backup creation failed:', error);
      showToast({ tone: 'error', message: translate('component.settingsSyncPanel.fullBackupCreationFailedValue', { value: String(error) }), durationMs: 8000 });
    } finally {
      setIsCreatingFullBackup(false);
    }
  };

  const handleRestoreFullBackup = async () => {
    setIsRestoreConfirmOpen(false);
    setIsRestoringFullBackup(true);
    try {
      const outcome = await restoreFullBackupWithTransition(
        importInspection?.kind === 'backup' ? importInspection.path : undefined,
      );
      if (outcome === 'cancelled') {
        setIsRestoringFullBackup(false);
        return;
      }
      setImportInspection(null);
      if (outcome === 'restarting') return;
      showToast({
        tone: 'success',
        get message() { return translate('component.settingsSyncPanel.fullBackupRestoredRestartingWithTheRestoredState'); },
      });
      setIsRestoringFullBackup(false);
    } catch (error) {
      console.error('Full backup restore failed:', error);
      showToast({ tone: 'error', message: translate('component.settingsSyncPanel.fullBackupRestoreFailedValue', { value: String(error) }), durationMs: 8000 });
      setIsRestoringFullBackup(false);
    }
  };

  return (
    <div className="space-y-5 text-xs">
      <SettingsPanelHeader
        icon={Database}
        title={translate('component.settingsSyncPanel.storage')}
        description={translate('component.settingsSyncPanel.manageRecoveryAndDataTransfers')}
      />

      <SettingsSyncLibrarySection
        location={location}
        storageProtection={storageProtection}
        isMoving={isMoving}
        onMove={() => void handleMoveLibrary()}
        onRestoreDefault={() => void handleRestoreDefault()}
      />

      <SettingsSyncExportSection
        activityEnabled={activityEnabled}
        exportMode={exportMode}
        exportFormat={exportFormat}
        exportData={exportData}
        isExporting={isCreatingFullBackup}
        onChooseMode={setExportMode}
        onChooseFormat={chooseExportFormat}
        onToggleData={toggleExportData}
        onExport={() => void handleSelectedExport()}
      />

      <SettingsSyncImportSection
        inspection={importInspection}
        inspectionError={importInspectionError}
        isInspecting={isInspectingImport}
        isImporting={isImporting}
        isRestoring={isRestoringFullBackup}
        formatDateTime={formatDateTime}
        formatNumber={formatNumber}
        onChooseFile={() => void handleChooseImportFile()}
        onRemoveFile={() => setImportInspection(null)}
        onMerge={() => void handleMergeImport()}
        onRecover={() => setIsRestoreConfirmOpen(true)}
      />

      <section className="space-y-3 border-t theme-divider pt-5" aria-labelledby="migration-title">
        <SettingsSubsectionHeader
          id="migration-title"
          title={translate('component.settingsSyncPanel.moveFromAnotherClipboardManager')}
          description={translate('component.settingsSyncPanel.importSupportedHistoryWithoutChangingTheSourceClipboardManager')}
        />
        <ExternalHistoryImport
          onImported={async () => {
            await Promise.all([
              Promise.resolve(onRefreshClips?.()),
              Promise.resolve(onRefreshTrashedClips?.()),
            ]);
          }}
        />
      </section>

      {analyticsEnabled && onOpenAnalytics && (
        <button
          type="button"
          onClick={onOpenAnalytics}
          className="theme-secondary-button flex w-full items-center justify-between rounded-xl border px-4 py-3 text-start"
        >
          <span>
            <strong className="theme-title block text-xs">{translate('component.settingsSyncPanel.curiousWhatSTakingUpSpace')}</strong>
            <span className="theme-text-muted mt-0.5 block text-[11px]">{translate('component.settingsSyncPanel.openInsights')}</span>
          </span>
          <ArrowRight className="h-4 w-4 shrink-0 rtl:-scale-x-100" />
        </button>
      )}

      <LibraryTransitionDialog
        isOpen={isImporting || isRestoringFullBackup}
        variant="import"
        title={isRestoringFullBackup ? translate('component.settingsSyncPanel.recoveringFromBackup') : translate('component.settingsSyncPanel.mergingSelectedData')}
        description={isRestoringFullBackup ? translate('component.settingsSyncPanel.validatingAndReplacingTheCompleteLocalState') : translate('component.settingsSyncPanel.validatingAgainAndAddingTheSelectedRecords')}
      />
      <LibraryTransitionDialog
        isOpen={isMoving}
        variant="move"
        title={translate('component.settingsSyncPanel.movingDatabase')}
        description={translate('component.settingsSyncPanel.carryingEveryClipBinTransformAndRevisionToItsNewHome')}
      />
      <AppDialog
        isOpen={isRestoreConfirmOpen}
        onClose={() => setIsRestoreConfirmOpen(false)}
        labelledBy="restore-full-backup-title"
        panelClassName="app-dialog-danger theme-panel w-full max-w-md rounded-2xl border overflow-hidden font-sans"
      >
        {({ requestClose }) => <>
          <AppDialogHeader onClose={requestClose}>
            <AppDialogHeading
              id="restore-full-backup-title"
              title={translate('component.settingsSyncPanel.recoverFromBackup')}
              description={importInspection?.name ?? translate('component.settingsSyncPanel.theSelectedBackupWillReplaceTheCurrentState')}
              icon={<RotateCcw />}
              tone="danger"
            />
          </AppDialogHeader>
          <AppDialogBody>
            <p className="app-dialog-message theme-surface rounded-xl border p-3 text-xs leading-relaxed">
              {translate('component.settingsSyncPanel.theBackupIsValidatedBeforeReplacementAFullRecoveryBackupOfThe')}
            </p>
          </AppDialogBody>
          <AppDialogFooter>
            <AppDialogButton onClick={requestClose} autoFocus>{translate('common.cancel')}</AppDialogButton>
            <AppDialogButton variant="danger" onClick={() => void handleRestoreFullBackup()}>{translate('component.settingsSyncPanel.recover')}</AppDialogButton>
          </AppDialogFooter>
        </>}
      </AppDialog>
    </div>
  );
}
