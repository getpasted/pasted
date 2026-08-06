import { useState } from 'react';
import { Download, Upload } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import {
  LibraryTransitionDialog,
  waitForMinimumLibraryTransition,
} from './LibraryTransitionDialog';

const MAX_BACKUP_IMPORT_BYTES = 256 * 1024 * 1024;

interface SettingsSyncPanelProps {
  onRefreshBins?: () => void;
  onRefreshPipelines?: () => void;
  onRefreshClips?: () => void;
  onRefreshTrashedClips?: () => void;
}

export function SettingsSyncPanel({
  onRefreshBins,
  onRefreshPipelines,
  onRefreshClips,
  onRefreshTrashedClips,
}: SettingsSyncPanelProps) {
  const [status, setStatus] = useState<{ kind: 'success' | 'error'; message: string } | null>(null);
  const [isImporting, setIsImporting] = useState(false);

  const handleExport = async () => {
    try {
      const savedPath = await invoke<string | null>('export_backup_file');
      if (savedPath) setStatus({ kind: 'success', message: 'Backup saved successfully.' });
    } catch (error) {
      console.error('Backup export failed:', error);
      setStatus({ kind: 'error', message: 'Backup export failed.' });
    }
  };

  const handleImport = async (file: File) => {
    if (file.size > MAX_BACKUP_IMPORT_BYTES) {
      setStatus({ kind: 'error', message: 'Backup exceeds Pasted’s 256 MB safety limit.' });
      return;
    }
    const transitionStartedAt = performance.now();
    setIsImporting(true);
    setStatus(null);
    try {
      const importedCount = await invoke<number>('import_backup_json', { jsonStr: await file.text() });
      await Promise.all([
        Promise.resolve(onRefreshBins?.()),
        Promise.resolve(onRefreshPipelines?.()),
        Promise.resolve(onRefreshClips?.()),
        Promise.resolve(onRefreshTrashedClips?.()),
      ]);
      await waitForMinimumLibraryTransition(transitionStartedAt);
      setStatus({ kind: 'success', message: `Backup imported. Processed ${importedCount} clips.` });
    } catch (error) {
      console.error('Backup import failed:', error);
      await waitForMinimumLibraryTransition(transitionStartedAt);
      setStatus({ kind: 'error', message: 'Backup import failed. Check that the file is a valid Pasted backup.' });
    } finally {
      setIsImporting(false);
    }
  };

  return (
    <div className="space-y-5 text-xs">
      <SettingsPanelHeader
        icon={Download}
        title="Backup & Import"
        description="Export or merge a backup."
        actions={(
          <>
            <button
              type="button"
              onClick={handleExport}
              className="theme-primary-button flex items-center space-x-1.5 px-3 py-2 border font-semibold rounded-xl text-xs cursor-pointer"
            >
              <Download className="w-4 h-4" />
              <span>Export</span>
            </button>
            <label className="theme-secondary-button flex items-center space-x-1.5 px-3 py-2 font-semibold rounded-xl text-xs border cursor-pointer">
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
          </>
        )}
      />

      {status && (
        <p
          role={status.kind === 'error' ? 'alert' : 'status'}
          className={`rounded-lg border px-3 py-2 text-[11px] ${
            status.kind === 'error' ? 'theme-status-danger' : 'theme-status-success'
          }`}
        >
          {status.message}
        </p>
      )}

      <div className="grid gap-4 theme-surface rounded-xl border p-4 sm:grid-cols-2">
        <div className="flex items-start gap-3">
          <div className="settings-accent-tile shrink-0 rounded-lg border p-2">
            <Download className="h-4 w-4" />
          </div>
          <div className="min-w-0 pt-0.5">
            <h4 className="text-sm font-bold theme-title">Export</h4>
            <p className="mt-1 text-[11px] theme-text-muted leading-relaxed">
              Creates one portable JSON backup file.
            </p>
      </div>

      <LibraryTransitionDialog
        isOpen={isImporting}
        variant="import"
        title="Importing Backup"
        description="Gathering clips, Bins, and Transforms into this library…"
      />
    </div>
        <div className="flex items-start gap-3">
          <div className="settings-accent-tile shrink-0 rounded-lg border p-2">
            <Upload className="h-4 w-4" />
          </div>
          <div className="min-w-0 pt-0.5">
            <h4 className="text-sm font-bold theme-title">Import</h4>
            <p className="mt-1 text-[11px] theme-text-muted leading-relaxed">
              Merges into this library. Matching items update, new items are added, and unrelated items remain.
            </p>
          </div>
        </div>
      </div>

    </div>
  );
}
