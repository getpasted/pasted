import { useEffect, useLayoutEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { AppSettings } from '../types';
import { DEFAULT_SETTINGS, parseSavedSettings, readCachedTheme } from '../appSettingsModel';
import { settingsApi } from '../api/settings';
import { setConfiguredLanguage } from '../localization/runtime';
import { APP_EVENTS, type AppSettingChangedEvent } from '../utils/appEvents';
import { applyAppTheme } from '../utils/appTheme';

export function useAuxiliaryAppSettings() {
  const [appSettings, setAppSettings] = useState<AppSettings>(() => ({
    ...DEFAULT_SETTINGS,
    themeMode: readCachedTheme(),
  }));
  const [settingsHydrated, setSettingsHydrated] = useState(false);
  const [presentationReady, setPresentationReady] = useState(false);

  useEffect(() => {
    let cancelled = false;
    settingsApi.load()
      .then((saved) => {
        if (!cancelled && saved) setAppSettings(parseSavedSettings(saved));
      })
      .catch(console.error)
      .finally(() => {
        if (!cancelled) setSettingsHydrated(true);
      });
    return () => { cancelled = true; };
  }, []);

  useEffect(() => {
    if (!(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) return undefined;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<AppSettingChangedEvent>(APP_EVENTS.appSettingChanged, ({ payload }) => {
      if (!payload || disposed || !(payload.key in DEFAULT_SETTINGS)) return;
      const parsed = parseSavedSettings({ [payload.key]: payload.value });
      const key = payload.key as keyof AppSettings;
      setAppSettings((current) => Object.is(current[key], parsed[key])
        ? current
        : { ...current, [key]: parsed[key] });
    }).then((dispose) => {
      if (disposed) dispose();
      else unlisten = dispose;
    }).catch(console.error);
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useLayoutEffect(() => {
    const applyPresentation = () => applyAppTheme(appSettings.themeMode);
    applyPresentation();
    document.documentElement.style.fontSize = `${appSettings.textSize}px`;
    setConfiguredLanguage(appSettings.language);
    setPresentationReady(true);
    const mediaQuery = window.matchMedia('(prefers-color-scheme: light)');
    mediaQuery.addEventListener('change', applyPresentation);
    return () => mediaQuery.removeEventListener('change', applyPresentation);
  }, [appSettings.language, appSettings.textSize, appSettings.themeMode]);

  return { appSettings, settingsHydrated, presentationReady };
}
