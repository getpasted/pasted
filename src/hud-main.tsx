import React, { useMemo } from 'react';
import ReactDOM from 'react-dom/client';

import './App.css';
import { QuickHudWindow } from './components/QuickHudWindow';
import { LocalizationProvider, useLocalization } from './localization/LocalizationProvider';
import { useAppLock } from './hooks/useAppLock';
import { useAuxiliaryAppSettings } from './hooks/useAuxiliaryAppSettings';
import { useAuxiliaryWindowReady } from './hooks/useAuxiliaryWindowReady';
import { FeatureProvider } from './hooks/useFeatures';
import { enabledFeatureRecord } from './utils/features';
import { applyDesktopPlatform } from './utils/platform';

applyDesktopPlatform();

function HudRoot() {
  const { appSettings, settingsHydrated, presentationReady } = useAuxiliaryAppSettings();
  const appLock = useAppLock({ animateUnlock: false });
  const localization = useLocalization();
  const features = useMemo(() => enabledFeatureRecord(appSettings), [appSettings]);
  const ready = settingsHydrated
    && presentationReady
    && appLock.hydrated
    && !appLock.status.locked
    && localization.catalogReady
    && localization.configuredLanguage === appSettings.language;
  useAuxiliaryWindowReady(ready);
  if (!ready) return null;
  return <FeatureProvider features={features}><QuickHudWindow /></FeatureProvider>;
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <LocalizationProvider><HudRoot /></LocalizationProvider>
  </React.StrictMode>,
);
