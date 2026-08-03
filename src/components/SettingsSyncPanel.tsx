import { useState } from 'react';
import { Cloud, Download, ShieldCheck, Upload } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';

interface SettingsSyncPanelProps {
  onRefreshBoards?: () => void;
  onRefreshFilters?: () => void;
  onRefreshClips?: () => void;
}

export function SettingsSyncPanel({
  onRefreshBoards,
  onRefreshFilters,
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
    try {
      const importedCount = await invoke<number>('import_backup_json', { jsonStr: await file.text() });
      onRefreshBoards?.();
      onRefreshFilters?.();
      onRefreshClips?.();
      setStatus({ kind: 'success', message: `Imported ${importedCount} items from backup.` });
    } catch (error) {
      console.error('Backup import failed:', error);
      setStatus({ kind: 'error', message: 'Backup import failed. Check that the file is a valid Pasted backup.' });
    }
  };

  return (
    <div className="bg-[#212121] p-6 rounded-2xl border border-gray-700/80 shadow-2xl space-y-5 text-xs text-gray-200">
      <div className="p-5 theme-surface bg-[#181818] rounded-xl border border-gray-700/80 space-y-3">
        <div className="flex items-center space-x-3">
          <div className="p-2.5 rounded-xl bg-purple-500/10 border border-purple-500/20 text-purple-400">
            <Download className="w-5 h-5" />
          </div>
          <div>
            <h4 className="text-sm font-bold theme-title">Backup &amp; Restore Vault (.json)</h4>
            <p className="text-[11px] theme-text-muted">Export all clips, Trash, Bins, Tags, Filters, and Operations to a JSON file or restore from a backup.</p>
          </div>
        </div>

        <div className="flex items-center space-x-3 pt-1">
          <button
            type="button"
            onClick={handleExport}
            className="flex items-center space-x-2 px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white font-semibold rounded-xl text-xs transition-all shadow-md active:scale-95 cursor-pointer"
          >
            <Download className="w-4 h-4" />
            <span>Export Backup (.json)</span>
          </button>

          <label className="flex items-center space-x-2 px-4 py-2 bg-gray-800 hover:bg-gray-700 text-gray-200 font-semibold rounded-xl text-xs transition-all border border-gray-700 shadow-md cursor-pointer">
            <Upload className="w-4 h-4 text-gray-400" />
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
                ? 'border-red-500/30 bg-red-500/10 text-red-400'
                : 'border-emerald-500/30 bg-emerald-500/10 text-emerald-400'
            }`}
          >
            {status.message}
          </p>
        )}
      </div>

      <div className="p-5 theme-surface bg-[#181818] rounded-xl border border-gray-700/80 space-y-4">
        <div className="flex items-center space-x-3">
          <div className="p-2.5 rounded-xl bg-blue-500/10 border border-blue-500/20 text-blue-400">
            <Cloud className="w-6 h-6" />
          </div>
          <div>
            <h4 className="text-sm font-bold theme-title">iCloud Sync Coming Soon</h4>
            <span className="text-[10px] px-2 py-0.5 rounded-full bg-emerald-500/15 text-emerald-400 font-mono border border-emerald-500/20">
              Offline Local Storage Active
            </span>
          </div>
        </div>

        <p className="text-xs theme-text-muted leading-relaxed">
          All your clipboard history items, custom notes, smart bins, and filter pipelines are saved <strong>100% locally and securely</strong> on this device inside your private SQLite database.
        </p>

        <div className="p-3 bg-gray-800/40 rounded-lg border border-gray-700/50 space-y-1.5 text-[11px] theme-text-muted">
          <div className="flex items-center space-x-2 text-gray-300">
            <ShieldCheck className="w-4 h-4 text-emerald-400" />
            <span className="font-semibold theme-title">Local Privacy &amp; Safety First</span>
          </div>
          <p className="pl-6">No data ever leaves your computer. CloudKit cross-device synchronization will be enabled in an upcoming release.</p>
        </div>
      </div>
    </div>
  );
}
