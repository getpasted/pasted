import { useState } from 'react';
import { Sliders } from 'lucide-react';
import type { AppSettings } from '../types';
import { useAltKeyPressed } from '../hooks/useAltKeyPressed';
import { MenuSelect } from './MenuSelect';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
import { useToast } from './ToastProvider';
import { useLocalization } from '../localization/LocalizationProvider';
import { translate } from '../localization/runtime';
import { SettingsGeneralAppearanceSection } from './SettingsGeneralAppearanceSection';
import { SettingsGeneralLayoutSection } from './SettingsGeneralLayoutSection';
import { SettingsGeneralRetentionSections } from './SettingsGeneralRetentionSections';
import { SettingsGeneralHistoryLimits } from './SettingsGeneralHistoryLimits';
import { SettingsGeneralResetFooter } from './SettingsGeneralResetFooter';

interface SettingsGeneralPanelProps {
  settings: AppSettings;
  onUpdateSettings: (newSettings: Partial<AppSettings>) => void;
  onClearHistory?: (permanent: boolean) => void;
  onRestoreAllTrashedClips?: () => Promise<number>;
  trashedClipCount?: number;
  onResetColumnWidths?: () => void;
}

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
            <SettingsGeneralAppearanceSection settings={settings} onUpdateSettings={onUpdateSettings} />

            <div className="theme-divider border-t" />

            <SettingsGeneralLayoutSection
              settings={settings}
              onUpdateSettings={onUpdateSettings}
              onResetColumnWidths={onResetColumnWidths}
            />

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

              <SettingsGeneralHistoryLimits settings={settings} onUpdateSettings={onUpdateSettings} />

            </div>

            <SettingsGeneralRetentionSections
              settings={settings}
              trashAgeOptions={trashAgeMenuOptions}
              trashCountOptions={trashCountOptions}
              activityAgeOptions={activityAgeMenuOptions}
              activityCountOptions={activityCountOptions}
              trashedClipCount={trashedClipCount}
              isRestoringTrash={isRestoringTrash}
              isAltPressed={isAltPressed}
              canRestoreTrash={Boolean(onRestoreAllTrashedClips)}
              onUpdateSettings={onUpdateSettings}
              onRestoreTrash={() => void restoreAllTrashedClips()}
              onClearHistory={onClearHistory}
            />

            <SettingsGeneralResetFooter settings={settings} locales={locales} onUpdateSettings={onUpdateSettings} onResetColumnWidths={onResetColumnWidths} />

          </div>
  );
}
