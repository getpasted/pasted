import { Building2, Coffee, Download, Droplet, Drum, Laptop, Minus, Moon, Pizza, Plus, RotateCcw, Sliders, Snowflake, Trash2, Zap } from 'lucide-react';
import type { AppSettings } from '../types';
import { useAltKeyPressed } from '../hooks/useAltKeyPressed';
import { safeInvoke as invoke } from '../utils/tauri';
import { MenuSelect } from './MenuSelect';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
import { ACTUAL_SIZE, APP_ZOOM_STEPS, appZoomPercent, stepAppZoom } from '../utils/appZoom';
import { useToast } from './ToastProvider';
import { ActionButton } from './AppDialogLayout';

interface SettingsGeneralPanelProps {
  settings: AppSettings;
  onUpdateSettings: (newSettings: Partial<AppSettings>) => void;
  onClearHistory?: (permanent: boolean) => void;
  onResetColumnWidths?: () => void;
}

const appearanceModes = [
  { value: 'system', label: 'System', Icon: Laptop },
  { value: 'dark', label: 'Dark', Icon: Moon },
  { value: 'cool', label: 'Cool', Icon: Snowflake },
  { value: 'warm', label: 'Warm', Icon: Coffee },
  { value: '2894', label: '2894', Icon: Building2 },
  { value: 'sauced', label: 'Sauced', Icon: Pizza },
  { value: 'vampire', label: 'Vampire', Icon: Droplet },
  { value: 'flux', label: 'Flux', Icon: Zap },
  { value: '808', label: '808', Icon: Drum },
] as const;

const appearanceGroups = [
  { label: 'System', values: ['system'] },
  { label: 'Dark schemes', values: ['dark', 'vampire', 'flux', '808'] },
  { label: 'Light schemes', values: ['cool', 'warm', '2894', 'sauced'] },
] as const;

const pasteBehaviorOptions = [
  { value: 'rich', label: 'Preserve Formatting (Default)' },
  { value: 'plain', label: 'Always Paste Plain Text' },
];

const filePreviewOptions = [
  { value: 'off', label: 'Off' },
  { value: 'safe', label: 'Safe Types' },
  { value: 'all', label: 'All Supported' },
];

const filePreviewDescriptions: Record<AppSettings['filePreviewMode'], string> = {
  off: 'Show file names and locations without previewing their contents.',
  safe: 'Show previews for TXT, PNG, JPEG, WebP, and the first page of PDF files.',
  all: 'Preview supported text, images, and the first page of PDF files.',
};

const revisionLimitOptions = [10, 25, 50, 100]
  .map((value) => ({ value: String(value), label: `${value} revisions` }))
  .concat({ value: '0', label: 'Unlimited' });

const historyCountPresets = [
  { value: '0', label: 'Unlimited' },
  { value: '250', label: '250 clips' },
  { value: '500', label: '500 clips' },
  { value: '1000', label: '1,000 clips (Default)' },
  { value: '5000', label: '5,000 clips' },
  { value: '10000', label: '10,000 clips' },
  { value: '50000', label: '50,000 clips' },
];

const trashCountPresets = [
  { value: '0', label: 'Unlimited' },
  { value: '100', label: '100 clips' },
  { value: '250', label: '250 clips' },
  { value: '500', label: '500 clips (Default)' },
  { value: '1000', label: '1,000 clips' },
  { value: '2000', label: '2,000 clips' },
  { value: '5000', label: '5,000 clips' },
];

const activityCountPresets = [
  { value: '0', label: 'Unlimited' },
  { value: '250', label: '250 entries' },
  { value: '500', label: '500 entries' },
  { value: '1000', label: '1,000 entries (Default)' },
  { value: '2500', label: '2,500 entries' },
  { value: '5000', label: '5,000 entries' },
  { value: '10000', label: '10,000 entries' },
];

const retentionAgeOptions = [
  { value: '0', label: 'Forever' },
  { value: '1', label: '1 day' },
  { value: '7', label: '7 days' },
  { value: '30', label: '30 days' },
  { value: '90', label: '90 days' },
  { value: '365', label: '1 year' },
];

const rowHeightOptions = [
  { value: 'small', label: 'Compact' },
  { value: 'medium', label: 'Standard' },
  { value: 'large', label: 'Spacious' },
];

const startupViewOptions = [
  { value: 'last_active', label: 'Last Active Page' },
  { value: 'clip_history', label: 'Clip History' },
];

export function SettingsGeneralPanel({
  settings,
  onUpdateSettings,
  onClearHistory,
  onResetColumnWidths,
}: SettingsGeneralPanelProps) {
  const { showToast } = useToast();
  const isAltPressed = useAltKeyPressed();
  const isMac = typeof navigator !== 'undefined' && /Mac|iPod|iPhone|iPad/.test(navigator.userAgent || navigator.platform);
  const dockIconOptions = isMac
    ? [
        { value: 'auto_hide', label: 'Auto hide Dock Icon' },
        { value: 'both', label: 'Always show Dock & Menubar' },
        { value: 'menubar_only', label: 'Menubar Icon only' },
      ]
    : [
        { value: 'auto_hide', label: 'Auto hide Taskbar Icon' },
        { value: 'both', label: 'Always show Tray & Taskbar' },
        { value: 'menubar_only', label: 'System Tray Icon only' },
      ];
  const menubarIconOptions = [
    { value: 'clipboard', label: 'Clipboard' },
    { value: 'copycat', label: 'Copycat' },
  ];
  const historyCountOptions = historyCountPresets.some(({ value }) => Number(value) === settings.keepClipCount)
    ? historyCountPresets
    : [
        ...historyCountPresets.slice(0, 1),
        { value: String(settings.keepClipCount), label: `${settings.keepClipCount.toLocaleString()} clips (Custom)` },
        ...historyCountPresets.slice(1),
      ];
  const historyAgeMenuOptions = retentionAgeOptions.some(({ value }) => Number(value) === settings.keepClipAgeDays)
    ? retentionAgeOptions
    : [
        ...retentionAgeOptions.slice(0, 1),
        { value: String(settings.keepClipAgeDays), label: `${settings.keepClipAgeDays.toLocaleString()} days (Custom)` },
        ...retentionAgeOptions.slice(1),
      ];
  const trashCountOptions = trashCountPresets.some(({ value }) => Number(value) === settings.trashCapacityCount)
    ? trashCountPresets
    : [
        ...trashCountPresets.slice(0, 1),
        { value: String(settings.trashCapacityCount), label: `${settings.trashCapacityCount.toLocaleString()} clips (Custom)` },
        ...trashCountPresets.slice(1),
      ];
  const trashAgeMenuOptions = retentionAgeOptions.some(({ value }) => Number(value) === settings.trashAgeDays)
    ? retentionAgeOptions
    : [
        ...retentionAgeOptions.slice(0, 1),
        { value: String(settings.trashAgeDays), label: `${settings.trashAgeDays.toLocaleString()} days (Custom)` },
        ...retentionAgeOptions.slice(1),
      ];
  const activityCountOptions = activityCountPresets.some(({ value }) => Number(value) === settings.activityLogCapacity)
    ? activityCountPresets
    : [
        ...activityCountPresets.slice(0, 1),
        { value: String(settings.activityLogCapacity), label: `${settings.activityLogCapacity.toLocaleString()} entries (Custom)` },
        ...activityCountPresets.slice(1),
      ];
  const activityAgeMenuOptions = retentionAgeOptions.some(({ value }) => Number(value) === settings.activityLogAgeDays)
    ? retentionAgeOptions
    : [
        ...retentionAgeOptions.slice(0, 1),
        { value: String(settings.activityLogAgeDays), label: `${settings.activityLogAgeDays.toLocaleString()} days (Custom)` },
        ...retentionAgeOptions.slice(1),
      ];
  const retentionSummary = settings.keepClipCount === 0 && settings.keepClipAgeDays === 0
    ? 'Automatic history cleanup is off. Pinned and protected clips remain exempt if you enable a limit later.'
    : 'Eligible clips that exceed your active retention limits automatically move into Trash instead of dropping off forever.';

  const exportClips = async (format: 'json' | 'csv') => {
    try {
      const contents = format === 'json'
        ? await invoke<string>('export_clips_json')
        : await invoke<string>('export_clips_csv');
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
      showToast({ tone: 'success', message: `${format.toUpperCase()} export downloaded.` });
    } catch (error) {
      console.error(`Failed to export clips as ${format}:`, error);
      showToast({ tone: 'error', message: `${format.toUpperCase()} export failed.` });
    }
  };

  return (
          <div className="space-y-6 text-xs">
            <SettingsPanelHeader
              icon={Sliders}
              title="General"
              description="Appearance, clipboard behavior, and history."
            />
            {/* General Preferences */}
            <div className="space-y-4">
              <SettingsSubsectionHeader
                title="Appearance"
                description="Choose the color scheme Pasted uses throughout the app."
              />

              {/* Appearance Mode Switcher */}
              <div className="flex items-center justify-between pb-1">
                <span className="font-medium">
                  Color Scheme: <strong className="theme-text-muted ml-1">{appearanceModes.find(({ value }) => value === (settings.themeMode || 'system'))?.label}</strong>
                </span>
                <div className="theme-surface appearance-picker flex items-center p-1 rounded-xl border gap-1" role="group" aria-label="Appearance scheme">
                  {appearanceGroups.map((group) => (
                    <div key={group.label} className="appearance-picker-group flex items-center gap-1" role="group" aria-label={group.label}>
                      {group.values.map((value) => {
                        const mode = appearanceModes.find((candidate) => candidate.value === value)!;
                        const isActive = (settings.themeMode || 'system') === value;
                        return (
                          <button
                            key={value}
                            type="button"
                            title={mode.label}
                            aria-label={`${mode.label} appearance`}
                            aria-pressed={isActive}
                            onClick={() => onUpdateSettings({ themeMode: value })}
                            className={`appearance-mode-button flex h-8 w-8 items-center justify-center rounded-lg transition-[background-color,color,box-shadow] ${isActive ? 'is-active' : ''}`}
                          >
                            <mode.Icon className="w-3.5 h-3.5" />
                          </button>
                        );
                      })}
                    </div>
                  ))}
                </div>
              </div>
            </div>

            <div className="theme-divider border-t" />

            <div className="space-y-4">
              <SettingsSubsectionHeader
                title="Layout"
                description="Adjust app scaling, clip density, and workspace dimensions."
              />

              <div className="flex items-start justify-between gap-4">
                <div className="min-w-0 flex-1">
                  <span className="font-semibold theme-text-main block">Zoom:</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    Scales navigation, controls, and clip content throughout Pasted.
                  </p>
                </div>
                <div className="theme-surface flex shrink-0 items-center overflow-hidden rounded-lg border" role="group" aria-label="Application zoom">
                  <button
                    type="button"
                    aria-label="Zoom Out"
                    title="Zoom Out (⌘−)"
                    disabled={settings.textSize <= APP_ZOOM_STEPS[0]}
                    onClick={() => onUpdateSettings({ textSize: stepAppZoom(settings.textSize, -1) })}
                    className="theme-secondary-button flex h-8 w-8 items-center justify-center border-0 border-r disabled:cursor-not-allowed disabled:opacity-35"
                  >
                    <Minus className="h-3.5 w-3.5" />
                  </button>
                  <button
                    type="button"
                    aria-label="Actual Size"
                    title="Actual Size (⌘0)"
                    onClick={() => onUpdateSettings({ textSize: ACTUAL_SIZE })}
                    className="theme-secondary-button h-8 min-w-14 border-0 px-2 font-mono text-[10px] font-semibold"
                  >
                    {appZoomPercent(settings.textSize)}%
                  </button>
                  <button
                    type="button"
                    aria-label="Zoom In"
                    title="Zoom In (⌘+)"
                    disabled={settings.textSize >= APP_ZOOM_STEPS[APP_ZOOM_STEPS.length - 1]}
                    onClick={() => onUpdateSettings({ textSize: stepAppZoom(settings.textSize, 1) })}
                    className="theme-secondary-button flex h-8 w-8 items-center justify-center border-0 border-l disabled:cursor-not-allowed disabled:opacity-35"
                  >
                    <Plus className="h-3.5 w-3.5" />
                  </button>
                </div>
              </div>

              <div className="flex items-start justify-between">
                <div className="pr-4 flex-1 min-w-0">
                  <span className="font-semibold theme-text-main block">Clip Density:</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    Adjusts clip spacing, text depth, and preview size throughout the history list.
                  </p>
                </div>
                <MenuSelect
                  value={settings.rowHeight}
                  options={rowHeightOptions}
                  onChange={(value) => onUpdateSettings({ rowHeight: value as AppSettings['rowHeight'] })}
                  label="Clip density"
                  className="settings-menu-select"
                />
              </div>

              <div className="flex items-start justify-between gap-4">
                <div className="min-w-0 flex-1">
                  <span className="font-semibold theme-text-main block">Startup View:</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    Choose whether Pasted reopens where you left off or always starts in Clip History.
                  </p>
                </div>
                <MenuSelect
                  value={settings.startupView}
                  options={startupViewOptions}
                  onChange={(value) => onUpdateSettings({ startupView: value as AppSettings['startupView'] })}
                  label="Startup view"
                  className="settings-menu-select"
                />
              </div>

              <div className="flex items-start justify-between">
                <div className="pr-4 flex-1 min-w-0">
                  <span className="font-semibold theme-text-main block">Column Widths:</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    Resets the left sidebar and middle history list panel widths to their defaults.
                  </p>
                </div>
                <ActionButton
                  onClick={() => {
                    if (onResetColumnWidths) onResetColumnWidths();
                    else {
                      localStorage.removeItem('pasted_sidebar_width');
                      localStorage.removeItem('pasted_list_width');
                      window.location.reload();
                    }
                  }}
                  className="shrink-0 cursor-pointer"
                >
                  <RotateCcw className="w-3.5 h-3.5" />
                  <span>Reset Column Widths</span>
                </ActionButton>
              </div>
            </div>

            <div className="theme-divider border-t" />

            {/* System & OS Integration Subsection */}
            <div className="space-y-4">
              <SettingsSubsectionHeader
                title="System & OS Integration"
                description="Control startup, sounds, and how Pasted appears in the operating system."
              />

              {/* Dock / Menubar / System Tray Setting */}
              <div className="flex items-center justify-between pt-1">
                <span className="font-medium">
                  {isMac
                    ? 'Dock & Menubar Icon:'
                    : 'System Tray & Taskbar:'}
                </span>
                <MenuSelect
                  value={settings.dockMenubarIcon}
                  options={dockIconOptions}
                  onChange={(value) => onUpdateSettings({ dockMenubarIcon: value as AppSettings['dockMenubarIcon'] })}
                  label="Dock and menu bar icon behavior"
                  className="settings-menu-select"
                />
              </div>

              {isMac && (
                <div className="flex items-start justify-between gap-4 pt-1">
                  <div className="min-w-0 flex-1 pr-4">
                    <span className="font-semibold theme-text-main block">Menu Bar Icon:</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      Choose the classic clipboard or the resident Copycat.
                    </p>
                  </div>
                  <MenuSelect
                    value={settings.menubarIconStyle}
                    options={menubarIconOptions}
                    onChange={(value) => onUpdateSettings({ menubarIconStyle: value as AppSettings['menubarIconStyle'] })}
                    label="Menu bar icon"
                    className="settings-menu-select shrink-0"
                  />
                </div>
              )}

              <div className="flex items-start justify-between pt-1">
                <div className="pr-4 flex-1 min-w-0">
                  <span className="font-semibold theme-text-main block">Interaction Sounds:</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    Play subtle audio cues for copy, paste, and navigation actions.
                  </p>
                </div>
                <label className="flex items-center space-x-2 cursor-pointer shrink-0 pt-0.5">
                  <input
                    type="checkbox"
                    checked={settings.enableSounds}
                    onChange={(e) => onUpdateSettings({ enableSounds: e.target.checked })}
                    className="theme-checkbox w-4 h-4 cursor-pointer rounded"
                  />
                  <span className="theme-text-main">Enable Sounds</span>
                </label>
              </div>

              <div className="flex items-start justify-between">
                <div className="pr-4 flex-1 min-w-0">
                  <span className="font-semibold theme-text-main block">Startup Behavior:</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    Automatically launch Pasted when logging into macOS.
                  </p>
                </div>
                <label className="flex items-center space-x-2 cursor-pointer shrink-0 pt-0.5">
                  <input
                    type="checkbox"
                    checked={settings.openAtLogin}
                    onChange={(e) => onUpdateSettings({ openAtLogin: e.target.checked })}
                    className="theme-checkbox w-4 h-4 cursor-pointer rounded"
                  />
                  <span className="theme-text-main">Open at login</span>
                </label>
              </div>
            </div>

            <div className="theme-divider border-t" />

            {/* Clipboard Preferences */}
            <div className="space-y-4">
              <SettingsSubsectionHeader
                title="Clipboard"
                description="Set capture, preview, and history retention behavior."
              />

              <div className="theme-surface overflow-hidden rounded-xl border">
                <div className="flex items-center justify-between gap-4 px-3 py-2.5">
                  <div className="min-w-0">
                    <span className="font-semibold theme-text-main block">Keep clips for</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      Clips older than this move to Trash automatically.
                    </p>
                  </div>
                  <MenuSelect
                    value={String(settings.keepClipAgeDays)}
                    options={historyAgeMenuOptions}
                    onChange={(value) => onUpdateSettings({ keepClipAgeDays: Number(value) })}
                    label="Maximum clip age"
                    className="settings-menu-select w-40 shrink-0"
                  />
                </div>
                <div className="theme-divider flex items-center justify-between gap-4 border-t px-3 py-2.5">
                  <div className="min-w-0">
                    <span className="font-semibold theme-text-main block">Maximum clips</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      The oldest eligible clips are cleaned up first.
                    </p>
                  </div>
                  <MenuSelect
                    value={String(settings.keepClipCount)}
                    options={historyCountOptions}
                    onChange={(value) => onUpdateSettings({ keepClipCount: Number(value) })}
                    label="Maximum clips retained"
                    className="settings-menu-select w-40 shrink-0"
                  />
                </div>
                <p className="theme-divider theme-text-subtle border-t px-3 py-2 text-[10px] leading-normal">
                  Both limits apply. Pinned and protected clips are always kept.
                </p>
              </div>

              <div className="flex items-start justify-between pt-1">
                <div className="pr-4 flex-1 min-w-0">
                  <span className="font-semibold theme-text-main block">Default Paste Behavior:</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    Sets the text formatting output type.
                  </p>
                </div>
                <MenuSelect
                  value={settings.alwaysPastePlainText ? 'plain' : 'rich'}
                  options={pasteBehaviorOptions}
                  onChange={(value) => onUpdateSettings({ alwaysPastePlainText: value === 'plain' })}
                  label="Default paste behavior"
                  className="settings-menu-select"
                />
              </div>

              <div className="flex items-start justify-between">
                <div className="pr-4 flex-1 min-w-0">
                  <span className="font-semibold theme-text-main block">Maximum Clip Size (MB):</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    Ignore copied clippings larger than the specified limit.
                  </p>
                </div>
                <div className="flex items-center space-x-1.5 font-mono shrink-0">
                  <input
                    type="number"
                    min={1}
                    max={256}
                    value={settings.maxClipSizeMb}
                    onChange={(e) => onUpdateSettings({ maxClipSizeMb: Number(e.target.value) })}
                    className="theme-input w-16 border rounded-md px-2 py-1 text-center focus:outline-none"
                  />
                </div>
              </div>

              <div className="flex items-start justify-between gap-4">
                <div className="flex-1 min-w-0">
                  <span className="font-semibold theme-text-main block">File Previews:</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    {filePreviewDescriptions[settings.filePreviewMode]}
                  </p>
                </div>
                <MenuSelect
                  value={settings.filePreviewMode}
                  options={filePreviewOptions}
                  onChange={(value) => onUpdateSettings({ filePreviewMode: value as AppSettings['filePreviewMode'] })}
                  label="File preview behavior"
                  className="settings-menu-select"
                />
              </div>

              {settings.filePreviewMode !== 'off' && (
                <div className="flex items-start justify-between">
                  <div className="pr-4 flex-1 min-w-0">
                    <span className="font-semibold theme-text-main block">Maximum Preview File Size (MB):</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      Files above this size stay as references.
                    </p>
                  </div>
                  <input
                    type="number"
                    min={1}
                    max={64}
                    value={settings.filePreviewMaxMb}
                    onChange={(event) => onUpdateSettings({
                      filePreviewMaxMb: Math.max(1, Math.min(64, Number(event.target.value) || 1)),
                    })}
                    className="theme-input w-16 shrink-0 border rounded-md px-2 py-1 text-center font-mono focus:outline-none"
                  />
                </div>
              )}

              {settings.enableRevisions && <div className="flex items-start justify-between">
                <div className="pr-4 flex-1 min-w-0">
                  <span className="font-semibold theme-text-main block">Revisions per Clip:</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    Keeps complete text snapshots for edits, OCR, Transforms, and restores.
                    {settings.revisionHistoryLimit === 0 && ' Unlimited history can grow quickly when Transforms run automatically.'}
                  </p>
                </div>
                <MenuSelect
                  value={String(settings.revisionHistoryLimit)}
                  options={revisionLimitOptions}
                  onChange={(value) => onUpdateSettings({ revisionHistoryLimit: Number(value) })}
                  label="Revisions retained per clip"
                  className="settings-menu-select"
                />
              </div>}

              <div className="theme-divider border-t pt-4">
                <div className="flex items-start justify-between">
                  <div className="pr-4 flex-1 min-w-0">
                    <span className="font-semibold theme-text-main block">Backup & Export:</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      Export clipboard history to JSON or CSV format.
                    </p>
                  </div>
                  <div className="flex items-center space-x-2 shrink-0">
                    <button
                      type="button"
                      onClick={() => void exportClips('json')}
                      className="theme-status-info flex items-center space-x-1 px-2.5 py-1.5 rounded-lg text-xs font-semibold border transition-colors cursor-pointer"
                    >
                      <Download className="w-3.5 h-3.5" />
                      <span>Export JSON</span>
                    </button>

                    <button
                      type="button"
                      onClick={() => void exportClips('csv')}
                      className="theme-status-info flex items-center space-x-1 px-2.5 py-1.5 rounded-lg text-xs font-semibold border transition-colors cursor-pointer"
                    >
                      <Download className="w-3.5 h-3.5" />
                      <span>Export CSV</span>
                    </button>
                  </div>
                </div>
              </div>
            </div>

            {settings.enableTrash && <>
              <div className="theme-divider border-t" />

              {/* Trash Preferences */}
              <div className="space-y-4">
              <SettingsSubsectionHeader
                title="Trash"
                description="Control how much deleted history remains recoverable."
              />

              <div className="theme-surface overflow-hidden rounded-xl border">
                <div className="flex items-center justify-between gap-4 px-3 py-2.5">
                  <div className="min-w-0">
                    <span className="font-semibold theme-text-main block">Keep trashed clips for</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      Older trashed clips are permanently purged.
                    </p>
                  </div>
                  <MenuSelect
                    value={String(settings.trashAgeDays)}
                    options={trashAgeMenuOptions}
                    onChange={(value) => onUpdateSettings({ trashAgeDays: Number(value) })}
                    label="Maximum Trash age"
                    className="settings-menu-select w-40 shrink-0"
                  />
                </div>
                <div className="theme-divider flex items-center justify-between gap-4 border-t px-3 py-2.5">
                  <div className="min-w-0">
                    <span className="font-semibold theme-text-main block">Maximum trashed clips</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      The oldest eligible items are permanently purged first.
                    </p>
                  </div>
                  <MenuSelect
                    value={String(settings.trashCapacityCount)}
                    options={trashCountOptions}
                    onChange={(value) => onUpdateSettings({ trashCapacityCount: Number(value) })}
                    label="Maximum trashed clips retained"
                    className="settings-menu-select w-40 shrink-0"
                  />
                </div>
                <p className="theme-divider theme-text-subtle border-t px-3 py-2 text-[10px] leading-normal">
                  Both limits apply. Protected clips are always kept.
                </p>
              </div>

              <div className="theme-status-danger p-3 rounded-xl border text-xs">
                <span className="font-bold">Auto-Trash Safety Net: </span>
                {retentionSummary}
              </div>

              <div>
                <div className="flex items-start justify-between">
                  <div className="pr-4 flex-1 min-w-0">
                    <span className="font-semibold theme-danger-text block">
                      {isAltPressed ? 'Delete All Clips:' : 'Trash All Clips:'}
                    </span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      Moves all unpinned and unprotected clips (including clips assigned to Bins) into Trash. Hold Option ⌥ to permanently delete.
                    </p>
                  </div>
                  <button
                    type="button"
                    onClick={(e) => onClearHistory?.(e.altKey)}
                    className="theme-status-danger flex items-center space-x-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold border transition-colors shrink-0 cursor-pointer"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                    <span>{isAltPressed ? 'Delete All Clips' : 'Trash All Clips'}</span>
                  </button>
                </div>
              </div>
              </div>
            </>}

            {settings.enableActivityLog && <>
              <div className="theme-divider border-t" />

              {/* Activity Log Preferences */}
              <div className="space-y-4">
              <SettingsSubsectionHeader
                title="Activity History"
                description="Control how much application activity Pasted retains."
              />

              <div className="theme-surface overflow-hidden rounded-xl border">
                <div className="flex items-center justify-between gap-4 px-3 py-2.5">
                  <div className="min-w-0">
                    <span className="font-semibold theme-text-main block">Keep activity for</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      Older activity entries are removed automatically.
                    </p>
                  </div>
                  <MenuSelect
                    value={String(settings.activityLogAgeDays)}
                    options={activityAgeMenuOptions}
                    onChange={(value) => onUpdateSettings({ activityLogAgeDays: Number(value) })}
                    label="Maximum activity age"
                    className="settings-menu-select w-40 shrink-0"
                  />
                </div>
                <div className="theme-divider flex items-center justify-between gap-4 border-t px-3 py-2.5">
                  <div className="min-w-0">
                    <span className="font-semibold theme-text-main block">Maximum activity entries</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      The oldest entries are removed first.
                    </p>
                  </div>
                  <MenuSelect
                    value={String(settings.activityLogCapacity)}
                    options={activityCountOptions}
                    onChange={(value) => onUpdateSettings({ activityLogCapacity: Number(value) })}
                    label="Maximum activity entries retained"
                    className="settings-menu-select w-40 shrink-0"
                  />
                </div>
                <p className="theme-divider theme-text-subtle border-t px-3 py-2 text-[10px] leading-normal">
                  Both limits apply. Unlimited and Forever disable automatic removal.
                </p>
              </div>
              </div>
            </>}

          </div>
  );
}
