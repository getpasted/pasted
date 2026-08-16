import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { applyDesktopPlatform } from "./utils/platform";
import { ToastProvider } from "./components/ToastProvider";
import { CaptureFeedbackWindow } from "./components/CaptureFeedbackWindow";
import { useAppSettings } from "./hooks/useAppSettings";
import { ContentTypeProvider } from "./components/ContentTypeProvider";
import { useAppLock, type AppLockStatus } from "./hooks/useAppLock";
import { AppLockScreen } from "./components/AppLockScreen";
import { dismissStartupSplash } from "./utils/startupSplash";

// Window chrome is native on every desktop platform, but only macOS overlays
// those controls on top of Pasted's web content. Set this synchronously before
// React mounts so the first painted frame already has the correct safe area.
applyDesktopPlatform();

const rootView = new URLSearchParams(window.location.search).get("view");
if (rootView === "capture-feedback") {
  document.documentElement.classList.add("capture-feedback-mode");
  document.body.classList.add("capture-feedback-mode");
  document.getElementById("root")?.classList.add("capture-feedback-mode");
  document.getElementById("startup-splash")?.remove();
}

function CaptureFeedbackRoot() {
  const { appSettings, settingsHydrated } = useAppSettings();
  const appLock = useAppLock();
  if (!appLock.hydrated || appLock.status.locked) return null;
  return <CaptureFeedbackWindow settings={appSettings} settingsHydrated={settingsHydrated} />;
}

function ProtectedAppRoot() {
  const appLock = useAppLock();
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
      <div className={`h-screen w-screen ${appLock.unlockAnimationActive ? 'app-unlock-content' : ''}`}>
        <ToastProvider><ContentTypeProvider><App /></ContentTypeProvider></ToastProvider>
      </div>
    )}
  </>;
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {rootView === "capture-feedback" ? (
      <CaptureFeedbackRoot />
    ) : (
      <ProtectedAppRoot />
    )}
  </React.StrictMode>,
);
