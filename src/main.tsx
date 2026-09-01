import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { applyDesktopPlatform } from "./utils/platform";
import { ToastProvider } from "./components/ToastProvider";
import { ContentTypeProvider } from "./components/ContentTypeProvider";
import { useAppLock, type AppLockStatus } from "./hooks/useAppLock";
import { AppLockScreen } from "./components/AppLockScreen";
import { dismissStartupSplash } from "./utils/startupSplash";
import { LocalizationProvider } from "./localization/LocalizationProvider";
import { getLocalizationSnapshot } from "./localization/runtime";
import { restorePendingBackupClientStateBeforeMount } from "./utils/backupClientState";

// Window chrome is native on every desktop platform, but only macOS overlays
// those controls on top of Pasted's web content. Set this synchronously before
// React mounts so the first painted frame already has the correct safe area.
applyDesktopPlatform();

const markWindowActive = () => {
  document.documentElement.removeAttribute('data-window-inactive');
};

// WebKit can defer cross-process focus delivery during app activation. Keep
// inactive writes local to the page so a queued native blur cannot arrive
// after focus and turn the traffic lights gray again.
window.addEventListener('blur', () => {
  document.documentElement.setAttribute('data-window-inactive', '');
});
window.addEventListener('focus', markWindowActive);
window.addEventListener('pointerdown', markWindowActive, { capture: true });

function ProtectedAppRoot() {
  const appLock = useAppLock({ trackIdle: true });
  const lastLockedStatus = React.useRef<AppLockStatus | null>(null);
  if (appLock.status.locked) lastLockedStatus.current = appLock.status;

  React.useEffect(() => {
    if (!appLock.hydrated || !appLock.status.locked) return undefined;
    const splash = document.getElementById("startup-splash");
    if (!splash) return undefined;
    return dismissStartupSplash(splash);
  }, [appLock.hydrated, appLock.status.locked]);
  if (!appLock.hydrated) return null;
  const showLockScreen = appLock.status.locked || appLock.unlockingSuccess;
  const showApp = !appLock.status.locked || appLock.unlockingSuccess;
  const overlayStatus = appLock.status.locked ? appLock.status : lastLockedStatus.current;
  return <>
    {showLockScreen && overlayStatus && (
      <AppLockScreen
        key="app-lock-screen"
        status={overlayStatus}
        unlocking={appLock.unlockAnimationActive}
        onUnlockWithPassphrase={appLock.unlockWithPassphrase}
        onUnlockWithSystemAuth={appLock.unlockWithSystemAuth}
        onUnlockWithAppleWatch={appLock.unlockWithAppleWatch}
      />
    )}
    {showApp && (
      <div className="h-screen w-screen">
        <ToastProvider><ContentTypeProvider><App /></ContentTypeProvider></ToastProvider>
      </div>
    )}
  </>;
}

async function mountApp() {
  try {
    // Full Restore stages its backed-up interface state in the restored database.
    // Apply it before any hook reads localStorage so startup has one stable frame.
    await restorePendingBackupClientStateBeforeMount();
  } catch (error) {
    console.error('Failed to restore backed-up interface state:', error);
  }

  const initialLocalization = getLocalizationSnapshot();
  document.documentElement.lang = initialLocalization.locale;
  document.documentElement.dir = initialLocalization.direction;

  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <LocalizationProvider>
        <ProtectedAppRoot />
      </LocalizationProvider>
    </React.StrictMode>,
  );
}

void mountApp();
