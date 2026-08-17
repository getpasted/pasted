import { useState } from 'react';
import { Building2, Coffee, Droplet, Drum, Laptop, Minus, Moon, Pizza, Plus, RotateCcw, Sliders, Snowflake, Trash2, Zap } from 'lucide-react';
import type { AppSettings } from '../types';
import { useAltKeyPressed } from '../hooks/useAltKeyPressed';
import { MenuSelect } from './MenuSelect';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
import { ACTUAL_SIZE, APP_ZOOM_STEPS, appZoomPercent, stepAppZoom } from '../utils/appZoom';
import { useToast } from './ToastProvider';
import { ActionButton } from './AppDialogLayout';
import { useLocalization } from '../localization/LocalizationProvider';
import { translate } from '../localization/runtime';

interface SettingsGeneralPanelProps {
  settings: AppSettings;
  onUpdateSettings: (newSettings: Partial<AppSettings>) => void;
  onClearHistory?: (permanent: boolean) => void;
  onRestoreAllTrashedClips?: () => Promise<number>;
  trashedClipCount?: number;
  onResetColumnWidths?: () => void;
}

const appearanceModes = [
  { value: 'system', get label() { return translate('common.system'); }, Icon: Laptop },
  { value: 'dark', get label() { return translate('component.settingsGeneralPanel.dark'); }, Icon: Moon },
  { value: 'cool', get label() { return translate('component.settingsGeneralPanel.cool'); }, Icon: Snowflake },
  { value: 'warm', get label() { return translate('component.settingsGeneralPanel.warm'); }, Icon: Coffee },
  { value: '2894', label: '2894', Icon: Building2 },
  { value: 'sauced', get label() { return translate('component.settingsGeneralPanel.sauced'); }, Icon: Pizza },
  { value: 'vampire', get label() { return translate('component.settingsGeneralPanel.vampire'); }, Icon: Droplet },
  { value: 'flux', get label() { return translate('component.settingsGeneralPanel.flux'); }, Icon: Zap },
  { value: '808', label: '808', Icon: Drum },
] as const;

const appearanceGroups = [
  { get label() { return translate('common.system'); }, values: ['system'] },
  { get label() { return translate('component.settingsGeneralPanel.darkSchemes'); }, values: ['dark', 'vampire', 'flux', '808'] },
  { get label() { return translate('component.settingsGeneralPanel.lightSchemes'); }, values: ['cool', 'warm', '2894', 'sauced'] },
] as const;

const pasteBehaviorOptions = [
  { value: 'rich', get label() { return translate('component.settingsGeneralPanel.preserveFormattingDefault'); } },
  { value: 'plain', get label() { return translate('component.settingsGeneralPanel.alwaysPastePlainText'); } },
];

const filePreviewOptions = [
  { value: 'off', get label() { return translate('component.settingsGeneralPanel.off'); } },
  { value: 'safe', get label() { return translate('component.settingsGeneralPanel.safeTypes'); } },
  { value: 'all', get label() { return translate('component.settingsGeneralPanel.allSupported'); } },
];

const filePreviewDescriptions: Record<AppSettings['filePreviewMode'], string> = {
  get off() { return translate('component.settingsGeneralPanel.filePreviewOffDescription'); },
  get safe() { return translate('component.settingsGeneralPanel.filePreviewSafeDescription'); },
  get all() { return translate('component.settingsGeneralPanel.filePreviewAllDescription'); },
};

const revisionLimitOptions = [10, 25, 50, 100]
  .map((value) => ({ value: String(value), label: translate('component.settingsGeneralPanel.valueRevisions', { value: value }) }))
  .concat({ value: '0', get label() { return translate('component.settingsGeneralPanel.unlimited'); } });

const historyCountPresets = [
  { value: '0', get label() { return translate('component.settingsGeneralPanel.unlimited'); } },
  { value: '250', get label() { return translate('component.settingsGeneralPanel.value250Clips'); } },
  { value: '500', get label() { return translate('component.settingsGeneralPanel.value500Clips'); } },
  { value: '1000', get label() { return translate('component.settingsGeneralPanel.value1000ClipsDefault'); } },
  { value: '5000', get label() { return translate('component.settingsGeneralPanel.value5000Clips'); } },
  { value: '10000', get label() { return translate('component.settingsGeneralPanel.value10000Clips'); } },
  { value: '50000', get label() { return translate('component.settingsGeneralPanel.value50000Clips'); } },
];

const trashCountPresets = [
  { value: '0', get label() { return translate('component.settingsGeneralPanel.unlimited'); } },
  { value: '100', get label() { return translate('component.settingsGeneralPanel.value100Clips'); } },
  { value: '250', get label() { return translate('component.settingsGeneralPanel.value250Clips'); } },
  { value: '500', get label() { return translate('component.settingsGeneralPanel.value500ClipsDefault'); } },
  { value: '1000', get label() { return translate('component.settingsGeneralPanel.value1000Clips'); } },
  { value: '2000', get label() { return translate('component.settingsGeneralPanel.value2000Clips'); } },
  { value: '5000', get label() { return translate('component.settingsGeneralPanel.value5000Clips'); } },
];

const activityCountPresets = [
  { value: '0', get label() { return translate('component.settingsGeneralPanel.unlimited'); } },
  { value: '250', get label() { return translate('component.settingsGeneralPanel.value250Entries'); } },
  { value: '500', get label() { return translate('component.settingsGeneralPanel.value500Entries'); } },
  { value: '1000', get label() { return translate('component.settingsGeneralPanel.value1000EntriesDefault'); } },
  { value: '2500', get label() { return translate('component.settingsGeneralPanel.value2500Entries'); } },
  { value: '5000', get label() { return translate('component.settingsGeneralPanel.value5000Entries'); } },
  { value: '10000', get label() { return translate('component.settingsGeneralPanel.value10000Entries'); } },
];

const retentionAgeOptions = [
  { value: '0', get label() { return translate('component.settingsGeneralPanel.forever'); } },
  { value: '1', get label() { return translate('component.settingsGeneralPanel.value1Day'); } },
  { value: '7', get label() { return translate('component.settingsGeneralPanel.value7Days'); } },
  { value: '30', get label() { return translate('component.settingsGeneralPanel.value30Days'); } },
  { value: '90', get label() { return translate('component.settingsGeneralPanel.value90Days'); } },
  { value: '365', get label() { return translate('component.settingsGeneralPanel.value1Year'); } },
];

const rowHeightOptions = [
  { value: 'small', get label() { return translate('component.settingsGeneralPanel.compact'); } },
  { value: 'medium', get label() { return translate('component.settingsGeneralPanel.standard'); } },
  { value: 'large', get label() { return translate('component.settingsGeneralPanel.spacious'); } },
];

const startupViewOptions = [
  { value: 'last_active', get label() { return translate('component.settingsGeneralPanel.lastActivePage'); } },
  { value: 'clip_history', get label() { return translate('component.settingsGeneralPanel.clipHistory'); } },
];

export function SettingsGeneralPanel({
  settings,
  onUpdateSettings,
  onClearHistory,
  onRestoreAllTrashedClips,
  trashedClipCount = 0,
  onResetColumnWidths,
}: SettingsGeneralPanelProps) {
  const { showToast } = useToast();
  const { t, locales } = useLocalization();
  const [isRestoringTrash, setIsRestoringTrash] = useState(false);
  const isAltPressed = useAltKeyPressed();
  const isMac = typeof navigator !== 'undefined' && /Mac|iPod|iPhone|iPad/.test(navigator.userAgent || navigator.platform);
  const dockIconOptions = isMac
    ? [
        { value: 'auto_hide', get label() { return translate('component.settingsGeneralPanel.autoHideDockIcon'); } },
        { value: 'both', get label() { return translate('component.settingsGeneralPanel.alwaysShowDockAndMenuBar'); } },
        { value: 'menubar_only', get label() { return translate('component.settingsGeneralPanel.menubarIconOnly'); } },
      ]
    : [
        { value: 'auto_hide', get label() { return translate('component.settingsGeneralPanel.autoHideTaskbarIcon'); } },
        { value: 'both', get label() { return translate('component.settingsGeneralPanel.alwaysShowTrayAndTaskbar'); } },
        { value: 'menubar_only', get label() { return translate('component.settingsGeneralPanel.systemTrayIconOnly'); } },
      ];
  const menubarIconOptions = [
    { value: 'clipboard', get label() { return translate('component.settingsGeneralPanel.clipboard'); } },
    { value: 'copycat', get label() { return translate('component.settingsGeneralPanel.copycat'); } },
  ];
  const historyCountOptions = historyCountPresets.some(({ value }) => Number(value) === settings.keepClipCount)
    ? historyCountPresets
    : [
        ...historyCountPresets.slice(0, 1),
        { value: String(settings.keepClipCount), label: t('format.customValue', { value: t('format.clipCount', { count: settings.keepClipCount }), custom: t('common.custom') }) },
        ...historyCountPresets.slice(1),
      ];
  const historyAgeMenuOptions = retentionAgeOptions.some(({ value }) => Number(value) === settings.keepClipAgeDays)
    ? retentionAgeOptions
    : [
        ...retentionAgeOptions.slice(0, 1),
        { value: String(settings.keepClipAgeDays), label: t('format.customValue', { value: t('format.dayCount', { count: settings.keepClipAgeDays }), custom: t('common.custom') }) },
        ...retentionAgeOptions.slice(1),
      ];
  const trashCountOptions = trashCountPresets.some(({ value }) => Number(value) === settings.trashCapacityCount)
    ? trashCountPresets
    : [
        ...trashCountPresets.slice(0, 1),
        { value: String(settings.trashCapacityCount), label: t('format.customValue', { value: t('format.clipCount', { count: settings.trashCapacityCount }), custom: t('common.custom') }) },
        ...trashCountPresets.slice(1),
      ];
  const trashAgeMenuOptions = retentionAgeOptions.some(({ value }) => Number(value) === settings.trashAgeDays)
    ? retentionAgeOptions
    : [
        ...retentionAgeOptions.slice(0, 1),
        { value: String(settings.trashAgeDays), label: t('format.customValue', { value: t('format.dayCount', { count: settings.trashAgeDays }), custom: t('common.custom') }) },
        ...retentionAgeOptions.slice(1),
      ];
  const activityCountOptions = activityCountPresets.some(({ value }) => Number(value) === settings.activityLogCapacity)
    ? activityCountPresets
    : [
        ...activityCountPresets.slice(0, 1),
        { value: String(settings.activityLogCapacity), label: t('format.customValue', { value: t('format.entryCount', { count: settings.activityLogCapacity }), custom: t('common.custom') }) },
        ...activityCountPresets.slice(1),
      ];
  const activityAgeMenuOptions = retentionAgeOptions.some(({ value }) => Number(value) === settings.activityLogAgeDays)
    ? retentionAgeOptions
    : [
        ...retentionAgeOptions.slice(0, 1),
        { value: String(settings.activityLogAgeDays), label: t('format.customValue', { value: t('format.dayCount', { count: settings.activityLogAgeDays }), custom: t('common.custom') }) },
        ...retentionAgeOptions.slice(1),
      ];
  const restoreAllTrashedClips = async () => {
    if (!onRestoreAllTrashedClips || isRestoringTrash) return;
    setIsRestoringTrash(true);
    try {
      const restoredCount = await onRestoreAllTrashedClips();
      showToast({
        tone: 'success',
        message: translate('component.settingsGeneralPanel.restoredCountClipsFromTrash', { count: restoredCount }),
      });
    } catch (error) {
      console.error('Failed to restore Trash:', error);
      showToast({ tone: 'error', get message() { return translate('component.settingsGeneralPanel.couldNotRestoreClipsFromTrash'); } });
    } finally {
      setIsRestoringTrash(false);
    }
  };

  return (
          <div className="space-y-5 text-xs">
            <SettingsPanelHeader
              icon={Sliders}
              title={translate('component.settingsGeneralPanel.general')}
              description={translate('component.settingsGeneralPanel.appearanceClipboardBehaviorAndHistory')}
            />
            {/* General Preferences */}
            <div className="space-y-4">
              <SettingsSubsectionHeader
                title={translate('component.settingsGeneralPanel.appearance')}
                description={translate('component.settingsGeneralPanel.chooseAColorSchemeAndDisplayScale')}
              />

              {/* Appearance Mode Switcher */}
              <div className="flex items-center justify-between pb-1">
                <span className="font-medium">
                  {translate('component.settingsGeneralPanel.colorScheme')} <strong className="theme-text-muted ms-1">{appearanceModes.find(({ value }) => value === (settings.themeMode || 'system'))?.label}</strong>
                </span>
                <div className="theme-surface appearance-picker flex items-center p-1 rounded-xl border gap-1" role="group" aria-label={translate('component.settingsGeneralPanel.appearanceScheme')}>
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
                            aria-label={translate('component.settingsGeneralPanel.labelAppearance', { label: mode.label })}
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

              <div className="flex items-start justify-between gap-4">
                <div className="min-w-0 flex-1 pe-4">
                  <span className="font-semibold theme-text-main block">{t('settings.general.language.label')}</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    {t('settings.general.language.description')}
                  </p>
                </div>
                <MenuSelect
                  value={settings.language}
                  options={[
                    { value: 'system', label: t('common.automatic') },
                    ...locales.map(({ code, nativeName }, index) => ({ value: code, label: nativeName, dividerBefore: index === 0 })),
                  ]}
                  onChange={(value) => onUpdateSettings({ language: value })}
                  label={t('settings.general.language.ariaLabel')}
                  className="settings-menu-select shrink-0"
                />
              </div>
            </div>

            <div className="theme-divider border-t" />

            <div className="space-y-4">
              <SettingsSubsectionHeader
                title={translate('component.settingsGeneralPanel.layout')}
                description={translate('component.settingsGeneralPanel.adjustAppScalingClipDensityAndWorkspaceDimensions')}
              />

              <div className="flex items-start justify-between gap-4">
                <div className="min-w-0 flex-1">
                  <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.zoom')}</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    {translate('component.settingsGeneralPanel.adjustTheSizeOfNavigationControlsAndClipContent')}
                  </p>
                </div>
                <div className="theme-surface flex shrink-0 items-center overflow-hidden rounded-lg border" role="group" aria-label={translate('component.settingsGeneralPanel.applicationZoom')}>
                  <button
                    type="button"
                    aria-label={translate('component.settingsGeneralPanel.zoomOut')}
                    title={translate('component.settingsGeneralPanel.zoomOut2')}
                    disabled={settings.textSize <= APP_ZOOM_STEPS[0]}
                    onClick={() => onUpdateSettings({ textSize: stepAppZoom(settings.textSize, -1) })}
                    className="theme-secondary-button flex h-8 w-8 items-center justify-center border-0 border-e disabled:cursor-not-allowed disabled:opacity-35"
                  >
                    <Minus className="h-3.5 w-3.5" />
                  </button>
                  <button
                    type="button"
                    aria-label={translate('component.settingsGeneralPanel.actualSize')}
                    title={translate('component.settingsGeneralPanel.actualSize0')}
                    onClick={() => onUpdateSettings({ textSize: ACTUAL_SIZE })}
                    className="theme-secondary-button h-8 min-w-14 border-0 px-2 font-mono text-[10px] font-semibold"
                  >
                    {appZoomPercent(settings.textSize)}%
                  </button>
                  <button
                    type="button"
                    aria-label={translate('component.settingsGeneralPanel.zoomIn')}
                    title={translate('component.settingsGeneralPanel.zoomIn2')}
                    disabled={settings.textSize >= APP_ZOOM_STEPS[APP_ZOOM_STEPS.length - 1]}
                    onClick={() => onUpdateSettings({ textSize: stepAppZoom(settings.textSize, 1) })}
                    className="theme-secondary-button flex h-8 w-8 items-center justify-center border-0 border-s disabled:cursor-not-allowed disabled:opacity-35"
                  >
                    <Plus className="h-3.5 w-3.5" />
                  </button>
                </div>
              </div>

              <div className="flex items-start justify-between">
                <div className="pe-4 flex-1 min-w-0">
                  <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.clipDensity')}</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    {translate('component.settingsGeneralPanel.adjustsClipSpacingTextDepthAndPreviewSizeThroughoutTheHistoryList')}
                  </p>
                </div>
                <MenuSelect
                  value={settings.rowHeight}
                  options={rowHeightOptions}
                  onChange={(value) => onUpdateSettings({ rowHeight: value as AppSettings['rowHeight'] })}
                  label={translate('component.settingsGeneralPanel.clipDensity2')}
                  className="settings-menu-select"
                />
              </div>

              <div className="flex items-start justify-between gap-4">
                <div className="min-w-0 flex-1">
                  <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.startupView')}</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    {translate('component.settingsGeneralPanel.reopenTheLastViewOrAlwaysStartInClipHistory')}
                  </p>
                </div>
                <MenuSelect
                  value={settings.startupView}
                  options={startupViewOptions}
                  onChange={(value) => onUpdateSettings({ startupView: value as AppSettings['startupView'] })}
                  label={translate('component.settingsGeneralPanel.startupView2')}
                  className="settings-menu-select"
                />
              </div>

              <div className="flex items-start justify-between">
                <div className="pe-4 flex-1 min-w-0">
                  <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.columnWidths')}</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    {translate('component.settingsGeneralPanel.resetsTheLeftSidebarAndMiddleHistoryListPanelWidthsToTheir')}
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
                  <span>{translate('component.settingsGeneralPanel.resetColumnWidths')}</span>
                </ActionButton>
              </div>
            </div>

            <div className="theme-divider border-t" />

            {/* System integration subsection */}
            <div className="space-y-4">
              <SettingsSubsectionHeader
                title={translate('component.settingsGeneralPanel.systemIntegration')}
                description={translate('component.settingsGeneralPanel.controlStartupSoundsAndOperatingSystemIntegration')}
              />

              {/* Dock / Menubar / System Tray Setting */}
              <div className="flex items-center justify-between pt-1">
                <span className="font-medium">
                  {isMac
                    ? translate('component.settingsGeneralPanel.dockAndMenuBarIcon')
                    : translate('component.settingsGeneralPanel.systemTrayAndTaskbar')}
                </span>
                <MenuSelect
                  value={settings.dockMenubarIcon}
                  options={dockIconOptions}
                  onChange={(value) => onUpdateSettings({ dockMenubarIcon: value as AppSettings['dockMenubarIcon'] })}
                  label={translate('component.settingsGeneralPanel.dockAndMenuBarIconBehavior')}
                  className="settings-menu-select"
                />
              </div>

              {isMac && (
                <div className="flex items-start justify-between gap-4 pt-1">
                  <div className="min-w-0 flex-1 pe-4">
                    <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.menuBarIcon')}</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      {translate('component.settingsGeneralPanel.chooseTheClassicClipboardOrTheResidentCopycat')}
                    </p>
                  </div>
                  <MenuSelect
                    value={settings.menubarIconStyle}
                    options={menubarIconOptions}
                    onChange={(value) => onUpdateSettings({ menubarIconStyle: value as AppSettings['menubarIconStyle'] })}
                    label={translate('component.settingsGeneralPanel.menuBarIcon2')}
                    className="settings-menu-select shrink-0"
                  />
                </div>
              )}

              <div className="flex items-start justify-between pt-1">
                <div className="pe-4 flex-1 min-w-0">
                  <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.interactionSounds')}</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    {translate('component.settingsGeneralPanel.playSubtleAudioCuesForCopyPasteAndNavigationActions')}
                  </p>
                </div>
                <label className="flex items-center space-x-2 cursor-pointer shrink-0 pt-0.5">
                  <input
                    type="checkbox"
                    checked={settings.enableSounds}
                    onChange={(e) => onUpdateSettings({ enableSounds: e.target.checked })}
                    className="theme-checkbox w-4 h-4 cursor-pointer rounded"
                  />
                  <span className="theme-text-main">{translate('component.settingsGeneralPanel.enableSounds')}</span>
                </label>
              </div>

              <div className="flex items-start justify-between">
                <div className="pe-4 flex-1 min-w-0">
                  <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.startupBehavior')}</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    {translate('component.settingsGeneralPanel.launchAutomaticallyAfterLoggingIntoMacos')}
                  </p>
                </div>
                <label className="flex items-center space-x-2 cursor-pointer shrink-0 pt-0.5">
                  <input
                    type="checkbox"
                    checked={settings.openAtLogin}
                    onChange={(e) => onUpdateSettings({ openAtLogin: e.target.checked })}
                    className="theme-checkbox w-4 h-4 cursor-pointer rounded"
                  />
                  <span className="theme-text-main">{translate('component.settingsGeneralPanel.openAtLogin')}</span>
                </label>
              </div>
            </div>

            <div className="theme-divider border-t" />

            {/* Clipboard Preferences */}
            <div className="space-y-4">
              <SettingsSubsectionHeader
                title={translate('component.settingsGeneralPanel.clipboard')}
                description={translate('component.settingsGeneralPanel.setCapturePreviewAndHistoryRetentionBehavior')}
              />

              <div className="theme-surface overflow-hidden rounded-xl border">
                <div className="flex items-center justify-between gap-4 px-3 py-2.5">
                  <div className="min-w-0">
                    <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.keepClipsFor')}</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      {translate('component.settingsGeneralPanel.eligibleClipsOlderThanThisMoveToTrashAutomatically')}
                    </p>
                  </div>
                  <MenuSelect
                    value={String(settings.keepClipAgeDays)}
                    options={historyAgeMenuOptions}
                    onChange={(value) => onUpdateSettings({ keepClipAgeDays: Number(value) })}
                    label={translate('component.settingsGeneralPanel.maximumClipAge')}
                    className="settings-menu-select w-40 shrink-0"
                  />
                </div>
                <div className="theme-divider flex items-center justify-between gap-4 border-t px-3 py-2.5">
                  <div className="min-w-0">
                    <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.maximumClips')}</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      {translate('component.settingsGeneralPanel.theOldestEligibleClipsMoveToTrashFirst')}
                    </p>
                  </div>
                  <MenuSelect
                    value={String(settings.keepClipCount)}
                    options={historyCountOptions}
                    onChange={(value) => onUpdateSettings({ keepClipCount: Number(value) })}
                    label={translate('component.settingsGeneralPanel.maximumClipsRetained')}
                    className="settings-menu-select w-40 shrink-0"
                  />
                </div>
                <p className="theme-divider theme-text-subtle border-t px-3 py-2 text-[10px] leading-normal">
                  {translate('component.settingsGeneralPanel.bothLimitsApplyPinnedAndProtectedClipsNeverMoveToTrashAutomatically')}
                </p>
              </div>

              <div className="flex items-start justify-between pt-1">
                <div className="pe-4 flex-1 min-w-0">
                  <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.defaultPasteBehavior')}</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    {translate('component.settingsGeneralPanel.setsTheTextFormattingOutputType')}
                  </p>
                </div>
                <MenuSelect
                  value={settings.alwaysPastePlainText ? 'plain' : 'rich'}
                  options={pasteBehaviorOptions}
                  onChange={(value) => onUpdateSettings({ alwaysPastePlainText: value === 'plain' })}
                  label={translate('component.settingsGeneralPanel.defaultPasteBehavior2')}
                  className="settings-menu-select"
                />
              </div>

              <div className="flex items-start justify-between">
                <div className="pe-4 flex-1 min-w-0">
                  <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.maximumClipSizeMb')}</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    {translate('component.settingsGeneralPanel.ignoreCopiedClipsLargerThanTheSpecifiedLimit')}
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
                  <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.filePreviews')}</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                    {filePreviewDescriptions[settings.filePreviewMode]}
                  </p>
                </div>
                <MenuSelect
                  value={settings.filePreviewMode}
                  options={filePreviewOptions}
                  onChange={(value) => onUpdateSettings({ filePreviewMode: value as AppSettings['filePreviewMode'] })}
                  label={translate('component.settingsGeneralPanel.filePreviewBehavior')}
                  className="settings-menu-select"
                />
              </div>

              {settings.filePreviewMode !== 'off' && (
                <div className="flex items-start justify-between">
                  <div className="pe-4 flex-1 min-w-0">
                    <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.maximumPreviewFileSizeMb')}</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      {translate('component.settingsGeneralPanel.filesAboveThisSizeStayAsReferences')}
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
                <div className="pe-4 flex-1 min-w-0">
                  <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.revisionsPerClip')}</span>
                  <p className="text-[11px] theme-text-muted leading-normal mt-0.5">{settings.revisionHistoryLimit === 0
                    ? translate('component.settingsGeneralPanel.unlimitedRevisionHistoryDescription')
                    : translate('component.settingsGeneralPanel.keepsCompleteTextSnapshotsForEditsOcrTransformsAndRestores')}</p>
                </div>
                <MenuSelect
                  value={String(settings.revisionHistoryLimit)}
                  options={revisionLimitOptions}
                  onChange={(value) => onUpdateSettings({ revisionHistoryLimit: Number(value) })}
                  label={translate('component.settingsGeneralPanel.revisionsRetainedPerClip')}
                  className="settings-menu-select"
                />
              </div>}

            </div>

            {settings.enableTrash && <>
              <div className="theme-divider border-t" />

              {/* Trash Preferences */}
              <div className="space-y-4">
              <SettingsSubsectionHeader
                title={translate('component.settingsGeneralPanel.trash')}
                description={translate('component.settingsGeneralPanel.controlHowMuchDeletedHistoryRemainsRecoverable')}
              />

              <div className="theme-surface overflow-hidden rounded-xl border">
                <div className="flex items-center justify-between gap-4 px-3 py-2.5">
                  <div className="min-w-0">
                    <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.keepTrashedClipsFor')}</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      {translate('component.settingsGeneralPanel.olderTrashedClipsArePermanentlyPurged')}
                    </p>
                  </div>
                  <MenuSelect
                    value={String(settings.trashAgeDays)}
                    options={trashAgeMenuOptions}
                    onChange={(value) => onUpdateSettings({ trashAgeDays: Number(value) })}
                    label={translate('component.settingsGeneralPanel.maximumTrashAge')}
                    className="settings-menu-select w-40 shrink-0"
                  />
                </div>
                <div className="theme-divider flex items-center justify-between gap-4 border-t px-3 py-2.5">
                  <div className="min-w-0">
                    <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.maximumTrashedClips')}</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      {translate('component.settingsGeneralPanel.theOldestEligibleItemsArePermanentlyPurgedFirst')}
                    </p>
                  </div>
                  <MenuSelect
                    value={String(settings.trashCapacityCount)}
                    options={trashCountOptions}
                    onChange={(value) => onUpdateSettings({ trashCapacityCount: Number(value) })}
                    label={translate('component.settingsGeneralPanel.maximumTrashedClipsRetained')}
                    className="settings-menu-select w-40 shrink-0"
                  />
                </div>
                <p className="theme-divider theme-text-subtle border-t px-3 py-2 text-[10px] leading-normal">
                  {translate('component.settingsGeneralPanel.bothLimitsApplyProtectedClipsAreAlwaysKept')}
                </p>
              </div>

              <div className="theme-surface overflow-hidden rounded-xl border">
                <div className="flex items-start justify-between gap-4 px-3 py-2.5">
                  <div className="min-w-0">
                    <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.restoreTrashedClips')}</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      {translate('component.settingsGeneralPanel.returnEveryTrashedClipToHistory')}
                    </p>
                  </div>
                  <div className="shrink-0">
                    <ActionButton
                      onClick={() => void restoreAllTrashedClips()}
                      disabled={!onRestoreAllTrashedClips || trashedClipCount === 0 || isRestoringTrash}
                      className="disabled:cursor-not-allowed disabled:opacity-40"
                    >
                      <RotateCcw className="h-3.5 w-3.5" />
                      <span>{isRestoringTrash ? translate('component.settingsGeneralPanel.restoring') : translate('component.settingsGeneralPanel.restoreTrashedClips')}</span>
                    </ActionButton>
                  </div>
                </div>
                <div className="theme-divider flex items-start justify-between gap-4 border-t px-3 py-2.5">
                  <div className="min-w-0">
                    <span className={`font-semibold block ${isAltPressed ? 'theme-danger-text' : 'theme-text-main'}`}>
                      {isAltPressed ? translate('component.settingsGeneralPanel.deleteAllClips') : translate('component.settingsGeneralPanel.trashAllClips')}
                    </span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      {isAltPressed
                        ? translate('component.settingsGeneralPanel.permanentlyDeleteAllUnpinnedAndUnprotectedClips')
                        : translate('component.settingsGeneralPanel.moveAllUnpinnedAndUnprotectedClipsToTrashHoldOptionToPermanently')}
                    </p>
                  </div>
                  <button
                    type="button"
                    onClick={(e) => onClearHistory?.(e.altKey)}
                    className="theme-status-danger flex shrink-0 items-center space-x-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold border transition-colors cursor-pointer"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                    <span>{isAltPressed ? translate('component.settingsGeneralPanel.deleteAllClips2') : translate('component.settingsGeneralPanel.trashAllClips2')}</span>
                  </button>
                </div>
              </div>
              </div>
            </>}

            {settings.enableActivityLog && <>
              <div className="theme-divider border-t" />

              {/* Activity preferences */}
              <div className="space-y-4">
              <SettingsSubsectionHeader
                title={translate('component.settingsGeneralPanel.activityHistory')}
                description={translate('component.settingsGeneralPanel.chooseHowMuchActivityHistoryToKeep')}
              />

              <div className="theme-surface overflow-hidden rounded-xl border">
                <div className="flex items-center justify-between gap-4 px-3 py-2.5">
                  <div className="min-w-0">
                    <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.keepActivityFor')}</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      {translate('component.settingsGeneralPanel.olderActivityEntriesAreRemovedAutomatically')}
                    </p>
                  </div>
                  <MenuSelect
                    value={String(settings.activityLogAgeDays)}
                    options={activityAgeMenuOptions}
                    onChange={(value) => onUpdateSettings({ activityLogAgeDays: Number(value) })}
                    label={translate('component.settingsGeneralPanel.maximumActivityAge')}
                    className="settings-menu-select w-40 shrink-0"
                  />
                </div>
                <div className="theme-divider flex items-center justify-between gap-4 border-t px-3 py-2.5">
                  <div className="min-w-0">
                    <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.maximumActivityEntries')}</span>
                    <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
                      {translate('component.settingsGeneralPanel.theOldestEntriesAreRemovedFirst')}
                    </p>
                  </div>
                  <MenuSelect
                    value={String(settings.activityLogCapacity)}
                    options={activityCountOptions}
                    onChange={(value) => onUpdateSettings({ activityLogCapacity: Number(value) })}
                    label={translate('component.settingsGeneralPanel.maximumActivityEntriesRetained')}
                    className="settings-menu-select w-40 shrink-0"
                  />
                </div>
                <p className="theme-divider theme-text-subtle border-t px-3 py-2 text-[10px] leading-normal">
                  {translate('component.settingsGeneralPanel.bothLimitsApplyUnlimitedAndForeverDisableAutomaticRemoval')}
                </p>
              </div>

              </div>
            </>}

          </div>
  );
}
