import { useCallback, useEffect, useRef, useState } from 'react';
import { disable, enable } from '@tauri-apps/plugin-autostart';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { AppSettings, BlacklistApp } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { FEATURE_SETTING_KEYS } from '../utils/features';
import { clampAppZoom } from '../utils/appZoom';
import { isConfiguredLanguage, setConfiguredLanguage } from '../localization/runtime';

const DEFAULT_SETTINGS: AppSettings = {
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
  revisionHistoryLimit: 50,
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
  enableContentClassification: true,
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
  enableTransformations: true,
  enableTypes: true,
  enableSources: true,
  enableCli: true,
  enableHelp: true,
  hudHotkey: 'Alt+Shift+V',
  seqToggleHotkey: 'Alt+Shift+C',
  seqPopHotkey: 'Alt+Shift+X',
  lockAppHotkey: 'Alt+Shift+L',
};

const DEFAULT_BLACKLIST_APPS: BlacklistApp[] = [
  { id: '1', name: '1Password', icon: 'Lock', ignoreText: true, ignoreImages: true, ignoreFiles: true, ignoreHotkeys: false },
  { id: '2', name: 'Passwords', icon: 'Lock', ignoreText: true, ignoreImages: true, ignoreFiles: true, ignoreHotkeys: false },
  { id: '3', name: 'Keychain Access', icon: 'Key', ignoreText: true, ignoreImages: true, ignoreFiles: true, ignoreHotkeys: false },
  { id: '4', name: 'Bitwarden', icon: 'Shield', ignoreText: true, ignoreImages: true, ignoreFiles: true, ignoreHotkeys: false },
  { id: '5', name: 'Dashlane', icon: 'Shield', ignoreText: true, ignoreImages: true, ignoreFiles: true, ignoreHotkeys: false },
  { id: '6', name: 'Enpass', icon: 'Shield', ignoreText: true, ignoreImages: true, ignoreFiles: true, ignoreHotkeys: false },
  { id: '7', name: 'KeePassXC', icon: 'Shield', ignoreText: true, ignoreImages: true, ignoreFiles: true, ignoreHotkeys: false },
];

const HOTKEY_SETTING_KEYS = new Set([
  'hudHotkey',
  'seqToggleHotkey',
  'seqPopHotkey',
  'copyLastPipelineHotkey',
  'pasteLastPipelineHotkey',
  'openTransformationsHotkey',
  'openMainWindowHotkey',
  'lockAppHotkey',
  ...Array.from({ length: 9 }, (_, index) => `pasteClip${index + 1}Hotkey`),
]);

function normalizeBlacklistApps(value: unknown): BlacklistApp[] {
  if (!Array.isArray(value)) return DEFAULT_BLACKLIST_APPS;
  return value.flatMap((entry, index) => {
    if (typeof entry === 'string' && entry.trim()) {
      return [{ id: `legacy-${index}`, name: entry, icon: 'Lock', ignoreText: true, ignoreImages: true, ignoreFiles: true, ignoreHotkeys: false }];
    }
    if (!entry || typeof entry !== 'object') return [];
    const rule = entry as Partial<BlacklistApp>;
    if (typeof rule.name !== 'string' || !rule.name.trim()) return [];
    return [{
      id: typeof rule.id === 'string' ? rule.id : `legacy-${index}`,
      name: rule.name,
      icon: typeof rule.icon === 'string' ? rule.icon : 'Lock',
      ignoreText: rule.ignoreText !== false,
      ignoreImages: rule.ignoreImages !== false,
      ignoreFiles: rule.ignoreFiles !== false,
      ignoreHotkeys: rule.ignoreHotkeys === true,
    }];
  });
}

function parseSavedSettings(saved: Record<string, string>) {
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
  if (saved.revisionHistoryLimit !== undefined) next.revisionHistoryLimit = numberValue('revisionHistoryLimit', next.revisionHistoryLimit);
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
    'enableAnalytics',
    'enableBins',
    'enableContentClassification',
    'enableNotes',
    'enableNotifications',
    'enableAppLock',
    'enableOcr',
    'enableTranscriptions',
    'enablePinning',
    'enableProtection',
    'enableQueue',
    'enableRevisions',
    'enableHud',
    'enableTransformations',
    'enableTypes',
    'enableSources',
    'enableCli',
    'enableHelp',
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

function readCachedBlacklist() {
  try {
    const parsed = JSON.parse(localStorage.getItem('pasted_cache_blacklist_apps') ?? 'null');
    return normalizeBlacklistApps(parsed);
  } catch {
    return DEFAULT_BLACKLIST_APPS;
  }
}

function readCachedTheme(): AppSettings['themeMode'] {
  try {
    const cached = localStorage.getItem('pasted_cache_theme');
    return ['system', 'cool', 'dark', 'warm', '2894', 'sauced', 'vampire', 'flux', '808'].includes(cached ?? '')
      ? cached as AppSettings['themeMode']
      : DEFAULT_SETTINGS.themeMode;
  } catch {
    return DEFAULT_SETTINGS.themeMode;
  }
}

interface AppSettingChanged {
  key: string;
  value: string;
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
            if (Array.isArray(parsed)) setBlacklistApps(normalizeBlacklistApps(parsed));
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
    if (!(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) return undefined;
    let disposed = false;
    let unlistenSetting: (() => void) | undefined;

    void listen<AppSettingChanged>('app-setting-changed', ({ payload }) => {
      if (!payload || disposed) return;
      if (!(payload.key in DEFAULT_SETTINGS)) return;
      const parsed = parseSavedSettings({ [payload.key]: payload.value });
      const key = payload.key as keyof AppSettings;
      setAppSettings((current) => Object.is(current[key], parsed[key])
        ? current
        : { ...current, [key]: parsed[key] });
    }).then((unlisten) => {
      if (disposed) unlisten();
      else unlistenSetting = unlisten;
    }).catch(console.error);

    return () => {
      disposed = true;
      unlistenSetting?.();
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
      root.classList.toggle('theme-2894', resolvedTheme === '2894');
      root.classList.toggle('theme-sauced', resolvedTheme === 'sauced');
      root.classList.toggle('vampire', resolvedTheme === 'vampire');
      root.classList.toggle('flux', resolvedTheme === 'flux');
      root.classList.toggle('theme-808', resolvedTheme === '808');
      if (
        root.dataset.platform === 'linux'
        && (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
      ) {
        const nativeTheme = ['cool', 'warm', '2894', 'sauced'].includes(resolvedTheme) ? 'light' : 'dark';
        void Promise.all([
          getCurrentWindow().setTheme(nativeTheme),
          invoke('set_linux_native_menu_theme', { dark: nativeTheme === 'dark' }),
        ]).catch(console.error);
      }
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
    setConfiguredLanguage(appSettings.language);
  }, [appSettings.language]);

  useEffect(() => {
    if (!settingsHydrated || !(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) return;
    (appSettings.openAtLogin ? enable() : disable()).catch(console.error);
  }, [appSettings.openAtLogin, settingsHydrated]);

  useEffect(() => {
    if (settingsHydrated) {
      invoke('enforce_clip_retention', {
        keepCount: appSettings.keepClipCount,
        keepAgeDays: appSettings.keepClipAgeDays,
      }).catch(console.error);
    }
  }, [appSettings.keepClipAgeDays, appSettings.keepClipCount, settingsHydrated]);

  useEffect(() => {
    if (settingsHydrated && appSettings.enableRevisions) invoke('enforce_revision_retention', { keepCount: appSettings.revisionHistoryLimit }).catch(console.error);
  }, [appSettings.enableRevisions, appSettings.revisionHistoryLimit, settingsHydrated]);

  useEffect(() => {
    if (settingsHydrated && appSettings.enableTrash) {
      invoke('enforce_trash_retention', {
        keepCount: appSettings.trashCapacityCount,
        keepAgeDays: appSettings.trashAgeDays,
      }).catch(console.error);
    }
  }, [appSettings.enableTrash, appSettings.trashAgeDays, appSettings.trashCapacityCount, settingsHydrated]);

  useEffect(() => {
    if (settingsHydrated && appSettings.enableActivityLog) {
      invoke('enforce_activity_retention', {
        keepCount: appSettings.activityLogCapacity,
        keepAgeDays: appSettings.activityLogAgeDays,
      }).catch(console.error);
    }
  }, [appSettings.activityLogAgeDays, appSettings.activityLogCapacity, appSettings.enableActivityLog, settingsHydrated]);

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
    if (updates.language) setConfiguredLanguage(updates.language);
    const entries = Object.entries(updates);
    if (entries.length > 1 && entries.every(([key]) => FEATURE_SETTING_KEYS.includes(key as typeof FEATURE_SETTING_KEYS[number]))) {
      const values = Object.fromEntries(entries.map(([key, value]) => [key, String(value)]));
      for (const [key] of entries) {
        locallyChangedKeysRef.current.add(key);
        if (saveTimersRef.current[key]) clearTimeout(saveTimersRef.current[key]);
        delete saveTimersRef.current[key];
        delete pendingSettingsRef.current[key];
      }
      invoke('save_app_settings', { values }).catch(console.error);
      return;
    }
    for (const [key, value] of entries) {
      locallyChangedKeysRef.current.add(key);
      // Hotkeys are persisted synchronously by their registration commands so
      // SQLite and the native registry change as one operation. Scheduling a
      // second generic save here can leave those two sources out of sync.
      if (HOTKEY_SETTING_KEYS.has(key)) {
        if (saveTimersRef.current[key]) clearTimeout(saveTimersRef.current[key]);
        delete saveTimersRef.current[key];
        delete pendingSettingsRef.current[key];
        continue;
      }
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
      ignoreFiles: true,
      ignoreHotkeys: false,
    }]);
  }, []);

  const removeBlacklistApp = useCallback((id: string) => {
    blacklistChangedRef.current = true;
    setBlacklistApps((current) => current.filter((app) => app.id !== id));
  }, []);

  const toggleBlacklistRule = useCallback((id: string, rule: 'ignoreText' | 'ignoreImages' | 'ignoreFiles' | 'ignoreHotkeys') => {
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
