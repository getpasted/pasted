import type { AppSettings } from './types';
import { isConfiguredLanguage } from './localization/runtime';
import { clampAppZoom } from './utils/appZoom';
import {
  DEFAULT_ANALYSIS_ATTEMPTS_PER_CLIP,
  DEFAULT_REVISION_HISTORY_LIMIT,
  storedRetentionNumber,
} from './appSettingsRetentionModel';

export const DEFAULT_SETTINGS: AppSettings = {
  onboardingVersion: 0,
  language: 'system',
  textSize: 16,
  enableSounds: true,
  captureFeedback: true,
  captureFeedbackIgnored: false,
  captureFeedbackPreview: false,
  captureFeedbackPosition: 'top-right',
  captureFeedbackDismissSeconds: 7,
  openAtLogin: true,
  dockMenubarIcon: 'both',
  menubarIconStyle: 'clipboard',
  maxClipSizeMb: 100,
  filePreviewMode: 'safe',
  filePreviewMaxMb: 25,
  keepClipCount: 1000,
  keepClipAgeDays: 0,
  revisionHistoryLimit: DEFAULT_REVISION_HISTORY_LIMIT,
  analysisAttemptsPerClip: DEFAULT_ANALYSIS_ATTEMPTS_PER_CLIP,
  alwaysPastePlainText: false,
  rowHeight: 'medium',
  startupView: 'last_active',
  themeMode: 'system',
  enableActivityLog: true,
  activityLogCapacity: 1000,
  activityLogAgeDays: 0,
  enableTrash: true,
  trashCapacityCount: 500,
  trashAgeDays: 0,
  enableAnalytics: true,
  enableBins: true,
  enableClipTypes: true,
  enableFileFormats: true,
  enableContentClassification: true,
  enableConcealment: true,
  enableNaming: true,
  enableNotes: true,
  enableNotifications: true,
  enableAppLock: true,
  enableOcr: true,
  enableTranscriptions: true,
  enablePinning: true,
  enableProtection: true,
  enableQueue: true,
  enableRevisions: true,
  enableHud: true,
  enableHotkeys: true,
  enableTransformations: true,
  enableTypes: true,
  enableSources: true,
  enableSearch: true,
  enableCli: true,
  enableHelp: true,
  hudHotkey: 'Alt+Shift+V',
  seqToggleHotkey: 'Alt+Shift+C',
  seqPopHotkey: 'Alt+Shift+X',
  lockAppHotkey: 'Alt+Shift+L',
};

export function parseSavedSettings(saved: Record<string, string>): AppSettings {
  const next = { ...DEFAULT_SETTINGS };
  const numberValue = (key: string, fallback: number) => {
    const value = Number(saved[key]);
    return Number.isFinite(value) ? value : fallback;
  };

  if (saved.onboardingVersion) next.onboardingVersion = Math.max(0, numberValue('onboardingVersion', 0));
  if (saved.language && isConfiguredLanguage(saved.language)) next.language = saved.language;
  if (saved.textSize) next.textSize = clampAppZoom(numberValue('textSize', next.textSize));
  if (saved.enableSounds !== undefined) next.enableSounds = saved.enableSounds === 'true';
  if (saved.captureFeedback !== undefined) next.captureFeedback = saved.captureFeedback === 'true';
  if (saved.captureFeedbackIgnored !== undefined) next.captureFeedbackIgnored = saved.captureFeedbackIgnored === 'true';
  if (saved.captureFeedbackPreview !== undefined) next.captureFeedbackPreview = saved.captureFeedbackPreview === 'true';
  if (['top-left', 'top-right', 'bottom-left', 'bottom-right'].includes(saved.captureFeedbackPosition)) {
    next.captureFeedbackPosition = saved.captureFeedbackPosition as AppSettings['captureFeedbackPosition'];
  }
  if (saved.captureFeedbackDismissSeconds !== undefined) {
    const seconds = numberValue('captureFeedbackDismissSeconds', next.captureFeedbackDismissSeconds);
    next.captureFeedbackDismissSeconds = [0, 3, 5, 7, 10, 15, 30].includes(seconds) ? seconds : 7;
  }
  if (saved.openAtLogin !== undefined) next.openAtLogin = saved.openAtLogin === 'true';
  if (['auto_hide', 'both', 'menubar_only'].includes(saved.dockMenubarIcon)) next.dockMenubarIcon = saved.dockMenubarIcon as AppSettings['dockMenubarIcon'];
  if (['clipboard', 'copycat'].includes(saved.menubarIconStyle)) next.menubarIconStyle = saved.menubarIconStyle as AppSettings['menubarIconStyle'];
  if (saved.maxClipSizeMb) next.maxClipSizeMb = numberValue('maxClipSizeMb', next.maxClipSizeMb);
  if (['off', 'safe', 'all'].includes(saved.filePreviewMode)) next.filePreviewMode = saved.filePreviewMode as AppSettings['filePreviewMode'];
  if (saved.filePreviewMaxMb) next.filePreviewMaxMb = Math.max(1, Math.min(64, numberValue('filePreviewMaxMb', next.filePreviewMaxMb)));
  if (saved.keepClipCount !== undefined) next.keepClipCount = Math.max(0, numberValue('keepClipCount', next.keepClipCount));
  if (saved.keepClipAgeDays !== undefined) next.keepClipAgeDays = Math.max(0, numberValue('keepClipAgeDays', next.keepClipAgeDays));
  if (saved.revisionHistoryLimit !== undefined) next.revisionHistoryLimit = storedRetentionNumber(saved, 'revisionHistoryLimit', next.revisionHistoryLimit);
  if (saved.analysisAttemptsPerClip !== undefined) next.analysisAttemptsPerClip = storedRetentionNumber(saved, 'analysisAttemptsPerClip', next.analysisAttemptsPerClip);
  if (saved.alwaysPastePlainText !== undefined) next.alwaysPastePlainText = saved.alwaysPastePlainText === 'true';
  if (['small', 'medium', 'large'].includes(saved.rowHeight)) next.rowHeight = saved.rowHeight as AppSettings['rowHeight'];
  if (['last_active', 'clip_history'].includes(saved.startupView)) next.startupView = saved.startupView as AppSettings['startupView'];
  if (['system', 'cool', 'dark', 'warm', '2894', 'sauced', 'vampire', 'flux', '808'].includes(saved.themeMode)) next.themeMode = saved.themeMode as AppSettings['themeMode'];
  if (saved.enableActivityLog !== undefined) next.enableActivityLog = saved.enableActivityLog === 'true';
  if (saved.activityLogCapacity !== undefined) next.activityLogCapacity = Math.max(0, numberValue('activityLogCapacity', next.activityLogCapacity ?? 1000));
  if (saved.activityLogAgeDays !== undefined) next.activityLogAgeDays = Math.max(0, numberValue('activityLogAgeDays', next.activityLogAgeDays));
  if (saved.enableTrash !== undefined) next.enableTrash = saved.enableTrash === 'true';
  if (saved.trashCapacityCount !== undefined) next.trashCapacityCount = Math.max(0, numberValue('trashCapacityCount', next.trashCapacityCount ?? 500));
  if (saved.trashAgeDays !== undefined) next.trashAgeDays = Math.max(0, numberValue('trashAgeDays', next.trashAgeDays));
  for (const key of [
    'enableAnalytics', 'enableBins', 'enableClipTypes', 'enableFileFormats',
    'enableContentClassification', 'enableConcealment', 'enableNaming', 'enableNotes',
    'enableNotifications', 'enableAppLock', 'enableOcr', 'enableTranscriptions',
    'enablePinning', 'enableProtection', 'enableQueue', 'enableRevisions', 'enableHud',
    'enableHotkeys', 'enableTransformations', 'enableTypes', 'enableSources',
    'enableSearch', 'enableCli', 'enableHelp',
  ] as const) {
    if (saved[key] !== undefined) next[key] = saved[key] === 'true';
  }

  const hotkeyKeys = [
    'hudHotkey', 'seqToggleHotkey', 'seqPopHotkey', 'copyLastPipelineHotkey',
    'pasteLastPipelineHotkey', 'openTransformationsHotkey', 'openMainWindowHotkey',
    'lockAppHotkey',
    ...Array.from({ length: 9 }, (_, index) => `pasteClip${index + 1}Hotkey`),
  ];
  for (const key of hotkeyKeys) {
    if (saved[key] !== undefined) Object.assign(next, { [key]: saved[key] });
  }
  return next;
}

export function readCachedTheme(): AppSettings['themeMode'] {
  try {
    const cached = localStorage.getItem('pasted_cache_theme');
    return ['system', 'cool', 'dark', 'warm', '2894', 'sauced', 'vampire', 'flux', '808'].includes(cached ?? '')
      ? cached as AppSettings['themeMode']
      : DEFAULT_SETTINGS.themeMode;
  } catch {
    return DEFAULT_SETTINGS.themeMode;
  }
}
