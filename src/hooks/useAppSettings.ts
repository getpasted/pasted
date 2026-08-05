import { useCallback, useEffect, useRef, useState } from 'react';
import { disable, enable } from '@tauri-apps/plugin-autostart';
import type { AppSettings, BlacklistApp } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';

const DEFAULT_SETTINGS: AppSettings = {
  textSize: 16,
  enableSounds: true,
  openAtLogin: true,
  dockMenubarIcon: 'auto_hide',
  maxClipSizeMb: 100,
  filePreviewMode: 'safe',
  filePreviewMaxMb: 25,
  detectColors: true,
  detectLinks: true,
  detectCode: true,
  keepClipCount: 900,
  revisionHistoryLimit: 50,
  alwaysPastePlainText: false,
  rowHeight: 'medium',
  iCloudSync: true,
  themeMode: 'system',
  spotlightSync: true,
  enableActivityLog: true,
  activityLogCapacity: 1000,
  enableTrash: true,
  trashCapacityCount: 500,
  hudHotkey: 'Alt+Shift+V',
  seqToggleHotkey: 'Alt+Shift+C',
  seqPopHotkey: 'Alt+Shift+X',
};

const DEFAULT_BLACKLIST_APPS: BlacklistApp[] = [
  { id: '1', name: '1Password', icon: 'Lock', ignoreText: true, ignoreImages: true, ignoreShortcuts: false },
  { id: '2', name: 'Passwords', icon: 'Lock', ignoreText: true, ignoreImages: true, ignoreShortcuts: false },
  { id: '3', name: 'Keychain Access', icon: 'Key', ignoreText: true, ignoreImages: true, ignoreShortcuts: false },
  { id: '4', name: 'Bitwarden', icon: 'Shield', ignoreText: true, ignoreImages: true, ignoreShortcuts: false },
];

function parseSavedSettings(saved: Record<string, string>) {
  const next = { ...DEFAULT_SETTINGS };
  const numberValue = (key: string, fallback: number) => {
    const value = Number(saved[key]);
    return Number.isFinite(value) ? value : fallback;
  };

  if (saved.textSize) next.textSize = numberValue('textSize', next.textSize);
  if (saved.enableSounds !== undefined) next.enableSounds = saved.enableSounds === 'true';
  if (saved.openAtLogin !== undefined) next.openAtLogin = saved.openAtLogin === 'true';
  if (['auto_hide', 'both', 'menubar_only'].includes(saved.dockMenubarIcon)) next.dockMenubarIcon = saved.dockMenubarIcon as AppSettings['dockMenubarIcon'];
  if (saved.maxClipSizeMb) next.maxClipSizeMb = numberValue('maxClipSizeMb', next.maxClipSizeMb);
  if (['off', 'safe', 'all'].includes(saved.filePreviewMode)) next.filePreviewMode = saved.filePreviewMode as AppSettings['filePreviewMode'];
  if (saved.filePreviewMaxMb) next.filePreviewMaxMb = Math.max(1, Math.min(64, numberValue('filePreviewMaxMb', next.filePreviewMaxMb)));
  if (saved.detectColors !== undefined) next.detectColors = saved.detectColors === 'true';
  if (saved.detectLinks !== undefined) next.detectLinks = saved.detectLinks === 'true';
  if (saved.detectCode !== undefined) next.detectCode = saved.detectCode === 'true';
  if (saved.keepClipCount) next.keepClipCount = numberValue('keepClipCount', next.keepClipCount);
  if (saved.revisionHistoryLimit !== undefined) next.revisionHistoryLimit = numberValue('revisionHistoryLimit', next.revisionHistoryLimit);
  if (saved.alwaysPastePlainText !== undefined) next.alwaysPastePlainText = saved.alwaysPastePlainText === 'true';
  if (['small', 'medium', 'large'].includes(saved.rowHeight)) next.rowHeight = saved.rowHeight as AppSettings['rowHeight'];
  if (saved.iCloudSync !== undefined) next.iCloudSync = saved.iCloudSync === 'true';
  if (['system', 'cool', 'dark', 'warm', 'vampire', 'flux', '808'].includes(saved.themeMode)) next.themeMode = saved.themeMode as AppSettings['themeMode'];
  if (saved.spotlightSync !== undefined) next.spotlightSync = saved.spotlightSync === 'true';
  if (saved.enableActivityLog !== undefined) next.enableActivityLog = saved.enableActivityLog === 'true';
  if (saved.activityLogCapacity) next.activityLogCapacity = numberValue('activityLogCapacity', next.activityLogCapacity ?? 1000);
  if (saved.enableTrash !== undefined) next.enableTrash = saved.enableTrash === 'true';
  if (saved.trashCapacityCount) next.trashCapacityCount = numberValue('trashCapacityCount', next.trashCapacityCount ?? 500);

  const hotkeyKeys = [
    'hudHotkey', 'seqToggleHotkey', 'seqPopHotkey', 'copyLastPipelineHotkey',
    'pasteLastPipelineHotkey', 'openTransformationsHotkey', 'openMainWindowHotkey',
    ...Array.from({ length: 9 }, (_, index) => `pasteClip${index + 1}Hotkey`),
  ];
  for (const key of hotkeyKeys) {
    if (saved[key] !== undefined) Object.assign(next, { [key]: saved[key] });
  }
  return next;
}

function readCachedBlacklist() {
  try {
    const parsed = JSON.parse(localStorage.getItem('pasted_cache_blacklist_apps') ?? 'null');
    return Array.isArray(parsed) ? parsed as BlacklistApp[] : DEFAULT_BLACKLIST_APPS;
  } catch {
    return DEFAULT_BLACKLIST_APPS;
  }
}

function readCachedTheme(): AppSettings['themeMode'] {
  try {
    const cached = localStorage.getItem('pasted_cache_theme');
    return ['system', 'cool', 'dark', 'warm', 'vampire', 'flux', '808'].includes(cached ?? '')
      ? cached as AppSettings['themeMode']
      : DEFAULT_SETTINGS.themeMode;
  } catch {
    return DEFAULT_SETTINGS.themeMode;
  }
}

export function useAppSettings() {
  const [appSettings, setAppSettings] = useState<AppSettings>(() => ({
    ...DEFAULT_SETTINGS,
    themeMode: readCachedTheme(),
  }));
  const [blacklistApps, setBlacklistApps] = useState<BlacklistApp[]>(readCachedBlacklist);
  const [settingsHydrated, setSettingsHydrated] = useState(false);
  const saveTimersRef = useRef<Record<string, ReturnType<typeof setTimeout>>>({});
  const pendingSettingsRef = useRef<Record<string, string>>({});
  const locallyChangedKeysRef = useRef(new Set<string>());
  const blacklistChangedRef = useRef(false);

  useEffect(() => {
    let cancelled = false;
    invoke<Record<string, string>>('get_all_app_settings')
      .then((saved) => {
        if (cancelled || !saved) return;
        setAppSettings((current) => {
          const hydrated = parseSavedSettings(saved);
          for (const key of locallyChangedKeysRef.current) {
            Object.assign(hydrated, { [key]: current[key as keyof AppSettings] });
          }
          return hydrated;
        });
        if (saved.blacklistApps && !blacklistChangedRef.current) {
          try {
            const parsed = JSON.parse(saved.blacklistApps);
            if (Array.isArray(parsed)) setBlacklistApps(parsed);
          } catch (error) {
            console.error('Failed to restore blacklist settings:', error);
          }
        }
      })
      .catch(console.error)
      .finally(() => {
        if (!cancelled) setSettingsHydrated(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const applyTheme = () => {
      const configuredTheme = appSettings.themeMode || 'system';
      const resolvedTheme = configuredTheme === 'system'
        ? (window.matchMedia('(prefers-color-scheme: light)').matches ? 'cool' : 'dark')
        : configuredTheme;
      const root = document.documentElement;
      root.dataset.theme = resolvedTheme;
      root.classList.toggle('cool', resolvedTheme === 'cool');
      root.classList.toggle('dark', resolvedTheme === 'dark');
      root.classList.toggle('warm', resolvedTheme === 'warm');
      root.classList.toggle('vampire', resolvedTheme === 'vampire');
      root.classList.toggle('flux', resolvedTheme === 'flux');
      root.classList.toggle('theme-808', resolvedTheme === '808');
    };
    applyTheme();
    const mediaQuery = window.matchMedia('(prefers-color-scheme: light)');
    mediaQuery.addEventListener('change', applyTheme);
    return () => mediaQuery.removeEventListener('change', applyTheme);
  }, [appSettings.themeMode]);

  useEffect(() => {
    try {
      localStorage.setItem('pasted_cache_theme', appSettings.themeMode || 'system');
    } catch {
      // SQLite remains authoritative when browser storage is unavailable.
    }
  }, [appSettings.themeMode]);

  useEffect(() => {
    document.documentElement.style.fontSize = `${appSettings.textSize}px`;
  }, [appSettings.textSize]);

  useEffect(() => {
    if (!settingsHydrated || !(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) return;
    (appSettings.openAtLogin ? enable() : disable()).catch(console.error);
  }, [appSettings.openAtLogin, settingsHydrated]);

  useEffect(() => {
    if (settingsHydrated) invoke('enforce_clip_retention', { keepCount: appSettings.keepClipCount }).catch(console.error);
  }, [appSettings.keepClipCount, settingsHydrated]);

  useEffect(() => {
    if (settingsHydrated) invoke('enforce_revision_retention', { keepCount: appSettings.revisionHistoryLimit }).catch(console.error);
  }, [appSettings.revisionHistoryLimit, settingsHydrated]);

  useEffect(() => {
    if (settingsHydrated) invoke('set_dock_visibility', { showDock: appSettings.dockMenubarIcon === 'both' }).catch(console.error);
  }, [appSettings.dockMenubarIcon, settingsHydrated]);

  useEffect(() => {
    if (!settingsHydrated) return;
    try {
      localStorage.setItem('pasted_cache_blacklist_apps', JSON.stringify(blacklistApps));
    } catch {
      // SQLite remains the source of truth when the browser cache is unavailable.
    }
    invoke('save_app_setting', { key: 'blacklistApps', value: JSON.stringify(blacklistApps) }).catch(console.error);
  }, [blacklistApps, settingsHydrated]);

  useEffect(() => () => {
    Object.values(saveTimersRef.current).forEach(clearTimeout);
    for (const [key, value] of Object.entries(pendingSettingsRef.current)) {
      invoke('save_app_setting', { key, value }).catch(console.error);
    }
  }, []);

  const updateSettings = useCallback((updates: Partial<AppSettings>) => {
    setAppSettings((current) => ({ ...current, ...updates }));
    if (updates.themeMode) {
      try {
        localStorage.setItem('pasted_cache_theme', updates.themeMode);
      } catch {
        // SQLite remains authoritative when browser storage is unavailable.
      }
    }
    for (const [key, value] of Object.entries(updates)) {
      locallyChangedKeysRef.current.add(key);
      pendingSettingsRef.current[key] = String(value);
      if (saveTimersRef.current[key]) clearTimeout(saveTimersRef.current[key]);
      saveTimersRef.current[key] = setTimeout(() => {
        invoke('save_app_setting', { key, value: pendingSettingsRef.current[key] }).catch(console.error);
        delete saveTimersRef.current[key];
        delete pendingSettingsRef.current[key];
      }, 250);
    }
  }, []);

  const addBlacklistApp = useCallback((name: string) => {
    blacklistChangedRef.current = true;
    setBlacklistApps((current) => [...current, {
      id: `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
      name,
      icon: 'Lock',
      ignoreText: true,
      ignoreImages: true,
      ignoreShortcuts: false,
    }]);
  }, []);

  const removeBlacklistApp = useCallback((id: string) => {
    blacklistChangedRef.current = true;
    setBlacklistApps((current) => current.filter((app) => app.id !== id));
  }, []);

  const toggleBlacklistRule = useCallback((id: string, rule: 'ignoreText' | 'ignoreImages' | 'ignoreShortcuts') => {
    blacklistChangedRef.current = true;
    setBlacklistApps((current) => current.map((app) => app.id === id ? { ...app, [rule]: !app[rule] } : app));
  }, []);

  return {
    appSettings,
    blacklistApps,
    settingsHydrated,
    updateSettings,
    addBlacklistApp,
    removeBlacklistApp,
    toggleBlacklistRule,
  };
}
