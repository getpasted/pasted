import type { AppSettings } from './types';
import { settingDefault } from './settingsContract.ts';

export const DEFAULT_GENERAL_SETTINGS = {
  language: settingDefault('language'), themeMode: settingDefault('themeMode'),
  textSize: settingDefault('textSize'), rowHeight: settingDefault('rowHeight'),
  startupView: settingDefault('startupView'), enableSounds: settingDefault('enableSounds'),
  openAtLogin: settingDefault('openAtLogin'), dockMenubarIcon: settingDefault('dockMenubarIcon'),
  menubarIconStyle: settingDefault('menubarIconStyle'), alwaysPastePlainText: settingDefault('alwaysPastePlainText'),
  maxClipSizeMb: settingDefault('maxClipSizeMb'), filePreviewMode: settingDefault('filePreviewMode'),
  filePreviewMaxMb: settingDefault('filePreviewMaxMb'), keepClipCount: settingDefault('keepClipCount'),
  keepClipAgeDays: settingDefault('keepClipAgeDays'), revisionHistoryLimit: settingDefault('revisionHistoryLimit'),
  analysisAttemptsPerClip: settingDefault('analysisAttemptsPerClip'), trashCapacityCount: settingDefault('trashCapacityCount'),
  trashAgeDays: settingDefault('trashAgeDays'), activityLogCapacity: settingDefault('activityLogCapacity'),
  activityLogAgeDays: settingDefault('activityLogAgeDays'),
  searchHistoryLimit: settingDefault('searchHistoryLimit'),
  searchHistoryAgeDays: settingDefault('searchHistoryAgeDays'),
} as const satisfies Partial<AppSettings>;

export function generalDefaultUpdates(): Partial<AppSettings> {
  return { ...DEFAULT_GENERAL_SETTINGS };
}
