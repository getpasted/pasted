import { useState } from 'react';
import { Coffee, Download, Laptop, Moon, RotateCcw, Snowflake, Trash2 } from 'lucide-react';
import type { AppSettings } from '../types';
import { useAltKeyPressed } from '../hooks/useAltKeyPressed';
import { safeInvoke as invoke } from '../utils/tauri';

interface SettingsGeneralPanelProps {
  settings: AppSettings;
  onUpdateSettings: (newSettings: Partial<AppSettings>) => void;
  onClearHistory?: (permanent: boolean) => void;
  onResetColumnWidths?: () => void;
}

export function SettingsGeneralPanel({
  settings,
  onUpdateSettings,
  onClearHistory,
  onResetColumnWidths,
}: SettingsGeneralPanelProps) {
  const isAltPressed = useAltKeyPressed();
  const [exportStatus, setExportStatus] = useState<string | null>(null);

  const exportClips = async (format: 'json' | 'csv') => {
    try {
      const contents = await invoke<string>(format === 'json' ? 'export_clips_json' : 'export_clips_csv');
      const url = URL.createObjectURL(new Blob([contents], {
        type: format === 'json' ? 'application/json' : 'text/csv',
      }));
      const link = document.createElement('a');
      link.href = url;
      link.download = format === 'json'
        ? `pasted_backup_${Date.now()}.json`
        : `pasted_export_${Date.now()}.csv`;
      link.click();
      URL.revokeObjectURL(url);
      setExportStatus(`${format.toUpperCase()} export downloaded.`);
    } catch (error) {
      console.error(`Failed to export clips as ${format}:`, error);
      setExportStatus(`${format.toUpperCase()} export failed.`);
    }
  };

  return (
          <div className="theme-panel p-6 rounded-2xl border space-y-6 text-xs">
            {/* General Preferences */}
            <div className="space-y-3">
              {/* Appearance Mode Switcher */}
              <div className="flex items-center justify-between pb-1">
                <span className="font-medium">Appearance:</span>
                <div className="theme-surface flex items-center p-1 rounded-lg border space-x-1">
                  <button
                    type="button"
                    onClick={() => onUpdateSettings({ themeMode: 'system' })}
                    className={`appearance-mode-button flex items-center space-x-1 px-2.5 py-1 rounded-md text-[11px] font-medium transition-all ${(settings.themeMode || 'system') === 'system' ? 'is-active' : ''}`}
                  >
                    <Laptop className="w-3.5 h-3.5" />
                    <span>System</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => onUpdateSettings({ themeMode: 'dark' })}
                    className={`appearance-mode-button flex items-center space-x-1 px-2.5 py-1 rounded-md text-[11px] font-medium transition-all ${settings.themeMode === 'dark' ? 'is-active' : ''}`}
                  >
                    <Moon className="w-3.5 h-3.5" />
                    <span>Dark</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => onUpdateSettings({ themeMode: 'cool' })}
                    className={`appearance-mode-button flex items-center space-x-1 px-2.5 py-1 rounded-md text-[11px] font-medium transition-all ${settings.themeMode === 'cool' ? 'is-active' : ''}`}
                  >
                    <Snowflake className="w-3.5 h-3.5" />
                    <span>Cool</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => onUpdateSettings({ themeMode: 'warm' })}
                    className={`appearance-mode-button flex items-center space-x-1 px-2.5 py-1 rounded-md text-[11px] font-medium transition-all ${settings.themeMode === 'warm' ? 'is-active' : ''}`}
                  >
                    <Coffee className="w-3.5 h-3.5" />
                    <span>Warm</span>
                  </button>
                </div>
              </div>
              <div className="flex items-start justify-between">
                <div className="pr-4 flex-1 min-w-0">
                  <span className="font-semibold text-gray-200 block">Text Size:</span>
                  <p className="text-[11px] text-gray-400/90 leading-normal mt-0.5">
                    Adjust font size for application text and clip content views.
                  </p>
                </div>
                <div className="flex items-center space-x-2 shrink-0">
                  <select
                    value={settings.textSize}
                    onChange={(e) => onUpdateSettings({ textSize: Number(e.target.value) })}
                    className="theme-input border rounded-md px-3 py-1 font-mono text-xs focus:outline-none focus:border-gray-500"
                  >
                    <option value={14}>14 Points (Compact)</option>
                    <option value={16}>16 Points (Standard)</option>
                    <option value={18}>18 Points (Large)</option>
                    <option value={20}>20 Points (Extra Large)</option>
                  </select>
                </div>
              </div>

              <div className="flex items-start justify-between pt-1">
                <div className="pr-4 flex-1 min-w-0">
                  <span className="font-semibold text-gray-200 block">Interaction Sounds:</span>
                  <p className="text-[11px] text-gray-400/90 leading-normal mt-0.5">
                    Play subtle audio cues for copy, paste, and navigation actions.
                  </p>
                </div>
                <label className="flex items-center space-x-2 cursor-pointer shrink-0 pt-0.5">
                  <input
                    type="checkbox"
                    checked={settings.enableSounds}
                    onChange={(e) => onUpdateSettings({ enableSounds: e.target.checked })}
                    className="w-4 h-4 accent-[#007aff] cursor-pointer rounded"
                  />
                  <span className="text-gray-200">Enable Sounds</span>
                </label>
              </div>

              <div className="flex items-start justify-between">
                <div className="pr-4 flex-1 min-w-0">
                  <span className="font-semibold text-gray-200 block">Startup Behavior:</span>
                  <p className="text-[11px] text-gray-400/90 leading-normal mt-0.5">
                    Automatically launch Pasted when logging into macOS.
                  </p>
                </div>
                <label className="flex items-center space-x-2 cursor-pointer shrink-0 pt-0.5">
                  <input
                    type="checkbox"
                    checked={settings.openAtLogin}
                    onChange={(e) => onUpdateSettings({ openAtLogin: e.target.checked })}
                    className="w-4 h-4 accent-[#007aff] cursor-pointer rounded"
                  />
                  <span className="text-gray-200">Open at login</span>
                </label>
              </div>
            </div>

            <div className="border-t border-gray-700/80" />

            {/* System & OS Integration Subsection */}
            <div className="space-y-4">
              <h4 className="font-bold theme-text-muted text-center uppercase tracking-wider text-[11px]">
                System & OS Integration
              </h4>

              {/* Dock / Menubar / System Tray Setting */}
              <div className="flex items-center justify-between pt-1">
                <span className="font-medium">
                  {typeof navigator !== 'undefined' && /Mac|iPod|iPhone|iPad/.test(navigator.userAgent || navigator.platform)
                    ? 'Dock & Menubar Icon:'
                    : 'System Tray & Taskbar:'}
                </span>
                <select
                  value={settings.dockMenubarIcon}
                  aria-label="Dock and menu bar icon behavior"
                  onChange={(e) => onUpdateSettings({ dockMenubarIcon: e.target.value as AppSettings['dockMenubarIcon'] })}
                  className="theme-input border rounded-md px-3 py-1 text-xs focus:outline-none focus:border-gray-500"
                >
                  {typeof navigator !== 'undefined' && /Mac|iPod|iPhone|iPad/.test(navigator.userAgent || navigator.platform) ? (
                    <>
                      <option value="auto_hide">Auto hide Dock Icon</option>
                      <option value="both">Always show Dock & Menubar</option>
                      <option value="menubar_only">Menubar Icon only</option>
                    </>
                  ) : (
                    <>
                      <option value="auto_hide">Auto hide Taskbar Icon</option>
                      <option value="both">Always show Tray & Taskbar</option>
                      <option value="menubar_only">System Tray Icon only</option>
                    </>
                  )}
                </select>
              </div>

              {/* macOS Only Spotlight Indexing Setting */}
              {typeof navigator !== 'undefined' && /Mac|iPod|iPhone|iPad/.test(navigator.userAgent || navigator.platform) && (
                <div className="flex items-start justify-between pt-1">
                  <div className="pr-4 flex-1 min-w-0">
                    <span className="font-semibold text-gray-200 block">Spotlight Indexing:</span>
                    <p className="text-[11px] text-gray-400/90 leading-normal mt-0.5">
                      Allow ⌘Space Spotlight to search Pasted history
                    </p>
                  </div>
                  <label className="flex items-center space-x-2 cursor-pointer shrink-0 pt-0.5">
                    <input
                      type="checkbox"
                      checked={settings.spotlightSync ?? true}
                      onChange={(e) => onUpdateSettings({ spotlightSync: e.target.checked })}
                      className="w-4 h-4 accent-[#007aff] cursor-pointer rounded"
                    />
                    <span className="text-gray-200">Index in Spotlight</span>
                  </label>
                </div>
              )}
            </div>

            <div className="border-t border-gray-700/80" />

            {/* Clipboard Preferences */}
            <div className="space-y-4">
              <h4 className="font-bold text-gray-100 text-center uppercase tracking-wider text-[11px]">
                Clipboard
              </h4>

              <div className="flex items-start justify-between pt-1">
                <div className="pr-4 flex-1 min-w-0">
                  <span className="font-semibold text-gray-200 block">Default Paste Behavior:</span>
                  <p className="text-[11px] text-gray-400/90 leading-normal mt-0.5">
                    Sets the text formatting output type.
                  </p>
                </div>
                <select
                  value={settings.alwaysPastePlainText ? 'plain' : 'rich'}
                  onChange={(e) => onUpdateSettings({ alwaysPastePlainText: e.target.value === 'plain' })}
                  className="theme-input border rounded-md px-3 py-1 text-xs focus:outline-none focus:border-gray-500 shrink-0"
                >
                  <option value="rich">Preserve Formatting (Default)</option>
                  <option value="plain">Always Paste Plain Text</option>
                </select>
              </div>

              <div className="flex items-start justify-between">
                <div className="pr-4 flex-1 min-w-0">
                  <span className="font-semibold text-gray-200 block">Maximum Clip Size (MB):</span>
                  <p className="text-[11px] text-gray-400/90 leading-normal mt-0.5">
                    Ignore copied clippings larger than the specified limit.
                  </p>
                </div>
                <div className="flex items-center space-x-1.5 font-mono shrink-0">
                  <input
                    type="number"
                    value={settings.maxClipSizeMb}
                    onChange={(e) => onUpdateSettings({ maxClipSizeMb: Number(e.target.value) })}
                    className="theme-input w-16 border rounded-md px-2 py-1 text-center focus:outline-none focus:border-gray-500"
                  />
                </div>
              </div>

              {/* Compact Keep Clippings Input + Slider */}
              <div className="space-y-2">
                <div className="flex items-start justify-between">
                  <div className="pr-4 flex-1 min-w-0">
                    <span className="font-semibold text-gray-200 block">History Capacity (clips):</span>
                    <p className="text-[11px] text-gray-400/90 leading-normal mt-0.5">
                      Maximum number of clippings saved in your local database history.
                    </p>
                  </div>
                  <div className="flex items-center space-x-2 font-mono shrink-0">
                    <input
                      type="number"
                      min={50}
                      max={5000}
                      step={50}
                      value={settings.keepClipCount}
                      onChange={(e) => onUpdateSettings({ keepClipCount: Number(e.target.value) })}
                      className="theme-input w-20 border rounded-md px-2 py-1 text-center font-bold focus:outline-none focus:border-gray-500"
                    />
                  </div>
                </div>

                <div className="flex items-center space-x-3 pt-1">
                  <span className="text-[10px] text-gray-500 font-mono">50</span>
                  <input
                    type="range"
                    min={50}
                    max={3000}
                    step={50}
                    value={settings.keepClipCount}
                    onChange={(e) => onUpdateSettings({ keepClipCount: Number(e.target.value) })}
                    className="flex-1 accent-gray-200 bg-gray-700 h-1.5 rounded-lg cursor-pointer"
                  />
                  <span className="text-[10px] text-gray-500 font-mono">3000</span>
                </div>
              </div>

              <div className="pt-3 border-t border-gray-700/80">
                <div className="flex items-start justify-between">
                  <div className="pr-4 flex-1 min-w-0">
                    <span className="font-semibold text-red-400 block">
                      {isAltPressed ? 'Delete All Clips:' : 'Trash All Clips:'}
                    </span>
                    <p className="text-[11px] text-gray-400/90 leading-normal mt-0.5">
                      Moves all unpinned and unprotected clips (including clips assigned to Bins) into Trash. Hold Option ⌥ to permanently delete.
                    </p>
                  </div>
                  <button
                    type="button"
                    onClick={(e) => onClearHistory?.(e.altKey)}
                    className="flex items-center space-x-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold text-red-400 bg-red-500/10 hover:bg-red-500/20 border border-red-500/30 transition-all shrink-0 cursor-pointer"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                    <span>{isAltPressed ? 'Delete All Clips' : 'Trash All Clips'}</span>
                  </button>
                </div>
              </div>

              <div className="pt-3 border-t border-gray-700/80">
                <div className="flex items-start justify-between">
                  <div className="pr-4 flex-1 min-w-0">
                    <span className="font-semibold text-gray-200 block">Backup & Export:</span>
                    <p className="text-[11px] text-gray-400/90 leading-normal mt-0.5">
                      Export clipboard history to JSON or CSV format.
                    </p>
                  </div>
                  <div className="flex items-center space-x-2 shrink-0">
                    <button
                      type="button"
                      onClick={() => void exportClips('json')}
                      className="flex items-center space-x-1 px-2.5 py-1.5 rounded-lg text-xs font-semibold text-cyan-300 bg-cyan-500/10 hover:bg-cyan-500/20 border border-cyan-500/30 transition-all cursor-pointer"
                    >
                      <Download className="w-3.5 h-3.5" />
                      <span>Export JSON</span>
                    </button>

                    <button
                      type="button"
                      onClick={() => void exportClips('csv')}
                      className="flex items-center space-x-1 px-2.5 py-1.5 rounded-lg text-xs font-semibold text-purple-300 bg-purple-500/10 hover:bg-purple-500/20 border border-purple-500/30 transition-all cursor-pointer"
                    >
                      <Download className="w-3.5 h-3.5" />
                      <span>Export CSV</span>
                    </button>
                  </div>
                </div>
                {exportStatus && (
                  <p role="status" className="theme-text-muted pt-2 text-right text-[11px]">
                    {exportStatus}
                  </p>
                )}
              </div>
            </div>

            <div className="border-t border-gray-700/80" />

            {/* Trash Preferences */}
            <div className="space-y-4">
              <h4 className="font-bold text-gray-100 text-center uppercase tracking-wider text-[11px]">
                Trash & Protection
              </h4>

              <div className="flex items-start justify-between">
                <div className="pr-4 flex-1 min-w-0">
                  <span className="font-semibold text-gray-200 block">Enable Soft Trash Protection:</span>
                  <p className="text-[11px] text-gray-400/90 leading-normal mt-0.5">
                    When enabled, deleted clips are moved into Trash for recovery. When disabled, clips are permanently purged immediately.
                  </p>
                </div>
                <button
                  type="button"
                  role="switch"
                  aria-checked={settings.enableTrash}
                  aria-label="Enable soft trash protection"
                  onClick={() => onUpdateSettings({ enableTrash: !settings.enableTrash })}
                  className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none ${
                    settings.enableTrash ? 'bg-rose-500' : 'bg-gray-700'
                  }`}
                >
                  <span
                    className={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out ${
                      settings.enableTrash ? 'translate-x-4' : 'translate-x-0'
                    }`}
                  />
                </button>
              </div>

              <div className="border-t border-gray-800/80 pt-3">
                <div className="flex items-start justify-between">
                  <div className="pr-4 flex-1 min-w-0">
                    <span className="font-semibold text-gray-200 block">Trash Capacity Limit:</span>
                    <p className="text-[11px] text-gray-400/90 leading-normal mt-0.5">
                      Maximum number of items retained in Trash before oldest trashed clips are permanently purged (default: 500).
                    </p>
                  </div>
                  <div className="flex items-center space-x-2 font-mono shrink-0">
                    <input
                      type="number"
                      min={50}
                      max={2000}
                      step={50}
                      value={settings.trashCapacityCount ?? 500}
                      onChange={(e) => onUpdateSettings({ trashCapacityCount: Number(e.target.value) })}
                      className="theme-input w-20 border rounded-md px-2 py-1 text-center font-bold focus:outline-none focus:border-gray-500 text-xs"
                    />
                  </div>
                </div>

                <div className="flex items-center space-x-3 pt-2">
                  <span className="text-[10px] text-gray-500 font-mono">50</span>
                  <input
                    type="range"
                    min={50}
                    max={2000}
                    step={50}
                    value={settings.trashCapacityCount ?? 500}
                    onChange={(e) => onUpdateSettings({ trashCapacityCount: Number(e.target.value) })}
                    className="flex-1 accent-rose-400 bg-gray-700 h-1.5 rounded-lg cursor-pointer"
                  />
                  <span className="text-[10px] text-gray-500 font-mono">2000</span>
                </div>
              </div>

              <div className="p-3 rounded-xl bg-rose-950/20 border border-rose-500/20 text-xs text-gray-300">
                <span className="font-bold text-rose-300">Auto-Trash Safety Net: </span>
                When your active history reaches your clip limit ({settings.keepClipCount} clips), older unpinned items automatically move into Trash instead of dropping off forever.
              </div>
            </div>

            <div className="border-t border-gray-700/80" />

            {/* Activity Log Preferences */}
            <div className="space-y-4">
              <h4 className="font-bold text-gray-100 text-center uppercase tracking-wider text-[11px]">
                Activity Log
              </h4>

              <div className="flex items-start justify-between">
                <div className="pr-4 flex-1 min-w-0">
                  <span className="font-semibold text-gray-200 block">Record Activity Logs:</span>
                  <p className="text-[11px] text-gray-400/90 leading-normal mt-0.5">
                    Automatically record system events such as trashed clips, restored clips, notes, and auto-pause events.
                  </p>
                </div>
                <button
                  type="button"
                  role="switch"
                  aria-checked={settings.enableActivityLog}
                  aria-label="Record activity logs"
                  onClick={() => onUpdateSettings({ enableActivityLog: !settings.enableActivityLog })}
                  className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none ${
                    settings.enableActivityLog ? 'bg-cyan-500' : 'bg-gray-700'
                  }`}
                >
                  <span
                    className={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out ${
                      settings.enableActivityLog ? 'translate-x-4' : 'translate-x-0'
                    }`}
                  />
                </button>
              </div>

              <div className="border-t border-gray-800/80 pt-3">
                <div className="flex items-start justify-between">
                  <div className="pr-4 flex-1 min-w-0">
                    <span className="font-semibold text-gray-200 block">Log Capacity Limit:</span>
                    <p className="text-[11px] text-gray-400/90 leading-normal mt-0.5">
                      Maximum number of activity log entries to retain before auto-purging old logs (default: 1000).
                    </p>
                  </div>
                  <div className="flex items-center space-x-2 font-mono shrink-0">
                    <input
                      type="number"
                      min={100}
                      max={5000}
                      step={100}
                      value={settings.activityLogCapacity ?? 1000}
                      onChange={(e) => onUpdateSettings({ activityLogCapacity: Number(e.target.value) })}
                      className="theme-input w-20 border rounded-md px-2 py-1 text-center font-bold focus:outline-none focus:border-gray-500 text-xs"
                    />
                  </div>
                </div>
              </div>
            </div>

            <div className="border-t border-gray-700/80" />

            {/* Layout Preferences */}
            <div className="space-y-4">
              <h4 className="font-bold text-gray-100 text-center uppercase tracking-wider text-[11px]">
                Layout
              </h4>

              <div className="flex items-start justify-between">
                <div className="pr-4 flex-1 min-w-0">
                  <span className="font-semibold text-gray-200 block">Row Height:</span>
                  <p className="text-[11px] text-gray-400/90 leading-normal mt-0.5">
                    Sets the fixed height of the quick paste menu and main window compact view.
                  </p>
                </div>
                <select
                  value={settings.rowHeight}
                  aria-label="Row height"
                  onChange={(e) => onUpdateSettings({ rowHeight: e.target.value as AppSettings['rowHeight'] })}
                  className="theme-input border rounded-md px-3 py-1 text-xs focus:outline-none focus:border-gray-500 shrink-0"
                >
                  <option value="small">Small</option>
                  <option value="medium">Medium</option>
                  <option value="large">Large</option>
                </select>
              </div>

              <div className="border-t border-gray-800/80 pt-3">
                <div className="flex items-start justify-between">
                  <div className="pr-4 flex-1 min-w-0">
                    <span className="font-semibold text-gray-200 block">Column Widths:</span>
                    <p className="text-[11px] text-gray-400/90 leading-normal mt-0.5">
                      Resets the left sidebar and middle history list panel widths back to their default macOS sizes.
                    </p>
                  </div>
                  <button
                    type="button"
                    onClick={() => {
                      if (onResetColumnWidths) {
                        onResetColumnWidths();
                      } else {
                        localStorage.removeItem('pasted_sidebar_width');
                        localStorage.removeItem('pasted_list_width');
                        window.location.reload();
                      }
                    }}
                    className="flex items-center space-x-1.5 px-3 py-1.5 bg-gray-800 hover:bg-gray-700 text-gray-200 hover:text-white border border-gray-600 rounded-lg text-xs font-semibold transition-colors shrink-0 cursor-pointer"
                  >
                    <RotateCcw className="w-3.5 h-3.5" />
                    <span>Reset Column Widths</span>
                  </button>
                </div>
              </div>
            </div>
          </div>
  );
}
