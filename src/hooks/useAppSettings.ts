import { useCallback, useEffect, useRef, useState } from 'react';
import { disable, enable } from '@tauri-apps/plugin-autostart';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { AppSettings, BlacklistApp } from '../types';
import { DEFAULT_SETTINGS, parseSavedSettings, readCachedTheme } from '../appSettingsModel';
import { safeInvoke as invoke } from '../utils/tauri';
import { FEATURE_SETTING_KEYS } from '../utils/features';
import { setConfiguredLanguage } from '../localization/runtime';
import { APP_EVENTS, type AppSettingChangedEvent } from '../utils/appEvents';
import { settingsApi } from '../api/settings';
import { applyAppTheme } from '../utils/appTheme';

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

function readCachedBlacklist() {
  try {
    const parsed = JSON.parse(localStorage.getItem('pasted_cache_blacklist_apps') ?? 'null');
    return normalizeBlacklistApps(parsed);
  } catch {
    return DEFAULT_BLACKLIST_APPS;
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
    settingsApi.load()
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

    void listen<AppSettingChangedEvent>(APP_EVENTS.appSettingChanged, ({ payload }) => {
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
      const resolvedTheme = applyAppTheme(appSettings.themeMode);
      const root = document.documentElement;
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
    settingsApi.save('blacklistApps', JSON.stringify(blacklistApps)).catch(console.error);
  }, [blacklistApps, settingsHydrated]);

  useEffect(() => () => {
    Object.values(saveTimersRef.current).forEach(clearTimeout);
    for (const [key, value] of Object.entries(pendingSettingsRef.current)) {
      settingsApi.save(key, value).catch(console.error);
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
      settingsApi.saveMany(values).catch(console.error);
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
        settingsApi.save(key, pendingSettingsRef.current[key]).catch(console.error);
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
