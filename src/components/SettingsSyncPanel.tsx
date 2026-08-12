import { useEffect, useState } from 'react';
import { ArrowRight, Database, Download, FolderInput, Upload } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import {
  LibraryTransitionDialog,
  waitForMinimumLibraryTransition,
} from './LibraryTransitionDialog';
import { useToast } from './ToastProvider';

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
        <div className="flex items-center justify-between gap-4">
          <div className="min-w-0">
            <h3 id="library-location-title" className="theme-title text-sm font-bold">Library Location</h3>
            <p className="mt-1 theme-text-muted text-[11px]">Pasted keeps its SQLite library in this folder.</p>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            {location && !location.isDefault && (
              <button
                type="button"
                onClick={() => void handleRestoreDefault()}
                disabled={isMoving}
                className="theme-secondary-button ui-control-radius border px-3 py-2 font-semibold disabled:opacity-50"
              >
                Use Default
              </button>
            )}
            <button
              type="button"
              onClick={() => void handleMoveLibrary()}
              disabled={isMoving}
              className="theme-secondary-button ui-control-radius flex items-center gap-1.5 border px-3 py-2 font-semibold disabled:opacity-50"
            >
              <FolderInput className="h-4 w-4" />
              <span>{isMoving ? 'Moving…' : 'Move Library…'}</span>
            </button>
          </div>
        </div>
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
        <div className="flex items-center justify-between gap-4">
          <div>
            <h3 id="backup-restore-title" className="theme-title text-sm font-bold">Backup & Restore</h3>
            <p className="mt-1 theme-text-muted text-[11px]">Export everything or merge a previous backup.</p>
          </div>
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={handleExport}
              className="theme-primary-button ui-control-radius flex items-center space-x-1.5 px-3 py-2 border font-semibold text-xs cursor-pointer"
            >
              <Download className="w-4 h-4" />
              <span>Export</span>
            </button>
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
          </div>
        </div>

        <div className="grid gap-4 theme-surface rounded-xl border p-4 sm:grid-cols-2">
          <div className="flex items-start gap-3">
            <div className="settings-accent-tile shrink-0 rounded-lg border p-2">
              <Download className="h-4 w-4" />
            </div>
            <div className="min-w-0 pt-0.5">
              <h4 className="text-sm font-bold theme-title">Export</h4>
              <p className="mt-1 text-[11px] theme-text-muted leading-relaxed">Creates one portable JSON backup file.</p>
            </div>
          </div>
          <div className="flex items-start gap-3">
            <div className="settings-accent-tile shrink-0 rounded-lg border p-2">
              <Upload className="h-4 w-4" />
            </div>
            <div className="min-w-0 pt-0.5">
              <h4 className="text-sm font-bold theme-title">Import</h4>
              <p className="mt-1 text-[11px] theme-text-muted leading-relaxed">
                Merges matching items, adds new ones, and leaves unrelated items alone.
              </p>
            </div>
          </div>
        </div>
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
