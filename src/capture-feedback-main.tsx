import React from 'react';
import ReactDOM from 'react-dom/client';

import './App.css';
import { CaptureFeedbackWindow } from './components/CaptureFeedbackWindow';
import { LocalizationProvider, useLocalization } from './localization/LocalizationProvider';
import { useAppLock } from './hooks/useAppLock';
import { useAuxiliaryAppSettings } from './hooks/useAuxiliaryAppSettings';
import { useAuxiliaryWindowReady } from './hooks/useAuxiliaryWindowReady';
import { applyDesktopPlatform } from './utils/platform';

applyDesktopPlatform();

function CaptureFeedbackRoot() {
  const { appSettings, settingsHydrated, presentationReady } = useAuxiliaryAppSettings();
  const appLock = useAppLock({ animateUnlock: false });
  const localization = useLocalization();
  const ready = settingsHydrated
    && presentationReady
    && appLock.hydrated
    && !appLock.status.locked
    && localization.catalogReady
    && localization.configuredLanguage === appSettings.language;
  useAuxiliaryWindowReady(ready);
  if (!ready) return null;
  return <CaptureFeedbackWindow settings={appSettings} settingsHydrated />;
}

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <LocalizationProvider><CaptureFeedbackRoot /></LocalizationProvider>
  </React.StrictMode>,
);
