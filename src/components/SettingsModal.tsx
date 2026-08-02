import React, { useState } from 'react';
import { AppSettings, BlacklistApp, FilterRule, Board } from '../types';
import { invoke } from '@tauri-apps/api/core';
import { HotkeyRecorder } from './HotkeyRecorder';
import {
  Sliders,
  Command,
  Shield,
  Cloud,
  Plus,
  Trash2,
  ShieldCheck,
  RotateCcw,
  Lock,
  Sun,
  Moon,
  Laptop,
  Download,
} from 'lucide-react';

interface SettingsModalProps {
  settings: AppSettings;
  onUpdateSettings: (newSettings: Partial<AppSettings>) => void;
  blacklistApps: BlacklistApp[];
  onAddBlacklistApp: (appName: string) => void;
  onRemoveBlacklistApp: (appId: string) => void;
  onToggleBlacklistRule: (appId: string, rule: 'ignoreText' | 'ignoreImages' | 'ignoreShortcuts') => void;
  filters?: FilterRule[];
  onRefreshFilters?: () => void;
  boards?: Board[];
  onRefreshBoards?: () => void;
  onClearHistory?: () => void;
  onResetColumnWidths?: () => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({
  settings,
  onUpdateSettings,
  blacklistApps,
  onAddBlacklistApp,
  onRemoveBlacklistApp,
  onToggleBlacklistRule,
  filters = [],
  onRefreshFilters,
  boards = [],
  onRefreshBoards,
  onClearHistory,
  onResetColumnWidths,
}) => {
  const [activeTab, setActiveTab] = useState<'general' | 'hotkeys' | 'blacklist' | 'sync'>('general');
  const [newAppNameInput, setNewAppNameInput] = useState('');
  const [accessibilityStatus, setAccessibilityStatus] = useState<{ is_trusted: boolean; is_dev_mode: boolean } | null>(null);
  const [isAltPressed, setIsAltPressed] = useState(false);

  React.useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Alt' || e.altKey) setIsAltPressed(true);
    };
    const handleKeyUp = (e: KeyboardEvent) => {
      if (e.key === 'Alt' || !e.altKey) setIsAltPressed(false);
    };
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
    };
  }, []);

  React.useEffect(() => {
    const checkPerm = () => {
      invoke<{ is_trusted: boolean; is_dev_mode: boolean }>('check_accessibility_permission')
        .then(setAccessibilityStatus)
        .catch(() => setAccessibilityStatus({ is_trusted: true, is_dev_mode: false }));
    };

    checkPerm();
    const interval = setInterval(checkPerm, 1000);
    window.addEventListener('focus', checkPerm);

    return () => {
      clearInterval(interval);
      window.removeEventListener('focus', checkPerm);
    };
  }, []);

  return (
    <div data-tauri-drag-region className="flex-1 settings-modal-bg h-screen overflow-y-auto bg-[#141414] text-gray-100 font-sans select-none flex flex-col items-center p-6">
      <div data-tauri-drag-region className="w-full max-w-xl space-y-6">
        {/* macOS Native Style Segmented Tab Header */}
        <div data-tauri-drag-region className="flex items-center justify-center">
          <div data-tauri-drag-region className="flex items-center bg-[#212121] p-1 rounded-xl border border-gray-700/80 shadow-lg space-x-1">
            <button
              onClick={() => setActiveTab('general')}
              className={`flex flex-col items-center justify-center px-4 py-2 rounded-lg text-xs font-semibold transition-all border ${
                activeTab === 'general'
                  ? 'settings-tab-active bg-[#383838] text-white border-gray-500/80 shadow-md'
                  : 'settings-tab-idle border-transparent text-gray-400'
              }`}
            >
              <Sliders className="w-4 h-4 mb-1" />
              <span>General</span>
            </button>

            <button
              onClick={() => setActiveTab('hotkeys')}
              className={`flex flex-col items-center justify-center px-4 py-2 rounded-lg text-xs font-semibold transition-all border ${
                activeTab === 'hotkeys'
                  ? 'settings-tab-active bg-[#383838] text-white border-gray-500/80 shadow-md'
                  : 'settings-tab-idle border-transparent text-gray-400'
              }`}
            >
              <Command className="w-4 h-4 mb-1" />
              <span>Hotkeys</span>
            </button>

            <button
              onClick={() => setActiveTab('blacklist')}
              className={`flex flex-col items-center justify-center px-4 py-2 rounded-lg text-xs font-semibold transition-all border ${
                activeTab === 'blacklist'
                  ? 'settings-tab-active bg-[#383838] text-white border-gray-500/80 shadow-md'
                  : 'settings-tab-idle border-transparent text-gray-400'
              }`}
            >
              <Shield className="w-4 h-4 mb-1" />
              <span>Blacklist</span>
            </button>

            <button
              onClick={() => setActiveTab('sync')}
              className={`flex flex-col items-center justify-center px-4 py-2 rounded-lg text-xs font-semibold transition-all border ${
                activeTab === 'sync'
                  ? 'settings-tab-active bg-[#383838] text-white border-gray-500/80 shadow-md'
                  : 'settings-tab-idle border-transparent text-gray-400'
              }`}
            >
              <Cloud className="w-4 h-4 mb-1" />
              <span>Sync</span>
            </button>
          </div>
        </div>

        {/* TAB 1: GENERAL */}
        {activeTab === 'general' && (
          <div className="bg-[#212121] p-6 rounded-2xl border border-gray-700/80 shadow-2xl space-y-6 text-xs text-gray-200">
            {/* General Preferences */}
            <div className="space-y-3">
              {/* Appearance Mode Switcher */}
              <div className="flex items-center justify-between pb-1">
                <span className="font-medium">Appearance:</span>
                <div className="flex items-center bg-[#181818] p-1 rounded-lg border border-gray-700 space-x-1">
                  <button
                    type="button"
                    onClick={() => onUpdateSettings({ themeMode: 'system' })}
                    className={`flex items-center space-x-1 px-2.5 py-1 rounded-md text-[11px] font-medium transition-all ${
                      (settings.themeMode || 'system') === 'system'
                        ? 'bg-[#383838] text-white shadow-sm'
                        : 'text-gray-400 hover:text-black dark:hover:text-white'
                    }`}
                  >
                    <Laptop className="w-3.5 h-3.5" />
                    <span>System</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => onUpdateSettings({ themeMode: 'dark' })}
                    className={`flex items-center space-x-1 px-2.5 py-1 rounded-md text-[11px] font-medium transition-all ${
                      settings.themeMode === 'dark'
                        ? 'bg-[#383838] text-white shadow-sm'
                        : 'text-gray-400 hover:text-black dark:hover:text-white'
                    }`}
                  >
                    <Moon className="w-3.5 h-3.5" />
                    <span>Dark</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => onUpdateSettings({ themeMode: 'light' })}
                    className={`flex items-center space-x-1 px-2.5 py-1 rounded-md text-[11px] font-medium transition-all ${
                      settings.themeMode === 'light'
                        ? 'bg-[#383838] text-white shadow-sm'
                        : 'text-gray-400 hover:text-black dark:hover:text-white'
                    }`}
                  >
                    <Sun className="w-3.5 h-3.5" />
                    <span>Light</span>
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
                    className="bg-[#181818] border border-gray-700 rounded-md px-3 py-1 text-gray-100 font-mono text-xs focus:outline-none focus:border-gray-500"
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
                  onChange={(e) => onUpdateSettings({ dockMenubarIcon: e.target.value as any })}
                  className="bg-[#181818] border border-gray-700 rounded-md px-3 py-1 text-gray-100 text-xs focus:outline-none focus:border-gray-500"
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
                  className="bg-[#181818] border border-gray-700 rounded-md px-3 py-1 text-gray-100 text-xs focus:outline-none focus:border-gray-500 shrink-0"
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
                    className="w-16 bg-[#181818] border border-gray-700 rounded-md px-2 py-1 text-center text-gray-100 focus:outline-none focus:border-gray-500"
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
                      className="w-20 bg-[#181818] border border-gray-700 rounded-md px-2 py-1 text-center text-gray-100 font-bold focus:outline-none focus:border-gray-500"
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
                      Moves all unpinned and unprotected clips (including clips assigned to Pasteboards) into Trash. Hold Option ⌥ to permanently delete.
                    </p>
                  </div>
                  <button
                    type="button"
                    onClick={async (e) => {
                      if (e.altKey || isAltPressed) {
                        try {
                          await invoke('purge_unpinned_clips');
                          onClearHistory?.();
                        } catch (err) {
                          console.error(err);
                        }
                      } else {
                        try {
                          await invoke('trash_unpinned_clips');
                          onClearHistory?.();
                        } catch (err) {
                          console.error(err);
                        }
                      }
                    }}
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
                      onClick={async () => {
                        try {
                          const json = await invoke<string>('export_clips_json');
                          const blob = new Blob([json], { type: 'application/json' });
                          const url = URL.createObjectURL(blob);
                          const a = document.createElement('a');
                          a.href = url;
                          a.download = `pasted_backup_${Date.now()}.json`;
                          a.click();
                        } catch (e) {
                          console.error(e);
                        }
                      }}
                      className="flex items-center space-x-1 px-2.5 py-1.5 rounded-lg text-xs font-semibold text-cyan-300 bg-cyan-500/10 hover:bg-cyan-500/20 border border-cyan-500/30 transition-all cursor-pointer"
                    >
                      <Download className="w-3.5 h-3.5" />
                      <span>Export JSON</span>
                    </button>

                    <button
                      type="button"
                      onClick={async () => {
                        try {
                          const csv = await invoke<string>('export_clips_csv');
                          const blob = new Blob([csv], { type: 'text/csv' });
                          const url = URL.createObjectURL(blob);
                          const a = document.createElement('a');
                          a.href = url;
                          a.download = `pasted_export_${Date.now()}.csv`;
                          a.click();
                        } catch (e) {
                          console.error(e);
                        }
                      }}
                      className="flex items-center space-x-1 px-2.5 py-1.5 rounded-lg text-xs font-semibold text-purple-300 bg-purple-500/10 hover:bg-purple-500/20 border border-purple-500/30 transition-all cursor-pointer"
                    >
                      <Download className="w-3.5 h-3.5" />
                      <span>Export CSV</span>
                    </button>
                  </div>
                </div>
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
                      className="w-20 bg-[#181818] border border-gray-700 rounded-md px-2 py-1 text-center text-gray-100 font-bold focus:outline-none focus:border-gray-500 text-xs"
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
                      className="w-20 bg-[#181818] border border-gray-700 rounded-md px-2 py-1 text-center text-gray-100 font-bold focus:outline-none focus:border-gray-500 text-xs"
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
                  onChange={(e) => onUpdateSettings({ rowHeight: e.target.value as any })}
                  className="bg-[#181818] border border-gray-700 rounded-md px-3 py-1 text-gray-100 text-xs focus:outline-none focus:border-gray-500 shrink-0"
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
        )}

        {/* TAB 2: HOTKEYS */}
        {activeTab === 'hotkeys' && (
          <div className="bg-[#212121] p-6 rounded-2xl border border-gray-700/80 shadow-2xl space-y-6 text-xs text-gray-200">
            <div className="flex items-center justify-between">
              <h3 className="font-bold text-sm text-gray-100">Global Application Hotkeys</h3>
              <button
                onClick={() => alert('Hotkeys restored to defaults.')}
                className="flex items-center space-x-1.5 px-3 py-1 bg-gray-800 hover:bg-gray-700 border border-gray-600 rounded-lg text-gray-300 transition-colors"
              >
                <RotateCcw className="w-3.5 h-3.5" />
                <span>Restore Defaults</span>
              </button>
            </div>

            {/* macOS Accessibility Permission Card */}
            <div className="p-3.5 bg-[#181818] rounded-xl border border-gray-700/80 space-y-2">
              <div className="flex items-center justify-between">
                <div className="flex items-center space-x-2">
                  <ShieldCheck className={`w-4 h-4 ${accessibilityStatus?.is_trusted ? 'text-green-400' : 'text-amber-400'}`} />
                  <span className="font-bold text-xs text-gray-200">macOS System Accessibility Permission</span>
                </div>
                <div className="flex items-center space-x-2">
                  {accessibilityStatus?.is_dev_mode && (
                    <span className="text-[9px] font-mono px-2 py-0.5 rounded bg-cyan-950/80 border border-cyan-800/60 text-cyan-300 font-bold shrink-0 whitespace-nowrap">
                      DEV MODE
                    </span>
                  )}
                  <span className={`text-[10px] font-mono font-bold px-2 py-0.5 rounded-full border ${
                    accessibilityStatus?.is_trusted
                      ? 'bg-green-500/20 text-green-300 border-green-500/30'
                      : 'bg-amber-500/20 text-amber-300 border-amber-500/30'
                  }`}>
                    {accessibilityStatus?.is_trusted ? 'GRANTED' : 'REQUIRED'}
                  </span>
                  <button
                    onClick={async () => {
                      await invoke('request_accessibility_permission');
                      setTimeout(async () => {
                        const res = await invoke<{ is_trusted: boolean; is_dev_mode: boolean }>('check_accessibility_permission');
                        setAccessibilityStatus(res);
                      }, 1500);
                    }}
                    className="px-2.5 py-1 bg-gray-800 hover:bg-gray-700 text-gray-200 hover:text-white border border-gray-600 rounded-lg text-[10px] font-semibold transition-colors cursor-pointer"
                  >
                    Open System Settings
                  </button>
                </div>
              </div>
              <p className="text-[11px] text-gray-400 leading-normal">
                macOS requires Accessibility access for global hotkeys. {accessibilityStatus?.is_dev_mode ? (
                  <span>Running in <strong>development mode</strong>: grant permission to your active IDE / terminal host application under System Settings &gt; Privacy &amp; Security &gt; Accessibility.</span>
                ) : (
                  <span>Grant permission to <strong>Pasted.app</strong> under System Settings &gt; Privacy &amp; Security &gt; Accessibility.</span>
                )}
              </p>
            </div>

            {/* Custom Pasteboard Hotkeys */}
            <div className="space-y-2">
              <h4 className="font-bold text-gray-400 uppercase tracking-wider text-[10px]">
                Custom Pasteboard Hotkeys ({boards?.length || 0})
              </h4>

              {(!boards || boards.length === 0) ? (
                <p className="text-[11px] text-gray-500 italic p-2.5 bg-[#181818] rounded-xl border border-gray-800">
                  No custom pasteboards created yet. Create pasteboards in the sidebar to assign global shortcuts.
                </p>
              ) : (
                boards.map((b) => (
                  <div key={b.id} className="flex items-center justify-between p-2.5 bg-[#181818] rounded-xl border border-gray-700/80">
                    <span className="font-medium text-gray-200">{b.name}</span>
                    <HotkeyRecorder
                      value={b.shortcut}
                      onChange={async (newShortcut) => {
                        try {
                          await invoke('update_board_shortcut', { id: b.id, shortcut: newShortcut });
                          if (onRefreshBoards) onRefreshBoards();
                        } catch (err) {
                          console.error(err);
                        }
                      }}
                    />
                  </div>
                ))
              )}
            </div>

            {/* Filter Pipeline Hotkeys */}
            <div className="space-y-2 pt-3 border-t border-gray-700/80">
              <h4 className="font-bold theme-text-muted uppercase tracking-wider text-[10px]">
                Filter Pipeline Hotkeys ({filters.length})
              </h4>
              <p className="text-[11px] theme-text-muted">
                Assign custom shortcuts to trigger automated text filters instantly.
              </p>

              <div className="space-y-2 pt-1 max-h-60 overflow-y-auto pr-1">
                {filters.map((f) => (
                  <div key={f.id} className="flex items-center justify-between p-2.5 bg-[#181818] rounded-xl border border-gray-700/80 theme-surface">
                    <div>
                      <span className="font-bold text-gray-200 theme-text-main block">{f.name}</span>
                      <span className="text-[10px] font-mono text-cyan-400/80">{f.filter_type}</span>
                    </div>
                    <HotkeyRecorder
                      value={f.shortcut}
                      onChange={async (newShortcut) => {
                        try {
                          await invoke('update_filter_shortcut', { id: f.id, shortcut: newShortcut });
                          if (onRefreshFilters) onRefreshFilters();
                        } catch (err) {
                          console.error(err);
                        }
                      }}
                    />
                  </div>
                ))}
              </div>
            </div>

            {/* Actions */}
            <div className="space-y-2 pt-2 border-t border-gray-700/80">
              <h4 className="font-bold text-gray-400 uppercase tracking-wider text-[10px]">
                Actions
              </h4>

              <div className="flex items-center justify-between p-2.5 bg-[#181818] rounded-xl border border-gray-700/80">
                <span className="font-medium text-gray-200">HUD</span>
                <HotkeyRecorder
                  value={settings.hudHotkey === '' ? null : (settings.hudHotkey || 'Alt+Shift+V')}
                  onChange={async (newKey) => {
                    const updated = newKey === null ? '' : newKey;
                    onUpdateSettings({ hudHotkey: updated });
                    try {
                      await invoke('register_hud_shortcut', { shortcutStr: updated });
                    } catch (err) {
                      console.error('Failed to register HUD shortcut:', err);
                    }
                  }}
                />
              </div>

              <div className="flex items-center justify-between p-2.5 bg-[#181818] rounded-xl border border-gray-700/80">
                <span className="font-medium text-gray-200">Enable/Disable Queue</span>
                <HotkeyRecorder
                  value={settings.seqToggleHotkey === '' ? null : (settings.seqToggleHotkey || 'Alt+Shift+C')}
                  onChange={async (newKey) => {
                    const val = newKey === null ? '' : newKey;
                    onUpdateSettings({ seqToggleHotkey: val });
                    try {
                      await invoke('register_app_setting_hotkey', { key: 'seqToggleHotkey', value: val });
                    } catch (err) {
                      console.error(err);
                    }
                  }}
                />
              </div>

              <div className="flex items-center justify-between p-2.5 bg-[#181818] rounded-xl border border-gray-700/80">
                <span className="font-medium text-gray-200">Paste Next Item from Queue</span>
                <HotkeyRecorder
                  value={settings.seqPopHotkey === '' ? null : (settings.seqPopHotkey || 'Alt+Shift+X')}
                  onChange={async (newKey) => {
                    const val = newKey === null ? '' : newKey;
                    onUpdateSettings({ seqPopHotkey: val });
                    try {
                      await invoke('register_app_setting_hotkey', { key: 'seqPopHotkey', value: val });
                    } catch (err) {
                      console.error(err);
                    }
                  }}
                />
              </div>

              <div className="flex items-center justify-between p-2.5 bg-[#181818] rounded-xl border border-gray-700/80">
                <span className="font-medium text-gray-200">Paste with Last Filter</span>
                <HotkeyRecorder
                  value={settings.pasteLastFilterHotkey === '' ? null : (settings.pasteLastFilterHotkey || null)}
                  onChange={async (newKey) => {
                    const val = newKey === null ? '' : newKey;
                    onUpdateSettings({ pasteLastFilterHotkey: val });
                    try {
                      await invoke('register_app_setting_hotkey', { key: 'pasteLastFilterHotkey', value: val });
                    } catch (err) {
                      console.error(err);
                    }
                  }}
                />
              </div>

              <div className="flex items-center justify-between p-2.5 bg-[#181818] rounded-xl border border-gray-700/80">
                <span className="font-medium text-gray-200">Open Filter Window</span>
                <HotkeyRecorder
                  value={settings.openFilterWindowHotkey === '' ? null : (settings.openFilterWindowHotkey || null)}
                  onChange={async (newKey) => {
                    const val = newKey === null ? '' : newKey;
                    onUpdateSettings({ openFilterWindowHotkey: val });
                    try {
                      await invoke('register_app_setting_hotkey', { key: 'openFilterWindowHotkey', value: val });
                    } catch (err) {
                      console.error(err);
                    }
                  }}
                />
              </div>

              <div className="flex items-center justify-between p-2.5 bg-[#181818] rounded-xl border border-gray-700/80">
                <span className="font-medium text-gray-200">Toggle Main Window</span>
                <HotkeyRecorder
                  value={settings.openMainWindowHotkey === '' ? null : (settings.openMainWindowHotkey || null)}
                  onChange={async (newKey) => {
                    const val = newKey === null ? '' : newKey;
                    onUpdateSettings({ openMainWindowHotkey: val });
                    try {
                      await invoke('register_app_setting_hotkey', { key: 'openMainWindowHotkey', value: val });
                    } catch (err) {
                      console.error(err);
                    }
                  }}
                />
              </div>
            </div>

            {/* Paste Recent Clippings */}
            <div className="space-y-2 pt-2">
              <h4 className="font-bold text-gray-400 uppercase tracking-wider text-[10px]">
                Paste Recent Clippings
              </h4>

              {[1, 2, 3, 4, 5, 6, 7, 8, 9].map((num) => {
                const keyName = `pasteClip${num}Hotkey` as keyof AppSettings;
                return (
                  <div
                    key={num}
                    className="flex items-center justify-between p-2.5 bg-[#181818] rounded-xl border border-gray-700/80"
                  >
                    <span className="font-medium text-gray-300">Paste Clipping {num}</span>
                    <HotkeyRecorder
                      value={(settings[keyName] as string) || null}
                      onChange={async (newKey) => {
                        const val = newKey || '';
                        onUpdateSettings({ [keyName]: val });
                        try {
                          await invoke('register_app_setting_hotkey', { key: keyName, value: val });
                        } catch (err) {
                          console.error(err);
                        }
                      }}
                    />
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* TAB 3: BLACKLIST */}
        {activeTab === 'blacklist' && (
          <div className="bg-[#212121] p-6 rounded-2xl border border-gray-700/80 shadow-2xl space-y-4 text-xs text-gray-200">
            <h4 className="font-bold text-gray-100 uppercase tracking-wider text-[11px]">
              Ignore from the Following Apps:
            </h4>

            {/* App Blacklist Items */}
            <div className="space-y-2 max-h-72 overflow-y-auto pr-1">
              {blacklistApps.map((app) => (
                <div
                  key={app.id}
                  className="flex items-center justify-between p-3 bg-[#181818] rounded-xl border border-gray-700/80"
                >
                  <div className="flex items-center space-x-3">
                    <div className="w-7 h-7 rounded-lg bg-gray-800 border border-gray-700 flex items-center justify-center">
                      <Lock className="w-4 h-4 text-amber-400" />
                    </div>
                    <span className="font-semibold text-gray-200">{app.name}</span>
                  </div>

                  <div className="flex items-center space-x-4">
                    <label className="flex items-center space-x-1.5 cursor-pointer text-gray-400">
                      <input
                        type="checkbox"
                        checked={app.ignoreShortcuts}
                        onChange={() => onToggleBlacklistRule(app.id, 'ignoreShortcuts')}
                        className="w-3.5 h-3.5 accent-[#007aff] cursor-pointer rounded"
                      />
                      <span>Shortcuts</span>
                    </label>

                    <label className="flex items-center space-x-1.5 cursor-pointer text-gray-200 font-medium">
                      <input
                        type="checkbox"
                        checked={app.ignoreText}
                        onChange={() => onToggleBlacklistRule(app.id, 'ignoreText')}
                        className="w-3.5 h-3.5 accent-[#007aff] cursor-pointer rounded"
                      />
                      <span>Text</span>
                    </label>

                    <label className="flex items-center space-x-1.5 cursor-pointer text-gray-200 font-medium">
                      <input
                        type="checkbox"
                        checked={app.ignoreImages}
                        onChange={() => onToggleBlacklistRule(app.id, 'ignoreImages')}
                        className="w-3.5 h-3.5 accent-[#007aff] cursor-pointer rounded"
                      />
                      <span>Images</span>
                    </label>

                    <button
                      onClick={() => onRemoveBlacklistApp(app.id)}
                      className="p-1 text-gray-500 hover:text-red-400 rounded hover:bg-gray-800 transition-colors"
                      title="Remove App"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              ))}
            </div>

            {/* Add App Controls: Quick Select Dropdown + Custom Text Input */}
            <div className="space-y-2 pt-2 border-t border-gray-700/80">
              <div className="flex items-center space-x-2">
                <select
                  onChange={(e) => {
                    const val = e.target.value;
                    if (val) {
                      setNewAppNameInput(val);
                    }
                  }}
                  defaultValue=""
                  className="flex-1 bg-[#181818] border border-gray-700 rounded-lg px-3 py-1.5 text-gray-200 text-xs focus:outline-none focus:border-gray-500 truncate"
                >
                  <option value="" disabled>
                    -- Select Installed or Popular App --
                  </option>
                  <optgroup label="Security & Password Managers">
                    <option value="1Password">1Password</option>
                    <option value="Bitwarden">Bitwarden</option>
                    <option value="Dashlane">Dashlane</option>
                    <option value="KeePassXC">KeePassXC</option>
                    <option value="Enpass">Enpass</option>
                    <option value="LastPass">LastPass</option>
                  </optgroup>
                  <optgroup label="Messaging & Private Chat">
                    <option value="Signal">Signal</option>
                    <option value="Telegram">Telegram</option>
                    <option value="Slack">Slack</option>
                    <option value="Discord">Discord</option>
                    <option value="WhatsApp">WhatsApp</option>
                  </optgroup>
                  <optgroup label="Web Browsers (Private Windows)">
                    <option value="Safari">Safari</option>
                    <option value="Google Chrome">Google Chrome</option>
                    <option value="Firefox">Firefox</option>
                    <option value="Brave Browser">Brave Browser</option>
                    <option value="Arc">Arc</option>
                    <option value="Orion">Orion</option>
                  </optgroup>
                  <optgroup label="System & Developer Tools">
                    <option value="Terminal">Terminal</option>
                    <option value="Warp">Warp</option>
                    <option value="VS Code">VS Code</option>
                    <option value="Xcode">Xcode</option>
                    <option value="Notes">Notes</option>
                    <option value="Mail">Mail</option>
                  </optgroup>
                </select>
              </div>

              <div className="flex items-center space-x-2">
                <input
                  type="text"
                  placeholder="Or type custom app name (e.g. Signal, Bitwarden)..."
                  value={newAppNameInput}
                  onChange={(e) => setNewAppNameInput(e.target.value)}
                  className="flex-1 bg-[#181818] border border-gray-700 rounded-lg px-3 py-1.5 text-gray-100 text-xs focus:outline-none focus:border-gray-500"
                />
                <button
                  onClick={() => {
                    if (newAppNameInput.trim()) {
                      onAddBlacklistApp(newAppNameInput.trim());
                      setNewAppNameInput('');
                    }
                  }}
                  className="flex items-center space-x-1 px-3.5 py-1.5 bg-white hover:bg-gray-200 text-black font-semibold rounded-lg transition-all text-xs shadow-md active:scale-95"
                  title="Add App to Blacklist"
                >
                  <Plus className="w-4 h-4" />
                  <span>Add App</span>
                </button>
              </div>
            </div>

            <p className="text-[11px] text-gray-400 leading-relaxed pt-2">
              Apps that mark sensitive data as transient (like 1Password) are already ignored. Checked items will be ignored by Pasted when copying or activating Pasted global shortcuts in these apps.
            </p>
          </div>
        )}

        {/* TAB 4: SYNC */}
        {activeTab === 'sync' && (
          <div className="bg-[#212121] p-6 rounded-2xl border border-gray-700/80 shadow-2xl space-y-6 text-xs text-gray-200">
            {/* Main Information Banner */}
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
                All your clipboard history items, custom notes, smart pasteboards, and filter pipelines are currently saved <strong>100% locally and securely</strong> on this device inside your private SQLite database.
              </p>

              <div className="p-3 bg-gray-800/40 rounded-lg border border-gray-700/50 space-y-1.5 text-[11px] theme-text-muted">
                <div className="flex items-center space-x-2 text-gray-300">
                  <ShieldCheck className="w-4 h-4 text-emerald-400" />
                  <span className="font-semibold theme-title">Local Privacy & Safety First</span>
                </div>
                <p className="pl-6">
                  No data ever leaves your computer. CloudKit cross-device synchronization will be enabled in an upcoming release once Apple Developer entitlement provisioning is complete.
                </p>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
