import assert from 'node:assert/strict';
import fs from 'node:fs';

const config = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json', 'utf8'));
const appSource = fs.readFileSync('src/App.tsx', 'utf8');
const rootSource = fs.readFileSync('src/main.tsx', 'utf8');
const monitorSource = fs.readFileSync('src-tauri/src/clipboard_monitor.rs', 'utf8');
const settingsSource = fs.readFileSync('src/hooks/useAppSettings.ts', 'utf8');
const panelSource = fs.readFileSync('src/components/SettingsNotificationsPanel.tsx', 'utf8');
const overlaySource = fs.readFileSync('src/components/CaptureFeedbackWindow.tsx', 'utf8');
const tabsSource = fs.readFileSync('src/components/SettingsTabs.tsx', 'utf8');
const capabilitiesSource = fs.readFileSync('src-tauri/capabilities/default.json', 'utf8');

const feedbackWindow = config.app.windows.find(({ label }) => label === 'capture-feedback');
assert.ok(feedbackWindow, 'Capture feedback needs a dedicated window');
assert.equal(feedbackWindow.visible, false, 'Capture feedback must start hidden');
assert.equal(feedbackWindow.focus, false, 'Capture feedback must not steal focus');
assert.equal(feedbackWindow.focusable, false, 'Capture feedback must remain non-focusable');
assert.equal(feedbackWindow.decorations, false, 'Capture feedback must remain chrome-free');
assert.equal(feedbackWindow.transparent, true, 'Capture feedback must preserve its overlay surface');
assert.equal(feedbackWindow.alwaysOnTop, true, 'Capture feedback must remain visible above the current app');
assert.match(capabilitiesSource, /"capture-feedback"/, 'Capture feedback must be authorized by Tauri capabilities');
for (const permission of [
  'allow-current-monitor',
  'allow-primary-monitor',
  'allow-cursor-position',
  'allow-monitor-from-point',
  'allow-outer-size',
  'allow-outer-position',
  'allow-set-size',
  'allow-set-position',
  'allow-set-ignore-cursor-events',
  'allow-set-cursor-icon',
  'allow-set-focusable',
  'allow-show',
  'allow-hide',
]) {
  assert.match(
    capabilitiesSource,
    new RegExp(`core:window:${permission}`),
    `Capture feedback requires ${permission}`,
  );
}

assert.match(settingsSource, /captureFeedback:\s*true/);
assert.match(settingsSource, /captureFeedbackIgnored:\s*false/);
assert.match(settingsSource, /captureFeedbackPreview:\s*false/);
assert.match(settingsSource, /captureFeedbackPosition:\s*'top-right'/);
assert.match(settingsSource, /captureFeedbackDismissSeconds:\s*7/);
assert.match(tabsSource, /id:\s*'notifications'/);
assert.doesNotMatch(appSource, /CaptureFeedbackWindow/);
assert.match(rootSource, /rootView === "capture-feedback"/);
assert.match(rootSource, /<CaptureFeedbackRoot/);
assert.match(panelSource, /never exposes copied text, images, file names, or paths to system notifications\./);
assert.doesNotMatch(panelSource, /capture-feedback-preview/);
assert.match(overlaySource, /clipboard-capture-feedback/);
assert.match(overlaySource, /is-global-pointer-hover/);
assert.match(overlaySource, /set_overlay_cursor/);
assert.match(overlaySource, /captureFeedbackDismissSeconds/);
assert.match(overlaySource, /SWIPE_DISMISS_THRESHOLD/);
assert.match(
  overlaySource,
  /if \(item\.clip\.isPinned\) return;/,
  'Pinned capture previews must opt out of automatic dismissal',
);
assert.match(
  overlaySource,
  /if \(isPinned\) pauseAutoDismiss\(item\.id\);[\s\S]*else scheduleAutoDismiss\(item\.id\);/,
  'Pinning must pause dismissal and unpinning must restart it',
);
assert.match(
  overlaySource,
  /setSize\(new LogicalSize\(WINDOW_WIDTH, MAX_WINDOW_HEIGHT\)\)/,
  'Capture feedback must keep stable native bounds while its visible stack changes',
);
assert.match(
  overlaySource,
  /flushSync\(\(\) => setItems\(next\)\)/,
  'Capture feedback must commit stack removal before resizing its native window',
);
assert.doesNotMatch(
  overlaySource,
  /await[^;]*requestAnimationFrame/,
  'Capture feedback must never wait for an animation frame before showing its hidden WebView',
);
assert.doesNotMatch(overlaySource, /clip-added/);

assert.match(monitorSource, /serde_json::json!\(\{ "kind": kind, "clip_id": clip_id \}\)/);
assert.doesNotMatch(
  monitorSource.match(/fn capture_feedback_payload[\s\S]*?\n\}/)?.[0] ?? '',
  /content|path|text|image|source_app/,
  'Capture feedback payloads must never expose clipboard data or source metadata',
);

console.log('Capture feedback privacy and window audit passed.');
