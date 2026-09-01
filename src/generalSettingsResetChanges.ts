import type { LocaleDefinition } from './localization/runtime';
import { translate } from './localization/runtime';
import type { AppSettings } from './types';
import { appZoomPercent } from './utils/appZoom';
import { DEFAULT_GENERAL_SETTINGS } from './generalSettingsDefaults';
import { resetBooleanLabel, type SettingsResetChange } from './components/SettingsResetChanges';

type GeneralKey = keyof typeof DEFAULT_GENERAL_SETTINGS;

const labels: Record<GeneralKey, () => string> = {
  language: () => translate('settings.general.language.label'),
  themeMode: () => translate('component.settingsGeneralPanel.colorScheme'),
  windowTransparency: () => translate('component.settingsGeneralPanel.windowTransparency'),
  windowBlur: () => translate('component.settingsGeneralPanel.windowBlur'),
  textSize: () => translate('component.settingsGeneralPanel.zoom'),
  rowHeight: () => translate('component.settingsGeneralPanel.clipDensity'),
  startupView: () => translate('component.settingsGeneralPanel.startupView'),
  enableSounds: () => translate('component.settingsGeneralPanel.interactionSounds'),
  openAtLogin: () => translate('component.settingsGeneralPanel.startupBehavior'),
  dockMenubarIcon: () => translate('component.settingsGeneralPanel.dockAndMenuBarIcon'),
  menubarIconStyle: () => translate('component.settingsGeneralPanel.menuBarIcon'),
  alwaysPastePlainText: () => translate('component.settingsGeneralPanel.defaultPasteBehavior'),
  maxClipSizeMb: () => translate('component.settingsGeneralPanel.maximumClipSizeMb'),
  filePreviewMode: () => translate('component.settingsGeneralPanel.filePreviews'),
  filePreviewMaxMb: () => translate('component.settingsGeneralPanel.maximumPreviewFileSizeMb'),
  keepClipCount: () => translate('component.settingsGeneralPanel.maximumClips'),
  keepClipAgeDays: () => translate('component.settingsGeneralPanel.keepClipsFor'),
  revisionHistoryLimit: () => translate('component.settingsGeneralPanel.revisionsPerClip'),
  analysisAttemptsPerClip: () => translate('component.settingsGeneralPanel.analyzationsPerClip'),
  trashCapacityCount: () => translate('component.settingsGeneralPanel.maximumTrashedClips'),
  trashAgeDays: () => translate('component.settingsGeneralPanel.keepTrashedClipsFor'),
  activityLogCapacity: () => translate('component.settingsGeneralPanel.maximumActivityEntries'),
  activityLogAgeDays: () => translate('component.settingsGeneralPanel.keepActivityFor'),
  searchHistoryLimit: () => translate('component.settingsGeneralPanel.searchHistory'),
  searchHistoryAgeDays: () => translate('component.settingsGeneralPanel.keepSearchesFor'),
};

export function generalSettingsResetChanges(
  settings: AppSettings,
  locales: readonly LocaleDefinition[],
  customColumnWidths: boolean,
  isMac: boolean,
): SettingsResetChange[] {
  const changes = (Object.keys(DEFAULT_GENERAL_SETTINGS) as GeneralKey[]).flatMap((key) => {
    const current = settings[key];
    const next = DEFAULT_GENERAL_SETTINGS[key];
    return current === next ? [] : [{
      label: key === 'dockMenubarIcon' && !isMac
        ? translate('component.settingsGeneralPanel.systemTrayAndTaskbar')
        : labels[key](),
      before: formatValue(key, current, locales, isMac),
      after: formatValue(key, next, locales, isMac),
    }];
  });
  if (customColumnWidths) changes.push({
    label: translate('component.settingsGeneralPanel.columnWidths'),
    before: translate('common.custom'),
    after: translate('common.default'),
  });
  return changes;
}

function formatValue(key: GeneralKey, value: AppSettings[GeneralKey], locales: readonly LocaleDefinition[], isMac: boolean) {
  if (key === 'alwaysPastePlainText') return optionLabel(key, String(value), isMac);
  if (typeof value === 'boolean') return resetBooleanLabel(value);
  if (key === 'language') return value === 'system'
    ? translate('common.automatic')
    : locales.find(({ code }) => code === value)?.nativeName ?? String(value);
  if (key === 'textSize') return `${appZoomPercent(Number(value))}%`;
  if (key === 'windowTransparency') return `${value}%`;
  if (key === 'windowBlur') return `${value}px`;
  if (key.endsWith('AgeDays')) return Number(value) === 0
    ? translate('component.settingsGeneralPanel.forever')
    : translate('format.dayCount', { count: Number(value) });
  if (key === 'keepClipCount' || key === 'trashCapacityCount') {
    return Number(value) === 0 ? translate('component.settingsGeneralPanel.unlimited') : translate('format.clipCount', { count: Number(value) });
  }
  if (key === 'activityLogCapacity') return Number(value) === 0
    ? translate('component.settingsGeneralPanel.unlimited')
    : translate('format.entryCount', { count: Number(value) });
  if (key === 'revisionHistoryLimit') return Number(value) === 0
    ? translate('component.settingsGeneralPanel.unlimited')
    : translate('component.settingsGeneralPanel.valueRevisions', { value: Number(value) });
  if (key === 'analysisAttemptsPerClip') return Number(value) === 0
    ? translate('component.settingsGeneralPanel.unlimited')
    : translate('component.settingsGeneralPanel.valueAnalysisAttempts', { value: Number(value) });
  if (key === 'searchHistoryLimit') return Number(value) === 0
    ? translate('component.settingsGeneralPanel.unlimited')
    : translate('component.settingsGeneralPanel.valueSearches', { value: Number(value) });
  if (key === 'maxClipSizeMb' || key === 'filePreviewMaxMb') return translate('component.settingsResetChanges.megabytes', { count: Number(value) });
  return optionLabel(key, String(value), isMac);
}

function optionLabel(key: GeneralKey, value: string, isMac: boolean) {
  const optionKeys: Partial<Record<GeneralKey, Record<string, Parameters<typeof translate>[0]>>> = {
    themeMode: { system: 'common.system', dark: 'component.settingsGeneralPanel.dark', cool: 'component.settingsGeneralPanel.cool', warm: 'component.settingsGeneralPanel.warm', sauced: 'component.settingsGeneralPanel.sauced', vampire: 'component.settingsGeneralPanel.vampire', flux: 'component.settingsGeneralPanel.flux' },
    rowHeight: { small: 'component.settingsGeneralPanel.compact', medium: 'component.settingsGeneralPanel.standard', large: 'component.settingsGeneralPanel.spacious' },
    startupView: { last_active: 'component.settingsGeneralPanel.lastActivePage', clip_history: 'component.settingsGeneralPanel.clipHistory' },
    dockMenubarIcon: isMac
      ? { auto_hide: 'component.settingsGeneralPanel.autoHideDockIcon', both: 'component.settingsGeneralPanel.alwaysShowDockAndMenuBar', menubar_only: 'component.settingsGeneralPanel.menubarIconOnly' }
      : { auto_hide: 'component.settingsGeneralPanel.autoHideTaskbarIcon', both: 'component.settingsGeneralPanel.alwaysShowTrayAndTaskbar', menubar_only: 'component.settingsGeneralPanel.systemTrayIconOnly' },
    menubarIconStyle: { clipboard: 'component.settingsGeneralPanel.clipboard', copycat: 'component.settingsGeneralPanel.copycat' },
    alwaysPastePlainText: { false: 'component.settingsGeneralPanel.preserveFormattingDefault', true: 'component.settingsGeneralPanel.alwaysPastePlainText' },
    filePreviewMode: { off: 'component.settingsGeneralPanel.off', safe: 'component.settingsGeneralPanel.safeTypes', all: 'component.settingsGeneralPanel.allSupported' },
  };
  if (key === 'themeMode' && ['2894', '808'].includes(value)) return value;
  const translationKey = optionKeys[key]?.[value];
  return translationKey ? translate(translationKey) : value;
}
