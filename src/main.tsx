import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { applyDesktopPlatform } from "./utils/platform";
import { ToastProvider } from "./components/ToastProvider";
import { CaptureFeedbackWindow } from "./components/CaptureFeedbackWindow";
import { useAppSettings } from "./hooks/useAppSettings";

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
  return <CaptureFeedbackWindow settings={appSettings} settingsHydrated={settingsHydrated} />;
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {rootView === "capture-feedback" ? (
      <CaptureFeedbackRoot />
    ) : (
      <ToastProvider>
        <App />
      </ToastProvider>
    )}
  </React.StrictMode>,
);
