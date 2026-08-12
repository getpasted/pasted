import { useEffect, useState } from 'react';
import { ArrowRight, Database, Download, FolderInput, Upload } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
import {
  LibraryTransitionDialog,
  waitForMinimumLibraryTransition,
} from './LibraryTransitionDialog';
import { useToast } from './ToastProvider';
import { ExternalHistoryImport } from './ExternalHistoryImport';
import { ActionButton } from './AppDialogLayout';
import { SettingsAccentTile } from './SettingsAccentTile';

const MAX_BACKUP_IMPORT_BYTES = 256 * 1024 * 1024;

interface SettingsSyncPanelProps {
  onRefreshBins?: () => void;
  onRefreshPipelines?: () => void;
  onRefreshClips?: () => void;
  onRefreshTrashedClips?: () => void;
  analyticsEnabled?: boolean;
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

export function SettingsSyncPanel({
  onRefreshBins,
  onRefreshPipelines,
  onRefreshClips,
  onRefreshTrashedClips,
  analyticsEnabled = false,
  onOpenAnalytics,
}: SettingsSyncPanelProps) {
  const { showToast } = useToast();
  const [isImporting, setIsImporting] = useState(false);
  const [isMoving, setIsMoving] = useState(false);
  const [location, setLocation] = useState<LibraryLocationInfo | null>(null);

  const refreshLocation = async () => {
    try {
      setLocation(await invoke<LibraryLocationInfo>('get_library_location'));
    } catch (error) {
      console.error('Could not load library location:', error);
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
        message: 'Library moved. The previous library was kept as a recovery copy.',
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
        message: 'Library returned to its default location. The custom library was kept as a recovery copy.',
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
      if (savedPath) showToast({ tone: 'success', message: 'Backup saved successfully.' });
    } catch (error) {
      console.error('Backup export failed:', error);
      showToast({ tone: 'error', message: 'Backup export failed.' });
    }
  };

  const handleImport = async (file: File) => {
    if (file.size > MAX_BACKUP_IMPORT_BYTES) {
      showToast({ tone: 'error', message: 'Backup exceeds Pasted’s 256 MB safety limit.' });
      return;
    }
    const transitionStartedAt = performance.now();
    setIsImporting(true);
    try {
      const importedCount = await invoke<number>('import_backup_json', { jsonStr: await file.text() });
      await Promise.all([
        Promise.resolve(onRefreshBins?.()),
        Promise.resolve(onRefreshPipelines?.()),
        Promise.resolve(onRefreshClips?.()),
        Promise.resolve(onRefreshTrashedClips?.()),
      ]);
      await waitForMinimumLibraryTransition(transitionStartedAt);
      showToast({ tone: 'success', message: `Backup imported. Processed ${importedCount} clips.` });
    } catch (error) {
      console.error('Backup import failed:', error);
      await waitForMinimumLibraryTransition(transitionStartedAt);
      showToast({ tone: 'error', message: 'Backup import failed. Check that the file is a valid Pasted backup.' });
    } finally {
      setIsImporting(false);
    }
  };

  return (
    <div className="space-y-6 text-xs">
      <SettingsPanelHeader
        icon={Database}
        title="Storage"
        description="Choose where Pasted lives and keep a portable backup."
      />

      <section className="space-y-3" aria-labelledby="library-location-title">
        <SettingsSubsectionHeader
          id="library-location-title"
          title="Library Location"
          description="Pasted keeps its SQLite library in this folder."
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
              <span>{isMoving ? 'Moving…' : 'Move Library…'}</span>
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
            {location?.path ?? 'Loading library location…'}
          </p>
        </div>
      </section>

      <section className="space-y-3 border-t theme-divider pt-5" aria-labelledby="backup-restore-title">
        <SettingsSubsectionHeader
          id="backup-restore-title"
          title="Backup & Restore"
          description="Export everything or merge a previous backup."
          actions={<div className="flex items-center gap-2">
            <ActionButton
              variant="primary"
              onClick={handleExport}
              className="cursor-pointer"
            >
              <Download className="w-4 h-4" />
              <span>Export</span>
            </ActionButton>
            <label className="theme-secondary-button ui-control-radius flex items-center space-x-1.5 px-3 py-2 font-semibold text-xs border cursor-pointer">
              <Upload className="w-4 h-4" />
              <span>Import</span>
              <input
                type="file"
                accept=".json,application/json"
                className="hidden"
                onChange={(event) => {
                  const file = event.target.files?.[0];
                  event.target.value = '';
                  if (file) void handleImport(file);
                }}
              />
            </label>
          </div>}
        />

        <div className="grid gap-4 theme-surface rounded-xl border p-4 sm:grid-cols-2">
          <div className="flex items-start gap-3">
            <SettingsAccentTile size="compact">
              <Download className="h-4 w-4" />
            </SettingsAccentTile>
            <div className="min-w-0 pt-0.5">
              <h4 className="text-sm font-bold theme-title">Export</h4>
              <p className="mt-1 text-[11px] theme-text-muted leading-relaxed">Creates one portable JSON backup file.</p>
            </div>
          </div>
          <div className="flex items-start gap-3">
            <SettingsAccentTile size="compact">
              <Upload className="h-4 w-4" />
            </SettingsAccentTile>
            <div className="min-w-0 pt-0.5">
              <h4 className="text-sm font-bold theme-title">Import</h4>
              <p className="mt-1 text-[11px] theme-text-muted leading-relaxed">
                Merges matching items, adds new ones, and leaves unrelated items alone.
              </p>
            </div>
          </div>
        </div>
      </section>

      <section className="space-y-3 border-t theme-divider pt-5" aria-labelledby="migration-title">
        <SettingsSubsectionHeader
          id="migration-title"
          title="Move from Another Clipboard Manager"
          description="Bring supported history into this library without changing the original app."
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
            <span className="theme-text-muted mt-0.5 block text-[11px]">View Storage Analytics</span>
          </span>
          <ArrowRight className="h-4 w-4 shrink-0" />
        </button>
      )}

      <LibraryTransitionDialog
        isOpen={isImporting}
        variant="import"
        title="Importing Backup"
        description="Gathering clips, Bins, and Transforms into this library…"
      />
      <LibraryTransitionDialog
        isOpen={isMoving}
        variant="move"
        title="Moving Library"
        description="Carrying every clip, Bin, Transform, and revision to its new home…"
      />
    </div>
  );
}
