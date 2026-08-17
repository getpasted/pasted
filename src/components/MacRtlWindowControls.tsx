import { getCurrentWindow } from '@tauri-apps/api/window';
import { translate } from '../localization/runtime';

function runWindowAction(action: 'close' | 'minimize' | 'toggleMaximize') {
  const window = getCurrentWindow();
  void window[action]().catch((error) => {
    console.error(`Failed to ${action} window:`, error);
  });
}

export function MacRtlWindowControls() {
  return (
    <div
      className="mac-rtl-window-controls platform-macos-only titlebar-no-drag"
      role="group"
      aria-label={translate('native.window.title')}
      onPointerDown={(event) => event.stopPropagation()}
    >
      <button
        type="button"
        className="mac-window-control is-close"
        aria-label={translate('native.file.closeWindow')}
        onClick={() => runWindowAction('close')}
      >
        <svg className="mac-window-control-glyph" viewBox="0 0 8 8" aria-hidden="true">
          <path d="M1.5 1.5 6.5 6.5M6.5 1.5 1.5 6.5" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
      </button>
      <button
        type="button"
        className="mac-window-control is-minimize"
        aria-label={translate('native.window.minimize')}
        onClick={() => runWindowAction('minimize')}
      >
        <svg className="mac-window-control-glyph" viewBox="0 0 8 8" aria-hidden="true">
          <path d="M1.25 4h5.5" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
        </svg>
      </button>
      <button
        type="button"
        className="mac-window-control is-zoom"
        aria-label={translate('native.window.maximize')}
        onClick={() => runWindowAction('toggleMaximize')}
      >
        <svg className="mac-window-control-glyph" viewBox="0 0 8 8" aria-hidden="true">
          <path d="M.75.75h5.5l-5.5 5.5V.75Zm6.5 6.5h-5.5l5.5-5.5v5.5Z" fill="currentColor" />
        </svg>
      </button>
    </div>
  );
}
