import { useState } from 'react';
import { Cloud, Download, ShieldCheck, Upload } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';

const MAX_BACKUP_IMPORT_BYTES = 256 * 1024 * 1024;

interface SettingsSyncPanelProps {
  onRefreshBins?: () => void;
  onRefreshPipelines?: () => void;
  onRefreshClips?: () => void;
}

export function SettingsSyncPanel({
  onRefreshBins,
  onRefreshPipelines,
  onRefreshClips,
}: SettingsSyncPanelProps) {
  const [status, setStatus] = useState<{ kind: 'success' | 'error'; message: string } | null>(null);

  const handleExport = async () => {
    try {
      const json = await invoke<string>('export_backup_json');
      const url = URL.createObjectURL(new Blob([json], { type: 'application/json' }));
      const link = document.createElement('a');
      link.href = url;
      link.download = `Pasted_Backup_${new Date().toISOString().slice(0, 10)}.json`;
      link.click();
      URL.revokeObjectURL(url);
      setStatus({ kind: 'success', message: 'Backup exported successfully.' });
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
    try {
      const importedCount = await invoke<number>('import_backup_json', { jsonStr: await file.text() });
      onRefreshBins?.();
      onRefreshPipelines?.();
      onRefreshClips?.();
      setStatus({ kind: 'success', message: `Imported ${importedCount} items from backup.` });
    } catch (error) {
      console.error('Backup import failed:', error);
      setStatus({ kind: 'error', message: 'Backup import failed. Check that the file is a valid Pasted backup.' });
    }
  };

  return (
    <div className="space-y-5 text-xs">
      <div className="p-5 theme-surface rounded-xl border space-y-3">
        <div className="flex items-center space-x-3">
          <div className="settings-accent-tile p-2.5 rounded-xl border">
            <Download className="w-5 h-5" />
          </div>
          <div>
            <h4 className="text-sm font-bold theme-title">Backup &amp; Restore Vault (.json)</h4>
            <p className="text-[11px] theme-text-muted">Export clips, Trash, Bins, Tags, Transforms, and Advanced tools to a JSON file or restore from a backup.</p>
          </div>
        </div>

        <div className="flex items-center space-x-3 pt-1">
          <button
            type="button"
            onClick={handleExport}
            className="theme-primary-button flex items-center space-x-2 px-4 py-2 border font-semibold rounded-xl text-xs transition-[background-color,transform] shadow-md active:scale-95 cursor-pointer"
          >
            <Download className="w-4 h-4" />
            <span>Export Backup (.json)</span>
          </button>

          <label className="theme-secondary-button flex items-center space-x-2 px-4 py-2 font-semibold rounded-xl text-xs transition-[background-color,border-color,color] border shadow-md cursor-pointer">
            <Upload className="w-4 h-4 theme-text-muted" />
            <span>Import Backup (.json)</span>
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

        {status && (
          <p
            role={status.kind === 'error' ? 'alert' : 'status'}
            className={`rounded-lg border px-3 py-2 text-[11px] ${
              status.kind === 'error'
                ? 'theme-status-danger'
                : 'theme-status-success'
            }`}
          >
            {status.message}
          </p>
        )}
      </div>

      <div className="p-5 theme-surface rounded-xl border space-y-4">
        <div className="flex items-center space-x-3">
          <div className="settings-accent-tile p-2.5 rounded-xl border">
            <Cloud className="w-6 h-6" />
          </div>
          <div>
            <h4 className="text-sm font-bold theme-title">iCloud Sync Coming Soon</h4>
            <span className="theme-status-success text-[10px] px-2 py-0.5 rounded-full font-mono border">
              Offline Local Storage Active
            </span>
          </div>
        </div>

        <p className="text-xs theme-text-muted leading-relaxed">
          Your clipboard history, notes, Bins, and Transforms are saved <strong>100% locally and securely</strong> on this device inside your private SQLite database.
        </p>

        <div className="theme-subtle-surface p-3 rounded-lg border space-y-1.5 text-[11px] theme-text-muted">
          <div className="flex items-center space-x-2 theme-text-main">
            <ShieldCheck className="w-4 h-4 theme-status-success-text" />
            <span className="font-semibold theme-title">Local Privacy &amp; Safety First</span>
          </div>
          <p className="pl-6">No data ever leaves your computer. CloudKit cross-device synchronization will be enabled in an upcoming release.</p>
        </div>
      </div>
    </div>
  );
}
