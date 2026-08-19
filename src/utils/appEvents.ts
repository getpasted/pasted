export const APP_EVENTS = {
  appLockChanged: 'app-lock-changed',
  appMenuAction: 'app-menu-action',
  appSettingChanged: 'app-setting-changed',
  blacklistClipIgnored: 'blacklist-clip-ignored',
  clipboardPauseChanged: 'clipboard-pause-changed',
  clipAdded: 'clip-added',
  clipLibraryChanged: 'clip-library-changed',
  navigateBin: 'navigate-bin',
  navigateTab: 'navigate-tab',
  sequentialUpdated: 'sequential-updated',
} as const;

export interface AppSettingChangedEvent {
  key: string;
  value: string;
}

export interface ClipboardPauseChangedEvent {
  isPaused: boolean;
  autoPausedBy: string | null;
}

export interface ClipLibraryChangedEvent {
  clipIds: number[];
}
