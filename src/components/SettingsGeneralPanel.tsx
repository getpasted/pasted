import { Code2, Coffee, Download, Droplet, Drum, Laptop, Link, Minus, Moon, Palette, Plus, RotateCcw, Sliders, Snowflake, Trash2, Zap } from 'lucide-react';
import type { AppSettings } from '../types';
import { useAltKeyPressed } from '../hooks/useAltKeyPressed';
import { safeInvoke as invoke } from '../utils/tauri';
import { MenuSelect } from './MenuSelect';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsSectionHeading } from './SettingsSectionHeading';
import { ACTUAL_SIZE, APP_ZOOM_STEPS, appZoomPercent, stepAppZoom } from '../utils/appZoom';
import { useToast } from './ToastProvider';

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
  { value: 'vampire', label: 'Vampire', Icon: Droplet },
  { value: 'flux', label: 'Flux', Icon: Zap },
  { value: '808', label: '808', Icon: Drum },
] as const;

const appearanceGroups = [
  { label: 'System', values: ['system'] },
  { label: 'Dark schemes', values: ['dark', 'vampire', 'flux', '808'] },
  { label: 'Light schemes', values: ['cool', 'warm'] },
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

const rowHeightOptions = [
  { value: 'small', label: 'Compact' },
  { value: 'medium', label: 'Standard' },
  { value: 'large', label: 'Spacious' },
];

const contentDetectors: Array<{
  key: 'detectColors' | 'detectLinks' | 'detectCode';
  label: string;
  description: string;
  Icon: typeof Palette;
}> = [
  { key: 'detectColors', label: 'Colors', description: '#RGB, #RRGGBB, RGB, and HSL values.', Icon: Palette },
  { key: 'detectLinks', label: 'Links', description: 'Web and file URLs.', Icon: Link },
  { key: 'detectCode', label: 'Code', description: 'Common source-code patterns and syntax.', Icon: Code2 },
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
              {/* Appearance Mode Switcher */}
              <div className="flex items-center justify-between pb-1">
                <span className="font-medium">
                  Appearance: <strong className="theme-text-muted ml-1">{appearanceModes.find(({ value }) => value === (settings.themeMode || 'system'))?.label}</strong>
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
              <SettingsSectionHeading title="Layout" align="center" />

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

              <div className="theme-divider border-t pt-3">
                <div className="flex items-start justify-between">
                  <div className="pr-4 flex-1 min-w-0">
                    <span className="font-semibold theme-text-main block">Column Widths:</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      Resets the left sidebar and middle history list panel widths to their defaults.
                    </p>
                  </div>
                  <button
                    type="button"
                    onClick={() => {
                      if (onResetColumnWidths) onResetColumnWidths();
                      else {
                        localStorage.removeItem('pasted_sidebar_width');
                        localStorage.removeItem('pasted_list_width');
                        window.location.reload();
                      }
                    }}
                    className="theme-secondary-button flex items-center space-x-1.5 px-3 py-1.5 border rounded-lg text-xs font-semibold transition-colors shrink-0 cursor-pointer"
                  >
                    <RotateCcw className="w-3.5 h-3.5" />
                    <span>Reset Column Widths</span>
                  </button>
                </div>
              </div>
            </div>

            <div className="theme-divider border-t" />

            {/* System & OS Integration Subsection */}
            <div className="space-y-4">
              <SettingsSectionHeading title="System & OS Integration" align="center" />

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

            {settings.enableContentDetection && <>
            <div className="theme-divider border-t" />

            <div className="space-y-4">
              <SettingsSectionHeading title="Content Detection" align="center" />

              <div className="theme-surface overflow-hidden rounded-xl border">
                {contentDetectors.map(({ key, label, description, Icon }, index) => (
                  <label
                    key={key}
                    className={`flex cursor-pointer items-center gap-3 px-3 py-2.5 ${index > 0 ? 'theme-divider border-t' : ''}`}
                  >
                    <span className="settings-accent-tile flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border">
                      <Icon className="h-4 w-4" />
                    </span>
                    <span className="min-w-0 flex-1">
                      <strong className="theme-text-main block text-xs">{label}</strong>
                      <span className="theme-text-muted block text-[10px]">{description}</span>
                    </span>
                    <input
                      type="checkbox"
                      checked={settings[key]}
                      onChange={(event) => onUpdateSettings({ [key]: event.target.checked })}
                      className="theme-checkbox h-4 w-4 shrink-0 cursor-pointer rounded"
                    />
                  </label>
                ))}
              </div>
            </div>
            </>}

            <div className="theme-divider border-t" />

            {/* Clipboard Preferences */}
            <div className="space-y-4">
              <SettingsSectionHeading title="Clipboard" align="center" />

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

              {/* Compact Keep Clippings Input + Slider */}
              <div className="space-y-2">
                <div className="flex items-start justify-between">
                  <div className="pr-4 flex-1 min-w-0">
                    <span className="font-semibold theme-text-main block">History Capacity (clips):</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
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
                      className="theme-input w-20 border rounded-md px-2 py-1 text-center font-bold focus:outline-none"
                    />
                  </div>
                </div>

                <div className="flex items-center space-x-3 pt-1">
                  <span className="text-[10px] theme-text-subtle font-mono">50</span>
                  <input
                    type="range"
                    min={50}
                    max={3000}
                    step={50}
                    value={settings.keepClipCount}
                    onChange={(e) => onUpdateSettings({ keepClipCount: Number(e.target.value) })}
                    className="theme-range flex-1 h-1.5 rounded-lg cursor-pointer"
                  />
                  <span className="text-[10px] theme-text-subtle font-mono">3000</span>
                </div>
              </div>

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

              <div className="theme-divider pt-3 border-t">
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
              <SettingsSectionHeading title="Trash" align="center" />

              <div>
                <div className="flex items-start justify-between">
                  <div className="pr-4 flex-1 min-w-0">
                    <span className="font-semibold theme-text-main block">Trash Capacity Limit:</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
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
                      className="theme-input w-20 border rounded-md px-2 py-1 text-center font-bold focus:outline-none text-xs"
                    />
                  </div>
                </div>

                <div className="flex items-center space-x-3 pt-2">
                  <span className="text-[10px] theme-text-subtle font-mono">50</span>
                  <input
                    type="range"
                    min={50}
                    max={2000}
                    step={50}
                    value={settings.trashCapacityCount ?? 500}
                    onChange={(e) => onUpdateSettings({ trashCapacityCount: Number(e.target.value) })}
                    className="theme-range settings-danger-range flex-1 h-1.5 rounded-lg cursor-pointer"
                  />
                  <span className="text-[10px] theme-text-subtle font-mono">2000</span>
                </div>
              </div>

              <div className="theme-status-danger p-3 rounded-xl border text-xs">
                <span className="font-bold">Auto-Trash Safety Net: </span>
                When your active history reaches your clip limit ({settings.keepClipCount} clips), older unpinned items automatically move into Trash instead of dropping off forever.
              </div>

              <div className="theme-divider pt-3 border-t">
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
              <SettingsSectionHeading title="Activity History" align="center" />

              <div>
                <div className="flex items-start justify-between">
                  <div className="pr-4 flex-1 min-w-0">
                    <span className="font-semibold theme-text-main block">Log Capacity Limit:</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
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
                      className="theme-input w-20 border rounded-md px-2 py-1 text-center font-bold focus:outline-none text-xs"
                    />
                  </div>
                </div>
              </div>
              </div>
            </>}

          </div>
  );
}
