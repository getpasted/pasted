import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { applyDesktopPlatform } from "./utils/platform";
import { ToastProvider } from "./components/ToastProvider";

// Window chrome is native on every desktop platform, but only macOS overlays
// those controls on top of Pasted's web content. Set this synchronously before
// React mounts so the first painted frame already has the correct safe area.
applyDesktopPlatform();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ToastProvider>
      <App />
    </ToastProvider>
  </React.StrictMode>,
);
