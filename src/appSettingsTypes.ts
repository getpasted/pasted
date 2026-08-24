import type { RetentionSettings } from './appSettingsTypes/retention';

export interface AppSettings extends RetentionSettings {
  onboardingVersion: number;
  language: string;
  textSize: number;
  enableSounds: boolean;
  captureFeedback: boolean;
  captureFeedbackIgnored: boolean;
  captureFeedbackPreview: boolean;
  captureFeedbackPosition: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';
  captureFeedbackDismissSeconds: number;
  openAtLogin: boolean;
  dockMenubarIcon: 'auto_hide' | 'both' | 'menubar_only';
  menubarIconStyle: 'clipboard' | 'copycat';
  maxClipSizeMb: number;
  filePreviewMode: 'off' | 'safe' | 'all';
  filePreviewMaxMb: number;
  alwaysPastePlainText: boolean;
  rowHeight: 'small' | 'medium' | 'large';
  startupView: 'last_active' | 'clip_history';
  themeMode: 'system' | 'dark' | 'cool' | 'warm' | '2894' | 'sauced' | 'vampire' | 'flux' | '808';
  enableActivityLog: boolean;
  enableTrash: boolean;
  enableAnalytics: boolean;
  enableBins: boolean;
  enableClipTypes: boolean;
  enableFileFormats: boolean;
  enableContentClassification: boolean;
  enableConcealment: boolean;
  enableNaming: boolean;
  enableNotes: boolean;
  enableNotifications: boolean;
  enableAppLock: boolean;
  enableOcr: boolean;
  enableTranscriptions: boolean;
  enablePinning: boolean;
  enableProtection: boolean;
  enableQueue: boolean;
  enableRevisions: boolean;
  enableHud: boolean;
  enableHotkeys: boolean;
  enableTransformations: boolean;
  enableTypes: boolean;
  enableSources: boolean;
  enableSearch: boolean;
  enableCli: boolean;
  enableHelp: boolean;
  hudHotkey?: string;
  seqToggleHotkey?: string;
  seqPopHotkey?: string;
  copyLastPipelineHotkey?: string;
  pasteLastPipelineHotkey?: string;
  openTransformationsHotkey?: string;
  openMainWindowHotkey?: string;
  lockAppHotkey?: string;
  pasteClip1Hotkey?: string;
  pasteClip2Hotkey?: string;
  pasteClip3Hotkey?: string;
  pasteClip4Hotkey?: string;
  pasteClip5Hotkey?: string;
  pasteClip6Hotkey?: string;
  pasteClip7Hotkey?: string;
  pasteClip8Hotkey?: string;
  pasteClip9Hotkey?: string;
}

export interface OcrBackfillStatus {
  totalImages: number;
  eligibleCount: number;
  queuedCount: number;
  runningCount: number;
  completedCount: number;
  noTextCount: number;
  failedCount: number;
}

export interface BlacklistApp {
  id: string;
  name: string;
  icon: string;
  ignoreText: boolean;
  ignoreImages: boolean;
  ignoreFiles: boolean;
  ignoreHotkeys: boolean;
}

export interface SequentialStatus {
  is_active: boolean;
  queue: string[];
  item_ids: number[];
  current_index: number;
  total_count: number;
}

export interface QueuePasteTarget {
  name: string;
  automaticPasteAvailable: boolean;
  unavailableReason: string | null;
}
