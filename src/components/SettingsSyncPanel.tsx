import { useEffect, useState } from 'react';
import { ArrowRight, CheckCircle2, Database, Download, FileWarning, FolderInput, LoaderCircle, RotateCcw, Upload, X } from 'lucide-react';
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
  { id: 'clips', label: 'Clips', description: 'History, rich content, notes, protection, pins, and capture details.', formats: ['json', 'csv', 'backup'] },
  { id: 'organization', label: 'Organization', description: 'Adds Trash, Bins, Transforms, Operations, Content Types, Classifiers, and OCR.', formats: ['json', 'backup'], nested: true },
  { id: 'activity', label: 'Activity', description: 'Portable audit records without clipboard contents or action replay.', formats: ['json', 'csv', 'backup'] },
  { id: 'settings', label: 'Settings and Application Data', description: 'Settings, hotkeys, blacklist rules, Queue state, and connection configuration.', formats: ['backup'] },
  { id: 'recovery', label: 'Revisions and Automation History', description: 'Clip revisions, automations, and execution history.', formats: ['backup'] },
  { id: 'interface', label: 'Interface and Window State', description: 'Saved layout, navigation, and window state.', formats: ['backup'] },
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
        <span className="flex items-center gap-1" aria-label={`Supported formats: ${item.formats.map((format) => EXPORT_FORMAT_LABEL[format]).join(', ')}`}>
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

  useEffect(() => {
    void refreshLocation();
  }, []);

  const handleMoveLibrary = async () => {
    const transitionStartedAt = performance.now();
    setIsMoving(true);
    try {
      const report = await invoke<LibraryMoveReport | null>('move_library');
      if (!report) return;
      setLocation(report.location);
      await waitForMinimumLibraryTransition(transitionStartedAt);
      showToast({
        tone: 'success',
        message: 'Database moved. The previous database was kept as a recovery copy.',
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
      await waitForMinimumLibraryTransition(transitionStartedAt);
      showToast({
        tone: 'success',
        message: 'Database returned to its default location. The custom database was kept as a recovery copy.',
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
      if (savedPath) showToast({ tone: 'success', message: 'History and organization data exported successfully.' });
    } catch (error) {
      console.error('History and organization export failed:', error);
      showToast({ tone: 'error', message: 'History and organization export failed.' });
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
      showToast({ tone: 'success', message: `${format.toUpperCase()} Activity export downloaded.` });
    } catch (error) {
      console.error(`Failed to export Activity as ${format}:`, error);
      showToast({ tone: 'error', message: `${format.toUpperCase()} Activity export failed.` });
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
      showToast({ tone: 'success', message: `${format.toUpperCase()} clip export downloaded.` });
    } catch (error) {
      console.error(`Failed to export clips as ${format}:`, error);
      showToast({ tone: 'error', message: `${format.toUpperCase()} clip export failed.` });
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
          ? `History and organization merged. Processed ${importedCount} clips.`
          : `Merged ${importedCount} ${selected.kind === 'activity' ? 'Activity entries' : 'clips'}; skipped ${duplicateCount} duplicates.`,
      });
      setImportInspection(null);
    } catch (error) {
      console.error('Import failed after inspection:', error);
      showToast({ tone: 'error', message: `The selected file could not be merged: ${String(error)}`, durationMs: 8000 });
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
      if (report) showToast({ tone: 'success', message: 'Full backup created successfully.' });
    } catch (error) {
      console.error('Full backup creation failed:', error);
      showToast({ tone: 'error', message: `Full backup creation failed: ${String(error)}`, durationMs: 8000 });
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
        message: 'Full backup restored. Restarting with the restored state…',
      });
      if ((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) {
        setTimeout(() => window.location.reload(), 700);
      }
      setIsRestoringFullBackup(false);
    } catch (error) {
      console.error('Full backup restore failed:', error);
      showToast({ tone: 'error', message: `Full backup restore failed: ${String(error)}`, durationMs: 8000 });
      setIsRestoringFullBackup(false);
    }
  };

  return (
    <div className="space-y-5 text-xs">
      <SettingsPanelHeader
        icon={Database}
        title="Storage"
        description="Manage recovery and data transfers."
      />

      <section className="space-y-3" aria-labelledby="library-location-title">
        <SettingsSubsectionHeader
          id="library-location-title"
          title="Database Location"
          description="Choose where everything is stored."
          actions={<div className="flex shrink-0 items-center gap-2">
            {location && !location.isDefault && (
              <ActionButton
                onClick={() => void handleRestoreDefault()}
                disabled={isMoving}
                className="disabled:opacity-50"
              >
                Use Default
              </ActionButton>
            )}
            <ActionButton
              onClick={() => void handleMoveLibrary()}
              disabled={isMoving}
              className="disabled:opacity-50"
            >
              <FolderInput className="h-4 w-4" />
              <span>{isMoving ? 'Moving…' : 'Move…'}</span>
            </ActionButton>
          </div>}
        />
        <div className="theme-surface rounded-xl border p-3">
          <p className="theme-label text-[10px] font-bold uppercase tracking-wider">
            {location?.isDefault ? 'Default Location' : 'Custom Location'}
          </p>
          <p
            className="theme-text-main mt-1 select-text truncate font-mono text-[11px]"
            title={location?.path}
          >
            {location?.path ?? 'Loading database location…'}
          </p>
        </div>
      </section>

      <section className="space-y-3 border-t theme-divider pt-5" aria-labelledby="export-title">
        <SettingsSubsectionHeader
          id="export-title"
          title="Export"
          description="Choose what to include and how to package it."
          actions={
            <div className="theme-code-surface flex shrink-0 rounded-lg border p-1" aria-label="Export format">
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
                  ? <>1 <code className="theme-code-surface theme-text-main rounded px-1 font-mono">.pastedbackup</code> file will be created.</>
                  : <>{customExportFileCount} <code className="theme-code-surface theme-text-main rounded px-1 font-mono">{EXPORT_EXTENSION[exportFormat]}</code> {customExportFileCount === 1 ? 'file' : 'files'} will be created.</>}
              </p>
              <dl className="mt-2 grid grid-cols-[5rem_minmax(0,1fr)] gap-x-2 gap-y-1 text-[9px] leading-relaxed">
                <dt className="theme-label font-semibold">Scope</dt>
                <dd className="theme-text-muted">{EXPORT_FORMAT_DESCRIPTION[activeExportFormat]}</dd>
                <dt className="theme-label font-semibold">Original files</dt>
                <dd className="theme-text-muted">Remain in their current locations</dd>
                <dt className="theme-label font-semibold">Credentials</dt>
                <dd className="theme-text-muted">Are not copied</dd>
              </dl>
            </div>
            <ActionButton
              variant="primary"
              onClick={() => void handleSelectedExport()}
              disabled={isCreatingFullBackup || (exportMode === 'custom' && customExportFileCount === 0)}
              className="shrink-0 disabled:opacity-50"
            >
              <Download className="h-4 w-4" />
              <span>{isCreatingFullBackup ? 'Exporting…' : 'Export…'}</span>
            </ActionButton>
          </div>
        </div>
      </section>

      <section className="space-y-3 border-t theme-divider pt-5" aria-labelledby="import-title">
        <SettingsSubsectionHeader
          id="import-title"
          title="Import"
          description="Choose a file to inspect before anything changes."
          actions={
            <ActionButton onClick={() => void handleChooseImportFile()} disabled={isInspectingImport || isImporting || isRestoringFullBackup} className="disabled:opacity-50">
              <Upload className="h-4 w-4" />
              {importInspection ? 'Choose Another…' : 'Choose File…'}
            </ActionButton>
          }
        />
        <div className="theme-surface overflow-hidden rounded-xl border">
          {isInspectingImport ? (
            <div className="flex min-h-24 items-center gap-3 px-4 py-5" role="status">
              <LoaderCircle className="theme-text-muted h-5 w-5 shrink-0 animate-spin" />
              <div>
                <h4 className="theme-text-main text-[11px] font-semibold">Checking file…</h4>
                <p className="theme-text-muted mt-0.5 text-[10px] leading-relaxed">Identifying the data and validating its contents.</p>
              </div>
            </div>
          ) : importInspectionError ? (
            <div className="flex min-h-24 items-center gap-3 px-4 py-4">
              <FileWarning className="theme-status-danger-text h-5 w-5 shrink-0" />
              <div className="min-w-0">
                <h4 className="theme-text-main text-[11px] font-semibold">This file cannot be used</h4>
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
                      <span className="theme-code-surface theme-label rounded border px-1.5 py-0.5 text-[8px] font-bold uppercase tracking-wide">Valid</span>
                      <span className="theme-code-surface theme-label rounded border px-1.5 py-0.5 text-[8px] font-bold uppercase tracking-wide">{importInspection.format === 'backup' ? 'Backup' : importInspection.format}</span>
                    </div>
                    <p className="theme-text-muted mt-1 text-[10px] leading-relaxed">
                      {importInspection.kind === 'clips' && 'Clips'}
                      {importInspection.kind === 'activity' && 'Activity'}
                      {importInspection.kind === 'organization' && 'History and Organization'}
                      {importInspection.kind === 'backup' && 'Complete recovery backup'}
                      {' · '}{(importInspection.sizeBytes / 1024 < 1024
                        ? `${Math.max(1, Math.round(importInspection.sizeBytes / 1024)).toLocaleString()} KB`
                        : `${(importInspection.sizeBytes / 1024 / 1024).toFixed(1)} MB`)}
                    </p>
                  </div>
                </div>
                <button
                  type="button"
                  className="theme-icon-button theme-focusable shrink-0 rounded-lg border p-1.5"
                  onClick={() => setImportInspection(null)}
                  aria-label="Remove selected file"
                  title="Remove selected file"
                >
                  <X className="h-3.5 w-3.5" />
                </button>
              </div>
              <div className="border-t theme-divider px-4 py-3">
                <p className="theme-label text-[9px] font-semibold uppercase tracking-wider">Contents</p>
                <p className="theme-text-main mt-1 text-[10px] leading-relaxed">
                  {importInspection.report && `${importInspection.report.scannedCount.toLocaleString()} ${importInspection.kind === 'activity' ? 'Activity entries' : 'clips'} checked · ${importInspection.report.importedCount.toLocaleString()} new · ${importInspection.report.duplicateCount.toLocaleString()} duplicates`}
                  {importInspection.library && `${importInspection.library.clipCount.toLocaleString()} clips · ${importInspection.library.binCount.toLocaleString()} Bins · ${importInspection.library.transformCount.toLocaleString()} Transforms · ${importInspection.library.operationCount.toLocaleString()} Operations`}
                  {importInspection.backup && `Created ${new Date(importInspection.backup.createdAt).toLocaleString()} · Format version ${importInspection.backup.formatVersion}`}
                </p>
              </div>
              <div className="theme-subtle-surface flex items-start justify-between gap-4 border-t px-4 py-3">
                <div className="min-w-0">
                  <h4 className="theme-text-main text-[11px] font-semibold">{importInspection.kind === 'backup' ? 'Recovery' : 'Merge'}</h4>
                  <p className="theme-text-muted mt-0.5 text-[10px] leading-relaxed">
                    {importInspection.kind === 'clips' && 'Adds new clips, skips existing matches, and keeps unrelated data.'}
                    {importInspection.kind === 'activity' && 'Adds inert Activity history, skips duplicates, and never replays recorded actions.'}
                    {importInspection.kind === 'organization' && 'Updates recognizable matches, adds new data, and keeps unrelated data.'}
                    {importInspection.kind === 'backup' && 'Replaces the current state after creating a complete recovery backup.'}
                  </p>
                </div>
                <ActionButton
                  variant={importInspection.kind === 'backup' ? 'danger' : 'primary'}
                  onClick={() => importInspection.kind === 'backup' ? setIsRestoreConfirmOpen(true) : void handleMergeImport()}
                  disabled={isImporting || isRestoringFullBackup}
                  className="shrink-0 disabled:opacity-50"
                >
                  {importInspection.kind === 'backup' ? <RotateCcw className="h-4 w-4" /> : <Upload className="h-4 w-4" />}
                  <span>{importInspection.kind === 'backup' ? 'Recover…' : 'Merge'}</span>
                </ActionButton>
              </div>
            </>
          ) : (
            <div className="flex min-h-24 items-center gap-3 px-4 py-5">
              <Upload className="theme-text-muted h-5 w-5 shrink-0" />
              <div>
                <h4 className="theme-text-main text-[11px] font-semibold">No file selected</h4>
                <p className="theme-text-muted mt-0.5 text-[10px] leading-relaxed">JSON, CSV, and <code className="theme-code-surface theme-text-main rounded px-1 font-mono">.pastedbackup</code> files are supported.</p>
              </div>
            </div>
          )}
        </div>
      </section>

      <section className="space-y-3 border-t theme-divider pt-5" aria-labelledby="migration-title">
        <SettingsSubsectionHeader
          id="migration-title"
          title="Move from another clipboard manager"
          description="Import supported history without changing the source clipboard manager."
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
            <strong className="theme-title block text-xs">Curious what’s taking up space?</strong>
            <span className="theme-text-muted mt-0.5 block text-[11px]">Open Insights</span>
          </span>
          <ArrowRight className="h-4 w-4 shrink-0" />
        </button>
      )}

      <LibraryTransitionDialog
        isOpen={isImporting || isRestoringFullBackup}
        variant="import"
        title={isRestoringFullBackup ? 'Recovering from backup' : 'Merging selected data'}
        description={isRestoringFullBackup ? 'Validating and replacing the complete local state…' : 'Validating again and adding the selected records…'}
      />
      <LibraryTransitionDialog
        isOpen={isMoving}
        variant="move"
        title="Moving database"
        description="Carrying every clip, Bin, Transform, and revision to its new home…"
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
              title="Recover from Backup?"
              description={importInspection?.name ?? 'The selected backup will replace the current state.'}
              icon={<RotateCcw />}
              tone="danger"
            />
          </AppDialogHeader>
          <AppDialogBody>
            <p className="app-dialog-message theme-surface rounded-xl border p-3 text-xs leading-relaxed">
              The backup is validated before replacement. A full recovery backup of the current state is created automatically beside the active database.
            </p>
          </AppDialogBody>
          <AppDialogFooter>
            <AppDialogButton onClick={requestClose} autoFocus>Cancel</AppDialogButton>
            <AppDialogButton variant="danger" onClick={() => void handleRestoreFullBackup()}>Recover</AppDialogButton>
          </AppDialogFooter>
        </>}
      </AppDialog>
    </div>
  );
}
