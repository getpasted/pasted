import { useEffect, useState } from 'react';
import { ArrowRight, CheckCircle2, Database, Download, FileWarning, FolderInput, LoaderCircle, RotateCcw, ShieldAlert, ShieldCheck, ShieldQuestion, Upload, X } from 'lucide-react';
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
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading, ActionButton } from './AppDialogLayout';
import { collectBackupClientState } from '../utils/backupClientState';
import { SettingsSwitch } from './SettingsSwitch';
import { useLocalization } from '../localization/LocalizationProvider';
import { translate } from '../localization/runtime';

interface SettingsSyncPanelProps {
  onRefreshBins?: () => void;
  onRefreshPipelines?: () => void;
  onRefreshClips?: () => void;
  onRefreshTrashedClips?: () => void;
  analyticsEnabled?: boolean;
  activityEnabled?: boolean;
  onOpenAnalytics?: () => void;
}

interface LibraryLocationInfo {
  path: string;
  directory: string;
  isDefault: boolean;
}

interface LibraryMoveReport {
  location: LibraryLocationInfo;
  recoveryPath: string;
}

interface StorageProtectionInfo {
  status: 'protected' | 'notDetected' | 'unknown';
  technology: string | null;
  summary: string;
  detail: string;
}

let cachedStorageProtection: StorageProtectionInfo | null = null;
let storageProtectionRequest: Promise<StorageProtectionInfo> | null = null;

function loadStorageProtection(force = false): Promise<StorageProtectionInfo> {
  if (force) {
    cachedStorageProtection = null;
    storageProtectionRequest = null;
  }
  if (cachedStorageProtection) return Promise.resolve(cachedStorageProtection);
  if (storageProtectionRequest) return storageProtectionRequest;
  storageProtectionRequest = invoke<StorageProtectionInfo>('get_storage_protection')
    .then((protection) => {
      cachedStorageProtection = protection;
      return protection;
    })
    .finally(() => {
      storageProtectionRequest = null;
    });
  return storageProtectionRequest;
}

interface FullBackupReport {
  path: string;
  createdAt: string;
  sizeBytes: number;
}

interface FullRestoreReport {
  recoveryPath: string;
  backupCreatedAt: string;
}

interface LibraryArchiveInspection {
  schemaVersion: number;
  clipCount: number;
  binCount: number;
  operationCount: number;
  transformCount: number;
  classifierCount: number;
  contentTypeCount: number;
}

type ImportKind = 'clips' | 'activity' | 'organization' | 'backup';

interface ImportFileInspection {
  path: string;
  name: string;
  kind: ImportKind;
  format: 'json' | 'csv' | 'backup';
  sizeBytes: number;
  report?: ClipImportReport | ActivityImportReport;
  library?: LibraryArchiveInspection;
  backup?: {
    formatVersion: number;
    createdAt: string;
    sizeBytes: number;
  };
}

interface ActivityImportReport {
  scannedCount: number;
  importedCount: number;
  duplicateCount: number;
  retainedCount: number;
}

interface ClipImportReport {
  scannedCount: number;
  importedCount: number;
  duplicateCount: number;
}

type ExportMode = 'custom' | 'full';
type ExportFormat = 'json' | 'csv';
type VisibleExportFormat = ExportFormat | 'backup';
type ExportDataId = 'clips' | 'organization' | 'activity' | 'settings' | 'recovery' | 'interface';

const EXPORT_EXTENSION: Record<VisibleExportFormat, string> = {
  json: '.json',
  csv: '.csv',
  backup: '.pastedbackup',
};

const EXPORT_FORMAT_LABEL: Record<VisibleExportFormat, string> = {
  json: 'JSON',
  csv: 'CSV',
  backup: 'BACKUP',
};

const EXPORT_FORMAT_DESCRIPTION: Record<VisibleExportFormat, string> = {
  json: 'Preserves rich data.',
  csv: 'Creates spreadsheet-ready records.',
  backup: 'Includes everything for recovery.',
};

const EXPORT_DATA: ReadonlyArray<{
  id: ExportDataId;
  label: string;
  description: string;
  formats: readonly VisibleExportFormat[];
  nested?: boolean;
}> = [
  { id: 'clips', get label() { return translate('component.settingsSyncPanel.clips'); }, get description() { return translate('component.settingsSyncPanel.historyRichContentNotesProtectionPinsAndCaptureDetails'); }, formats: ['json', 'csv', 'backup'] },
  { id: 'organization', get label() { return translate('component.settingsSyncPanel.organization'); }, get description() { return translate('component.settingsSyncPanel.addsTrashBinsTransformsOperationsContentTypesClassifiersAndOcr'); }, formats: ['json', 'backup'], nested: true },
  { id: 'activity', get label() { return translate('destination.activity'); }, get description() { return translate('component.settingsSyncPanel.portableAuditRecordsWithoutClipboardContentsOrActionReplay'); }, formats: ['json', 'csv', 'backup'] },
  { id: 'settings', get label() { return translate('component.settingsSyncPanel.settingsAndApplicationData'); }, get description() { return translate('component.settingsSyncPanel.settingsHotkeysAppExclusionRulesQueueStateAndConnectionConfiguration'); }, formats: ['backup'] },
  { id: 'recovery', get label() { return translate('component.settingsSyncPanel.revisionsAndAutomationHistory'); }, get description() { return translate('component.settingsSyncPanel.clipRevisionsAutomationsAndExecutionHistory'); }, formats: ['backup'] },
  { id: 'interface', get label() { return translate('component.settingsSyncPanel.interfaceAndWindowState'); }, get description() { return translate('component.settingsSyncPanel.savedLayoutNavigationAndWindowState'); }, formats: ['backup'] },
];

function ExportDataRow({
  item,
  checked,
  disabled,
  onToggle,
}: {
  item: (typeof EXPORT_DATA)[number];
  checked: boolean;
  disabled: boolean;
  onToggle: () => void;
}) {
  return <div className={`relative flex items-start justify-between gap-4 py-3 pr-4 ${item.nested ? 'pl-10' : 'pl-4'} ${disabled ? 'settings-disabled-row' : ''}`}>
    {item.nested && <span
      aria-hidden="true"
      className="theme-divider absolute left-4 top-0 h-1/2 w-3 rounded-bl-md border-b border-l"
    />}
    <div className="min-w-0">
      <div className="flex flex-wrap items-center gap-2">
        <h4 className="theme-text-main text-[11px] font-semibold">{item.label}</h4>
        <span className="flex items-center gap-1" aria-label={translate('component.settingsSyncPanel.supportedFormatsValue', { value: item.formats.map((format) => EXPORT_FORMAT_LABEL[format]).join(', ') })}>
          {item.formats.map((format) => <span
            key={format}
            className="theme-code-surface theme-label rounded border px-1.5 py-0.5 text-[8px] font-bold tracking-wide"
          >
            {EXPORT_FORMAT_LABEL[format]}
          </span>)}
        </span>
      </div>
      <p className="theme-text-muted mt-0.5 text-[10px] leading-relaxed">{item.description}</p>
    </div>
    <SettingsSwitch checked={checked} label={item.label} disabled={disabled} onClick={onToggle} />
  </div>;
}

export function SettingsSyncPanel({
  onRefreshBins,
  onRefreshPipelines,
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
  const [storageProtection, setStorageProtection] = useState<StorageProtectionInfo | null>(cachedStorageProtection);
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
      cachedStorageProtection = unavailable;
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
      const savedPath = await invoke<string | null>('export_backup_file');
      if (savedPath) showToast({ tone: 'success', get message() { return translate('component.settingsSyncPanel.historyAndOrganizationDataExportedSuccessfully'); } });
    } catch (error) {
      console.error('History and organization export failed:', error);
      showToast({ tone: 'error', get message() { return translate('component.settingsSyncPanel.historyAndOrganizationExportFailed'); } });
    }
  };

  const downloadActivityExport = async (format: 'json' | 'csv') => {
    try {
      const contents = format === 'json'
        ? await invoke<string>('export_activity_json')
        : await invoke<string>('export_activity_csv');
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
      const inspection = await invoke<ImportFileInspection | null>('choose_import_file');
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
      const report = await invoke<Record<string, number>>('import_inspected_file', {
        path: selected.path,
        kind: selected.kind,
        format: selected.format,
      });
      if (selected.kind === 'organization') {
        await Promise.all([
          Promise.resolve(onRefreshBins?.()),
          Promise.resolve(onRefreshPipelines?.()),
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

  const customPrimaryFileSelected = exportData.clips;
  const customActivityFileSelected = exportData.activity;
  const customExportFileCount = Number(customPrimaryFileSelected) + Number(customActivityFileSelected);
  const activeExportFormat: VisibleExportFormat = exportMode === 'full' ? 'backup' : exportFormat;

  const handleCreateFullBackup = async () => {
    setIsCreatingFullBackup(true);
    try {
      const report = await invoke<FullBackupReport | null>('export_full_backup_file', {
        clientStateJson: collectBackupClientState(),
      });
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
      const report = await invoke<FullRestoreReport | null>('restore_full_backup_file', {
        currentClientStateJson: collectBackupClientState(),
        backupPath: importInspection?.kind === 'backup' ? importInspection.path : undefined,
      });
      if (!report) {
        setIsRestoringFullBackup(false);
        return;
      }
      setImportInspection(null);
      showToast({
        tone: 'success',
        get message() { return translate('component.settingsSyncPanel.fullBackupRestoredRestartingWithTheRestoredState'); },
      });
      if ((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) {
        setTimeout(() => window.location.reload(), 700);
      }
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

      <section className="space-y-3" aria-labelledby="library-location-title">
        <SettingsSubsectionHeader
          id="library-location-title"
          title={translate('component.settingsSyncPanel.databaseLocation')}
          description={translate('component.settingsSyncPanel.chooseWhereEverythingIsStored')}
          actions={<div className="flex shrink-0 items-center gap-2">
            {location && !location.isDefault && (
              <ActionButton
                onClick={() => void handleRestoreDefault()}
                disabled={isMoving}
                className="disabled:opacity-50"
              >
                {translate('component.settingsSyncPanel.useDefault')}
              </ActionButton>
            )}
            <ActionButton
              onClick={() => void handleMoveLibrary()}
              disabled={isMoving}
              className="disabled:opacity-50"
            >
              <FolderInput className="h-4 w-4" />
              <span>{isMoving ? translate('component.settingsSyncPanel.moving') : translate('component.settingsSyncPanel.move')}</span>
            </ActionButton>
          </div>}
        />
        <div className="theme-surface overflow-hidden rounded-xl border">
          <div className="p-3">
            <p className="theme-label text-[10px] font-bold uppercase tracking-wider">
              {location?.isDefault ? translate('component.settingsSyncPanel.defaultLocation') : translate('component.settingsSyncPanel.customLocation')}
            </p>
            <p
              className="theme-text-main mt-1 select-text truncate font-mono text-[11px]"
              title={location?.path}
            >
              {location?.path ?? translate('component.settingsSyncPanel.loadingDatabaseLocation')}
            </p>
          </div>
          <div className="theme-subtle-surface flex min-h-[4.5rem] items-start gap-3 border-t theme-divider px-3 py-3">
            {storageProtection?.status === 'protected'
              ? <ShieldCheck className="theme-status-success-text mt-0.5 h-4 w-4 shrink-0" />
              : storageProtection?.status === 'notDetected'
                ? <ShieldAlert className="theme-status-warning-text mt-0.5 h-4 w-4 shrink-0" />
                : <ShieldQuestion className="theme-text-muted mt-0.5 h-4 w-4 shrink-0" />}
            <div className="min-w-0">
              <p className="theme-label text-[9px] font-bold uppercase tracking-wider">{translate('component.settingsSyncPanel.storageProtection')}</p>
              <p className="theme-text-main mt-0.5 text-[11px] font-semibold">
                {storageProtection?.summary ?? translate('component.settingsSyncPanel.checkingVolumeEncryption')}
              </p>
              <p className="theme-text-muted mt-0.5 text-[10px] leading-relaxed">
                {storageProtection?.detail ?? translate('component.settingsSyncPanel.checkingTheActiveDatabaseVolume')}
              </p>
            </div>
          </div>
        </div>
      </section>

      <section className="space-y-3 border-t theme-divider pt-5" aria-labelledby="export-title">
        <SettingsSubsectionHeader
          id="export-title"
          title={translate('component.settingsSyncPanel.export')}
          description={translate('component.settingsSyncPanel.chooseWhatToIncludeAndHowToPackageIt')}
          actions={
            <div className="theme-code-surface flex shrink-0 rounded-lg border p-1" aria-label={translate('component.settingsSyncPanel.exportFormat')}>
              {(['json', 'csv', 'backup'] as const satisfies readonly VisibleExportFormat[]).map((format) => {
                const active = exportMode === 'full' ? format === 'backup' : format === exportFormat;
                return <button
                  key={format}
                  type="button"
                  aria-pressed={active}
                  onClick={() => {
                    if (format === 'backup') {
                      setExportMode('full');
                    } else {
                      chooseExportFormat(format);
                      setExportMode('custom');
                    }
                  }}
                  className={`settings-feature-preset rounded-md px-3 py-1.5 text-[10px] font-semibold uppercase ${active ? 'is-active' : ''}`}
                >
                  {EXPORT_FORMAT_LABEL[format]}
                </button>;
              })}
            </div>
          }
        />
        <div className="theme-surface overflow-hidden rounded-xl border">
          <div className="divide-y theme-divide">
            {EXPORT_DATA
              .filter((item) => item.id !== 'activity' || activityEnabled || exportMode === 'full')
              .map((item) => {
                const supported = item.formats.includes(activeExportFormat);
                const checked = exportMode === 'full' ? true : supported && exportData[item.id];
                const disabled = exportMode === 'full' || !supported;
                return <ExportDataRow key={item.id} item={item} checked={checked} disabled={disabled} onToggle={() => toggleExportData(item.id)} />;
              })}
          </div>
          <div className="theme-subtle-surface flex items-start justify-between gap-4 border-t px-4 py-3">
            <div className="min-w-0">
              <p className="theme-text-muted text-[10px] leading-relaxed">
                {exportMode === 'full'
                  ? translate('component.settingsSyncPanel.fullBackupFileSummary', { extension: '.pastedbackup' })
                  : translate('component.settingsSyncPanel.exportFileSummary', { count: customExportFileCount, extension: EXPORT_EXTENSION[exportFormat] })}
              </p>
              <dl className="mt-2 grid grid-cols-[5rem_minmax(0,1fr)] gap-x-2 gap-y-1 text-[9px] leading-relaxed">
                <dt className="theme-label font-semibold">{translate('component.settingsSyncPanel.scope')}</dt>
                <dd className="theme-text-muted">{EXPORT_FORMAT_DESCRIPTION[activeExportFormat]}</dd>
                <dt className="theme-label font-semibold">{translate('component.settingsSyncPanel.originalFiles')}</dt>
                <dd className="theme-text-muted">{translate('component.settingsSyncPanel.remainInTheirCurrentLocations')}</dd>
                <dt className="theme-label font-semibold">{translate('component.settingsSyncPanel.credentials')}</dt>
                <dd className="theme-text-muted">{translate('component.settingsSyncPanel.areNotCopied')}</dd>
                <dt className="theme-label font-semibold">{translate('component.settingsSyncPanel.encryption')}</dt>
                <dd className="theme-text-muted">{translate('component.settingsSyncPanel.none')}</dd>
              </dl>
            </div>
            <ActionButton
              variant="primary"
              onClick={() => void handleSelectedExport()}
              disabled={isCreatingFullBackup || (exportMode === 'custom' && customExportFileCount === 0)}
              className="shrink-0 disabled:opacity-50"
            >
              <Download className="h-4 w-4" />
              <span>{isCreatingFullBackup ? translate('component.settingsSyncPanel.exporting') : translate('component.settingsSyncPanel.export2')}</span>
            </ActionButton>
          </div>
        </div>
      </section>

      <section className="space-y-3 border-t theme-divider pt-5" aria-labelledby="import-title">
        <SettingsSubsectionHeader
          id="import-title"
          title={translate('component.settingsSyncPanel.import')}
          description={translate('component.settingsSyncPanel.chooseAFileToInspectBeforeAnythingChanges')}
          actions={
            <ActionButton onClick={() => void handleChooseImportFile()} disabled={isInspectingImport || isImporting || isRestoringFullBackup} className="disabled:opacity-50">
              <Upload className="h-4 w-4" />
              {importInspection ? translate('component.settingsSyncPanel.chooseAnother') : translate('component.settingsSyncPanel.chooseFile')}
            </ActionButton>
          }
        />
        <div className="theme-surface overflow-hidden rounded-xl border">
          {isInspectingImport ? (
            <div className="flex min-h-24 items-center gap-3 px-4 py-5" role="status">
              <LoaderCircle className="theme-text-muted h-5 w-5 shrink-0 animate-spin" />
              <div>
                <h4 className="theme-text-main text-[11px] font-semibold">{translate('component.settingsSyncPanel.checkingFile')}</h4>
                <p className="theme-text-muted mt-0.5 text-[10px] leading-relaxed">{translate('component.settingsSyncPanel.identifyingTheDataAndValidatingItsContents')}</p>
              </div>
            </div>
          ) : importInspectionError ? (
            <div className="flex min-h-24 items-center gap-3 px-4 py-4">
              <FileWarning className="theme-status-danger-text h-5 w-5 shrink-0" />
              <div className="min-w-0">
                <h4 className="theme-text-main text-[11px] font-semibold">{translate('component.settingsSyncPanel.thisFileCannotBeUsed')}</h4>
                <p className="theme-text-muted mt-1 break-words text-[10px] leading-relaxed">{importInspectionError}</p>
              </div>
            </div>
          ) : importInspection ? (
            <>
              <div className="flex items-start justify-between gap-4 px-4 py-3">
                <div className="flex min-w-0 items-start gap-3">
                  <CheckCircle2 className="theme-status-success-text mt-0.5 h-5 w-5 shrink-0" />
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-1.5">
                      <h4 className="theme-text-main max-w-full truncate text-[11px] font-semibold" title={importInspection.name}>{importInspection.name}</h4>
                      <span className="theme-code-surface theme-label rounded border px-1.5 py-0.5 text-[8px] font-bold uppercase tracking-wide">{translate('component.settingsSyncPanel.valid')}</span>
                      <span className="theme-code-surface theme-label rounded border px-1.5 py-0.5 text-[8px] font-bold uppercase tracking-wide">{importInspection.format === 'backup' ? translate('component.settingsSyncPanel.backup') : importInspection.format}</span>
                    </div>
                    <p className="theme-text-muted mt-1 text-[10px] leading-relaxed">
                      {importInspection.kind === 'clips' && translate('component.settingsSyncPanel.clips')}
                      {importInspection.kind === 'activity' && translate('destination.activity')}
                      {importInspection.kind === 'organization' && translate('component.settingsSyncPanel.historyAndOrganization')}
                      {importInspection.kind === 'backup' && translate('component.settingsSyncPanel.completeRecoveryBackup')}
                      {' · '}{(importInspection.sizeBytes / 1024 < 1024
                        ? translate('component.settingsSyncPanel.valueKb', { value: formatNumber(Math.max(1, Math.round(importInspection.sizeBytes / 1024))) })
                        : translate('component.settingsSyncPanel.valueMb', { value: (importInspection.sizeBytes / 1024 / 1024).toFixed(1) }))}
                    </p>
                  </div>
                </div>
                <button
                  type="button"
                  className="theme-icon-button theme-focusable shrink-0 rounded-lg border p-1.5"
                  onClick={() => setImportInspection(null)}
                  aria-label={translate('component.settingsSyncPanel.removeSelectedFile')}
                  title={translate('component.settingsSyncPanel.removeSelectedFile')}
                >
                  <X className="h-3.5 w-3.5" />
                </button>
              </div>
              <div className="border-t theme-divider px-4 py-3">
                <p className="theme-label text-[9px] font-semibold uppercase tracking-wider">{translate('component.settingsSyncPanel.contents')}</p>
                <p className="theme-text-main mt-1 text-[10px] leading-relaxed">
                  {importInspection.report && (importInspection.kind === 'activity'
                    ? translate('component.settingsSyncPanel.activityInspectionSummary', {
                      scannedCount: importInspection.report.scannedCount,
                      importedCount: importInspection.report.importedCount,
                      duplicateCount: importInspection.report.duplicateCount,
                    })
                    : translate('component.settingsSyncPanel.clipInspectionSummary', {
                      scannedCount: importInspection.report.scannedCount,
                      importedCount: importInspection.report.importedCount,
                      duplicateCount: importInspection.report.duplicateCount,
                    }))}
                  {importInspection.library && translate('component.settingsSyncPanel.valueClipsValue2BinsValue3TransformsValue4Operations', { value: formatNumber(importInspection.library.clipCount), value2: formatNumber(importInspection.library.binCount), value3: formatNumber(importInspection.library.transformCount), value4: formatNumber(importInspection.library.operationCount) })}
                  {importInspection.backup && translate('component.settingsSyncPanel.createdValueFormatVersionFormatversion', { value: formatDateTime(importInspection.backup.createdAt), formatVersion: importInspection.backup.formatVersion })}
                </p>
              </div>
              <div className="theme-subtle-surface flex items-start justify-between gap-4 border-t px-4 py-3">
                <div className="min-w-0">
                  <h4 className="theme-text-main text-[11px] font-semibold">{importInspection.kind === 'backup' ? translate('component.settingsSyncPanel.recovery') : translate('component.settingsSyncPanel.merge')}</h4>
                  <p className="theme-text-muted mt-0.5 text-[10px] leading-relaxed">
                    {importInspection.kind === 'clips' && translate('component.settingsSyncPanel.addsNewClipsSkipsExistingMatchesAndKeepsUnrelatedData')}
                    {importInspection.kind === 'activity' && translate('component.settingsSyncPanel.addsInertActivityHistorySkipsDuplicatesAndNeverReplaysRecordedActions')}
                    {importInspection.kind === 'organization' && translate('component.settingsSyncPanel.updatesRecognizableMatchesAddsNewDataAndKeepsUnrelatedData')}
                    {importInspection.kind === 'backup' && translate('component.settingsSyncPanel.replacesTheCurrentStateAfterCreatingACompleteRecoveryBackup')}
                  </p>
                </div>
                <ActionButton
                  variant={importInspection.kind === 'backup' ? 'danger' : 'primary'}
                  onClick={() => importInspection.kind === 'backup' ? setIsRestoreConfirmOpen(true) : void handleMergeImport()}
                  disabled={isImporting || isRestoringFullBackup}
                  className="shrink-0 disabled:opacity-50"
                >
                  {importInspection.kind === 'backup' ? <RotateCcw className="h-4 w-4" /> : <Upload className="h-4 w-4" />}
                  <span>{importInspection.kind === 'backup' ? translate('component.settingsSyncPanel.recover2') : translate('component.settingsSyncPanel.merge')}</span>
                </ActionButton>
              </div>
            </>
          ) : (
            <div className="flex min-h-24 items-center gap-3 px-4 py-5">
              <Upload className="theme-text-muted h-5 w-5 shrink-0" />
              <div>
                <h4 className="theme-text-main text-[11px] font-semibold">{translate('component.settingsSyncPanel.noFileSelected')}</h4>
                <p className="theme-text-muted mt-0.5 text-[10px] leading-relaxed">{translate('component.settingsSyncPanel.supportedImportFiles', { backupExtension: '.pastedbackup' })}</p>
              </div>
            </div>
          )}
        </div>
      </section>

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
          className="theme-secondary-button flex w-full items-center justify-between rounded-xl border px-4 py-3 text-left"
        >
          <span>
            <strong className="theme-title block text-xs">{translate('component.settingsSyncPanel.curiousWhatSTakingUpSpace')}</strong>
            <span className="theme-text-muted mt-0.5 block text-[11px]">{translate('component.settingsSyncPanel.openInsights')}</span>
          </span>
          <ArrowRight className="h-4 w-4 shrink-0" />
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
