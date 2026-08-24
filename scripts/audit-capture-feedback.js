import assert from 'node:assert/strict';
import fs from 'node:fs';
import { readRustModuleTree } from './audit-source-trees.js';

const config = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json', 'utf8'));
const appSource = fs.readFileSync('src/App.tsx', 'utf8');
const mainRootSource = fs.readFileSync('src/main.tsx', 'utf8');
const rootSource = fs.readFileSync('src/capture-feedback-main.tsx', 'utf8');
const monitorSource = readRustModuleTree(
  'src-tauri/src/clipboard_monitor.rs',
  'src-tauri/src/clipboard_ingestion',
);
const exclusionsNativeSource = fs.readFileSync('src-tauri/src/app_exclusions.rs', 'utf8');
const hotkeySource = readRustModuleTree(
  'src-tauri/src/hotkey_manager.rs',
  'src-tauri/src/hotkey_manager',
);
const pasteTargetSource = fs.readFileSync('src-tauri/src/paste_target.rs', 'utf8');
const settingsSource = fs.readFileSync('src/appSettingsModel.ts', 'utf8');
const panelSource = fs.readFileSync('src/components/SettingsNotificationsPanel.tsx', 'utf8');
const exclusionsSource = fs.readFileSync('src/components/SettingsBlacklistPanel.tsx', 'utf8');
const panelNoteSource = fs.readFileSync('src/components/SettingsPanelNote.tsx', 'utf8');
const overlaySource = [
  'src/components/CaptureFeedbackWindow.tsx',
  'src/components/CaptureFeedbackCard.tsx',
  'src/components/captureFeedbackModel.ts',
].map((path) => fs.readFileSync(path, 'utf8')).join('\n');
const tabsSource = fs.readFileSync('src/components/SettingsTabs.tsx', 'utf8');
const capabilitiesSource = fs.readFileSync('src-tauri/capabilities/default.json', 'utf8');
const englishCatalog = JSON.parse(fs.readFileSync('src/locales/en.json', 'utf8'));

const feedbackWindow = config.app.windows.find(({ label }) => label === 'capture-feedback');
assert.ok(feedbackWindow, 'Capture feedback needs a dedicated window');
assert.equal(feedbackWindow.visible, false, 'Capture feedback must start hidden');
assert.equal(feedbackWindow.focus, false, 'Capture feedback must not steal focus');
assert.equal(feedbackWindow.focusable, false, 'Capture feedback must remain non-focusable');
assert.equal(feedbackWindow.decorations, false, 'Capture feedback must remain chrome-free');
assert.equal(feedbackWindow.transparent, true, 'Capture feedback must preserve its overlay surface');
assert.equal(feedbackWindow.alwaysOnTop, true, 'Capture feedback must remain visible above the current app');
assert.equal(feedbackWindow.url, 'capture-feedback.html', 'Capture feedback must load its dedicated entry point');
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
assert.doesNotMatch(mainRootSource, /CaptureFeedbackWindow|CaptureFeedbackRoot/);
assert.match(rootSource, /<CaptureFeedbackRoot/);
assert.match(rootSource, /useAuxiliaryWindowReady\(ready\)/,
  'Capture feedback must remain paint-hidden until its settings, lock, and locale are ready');
assert.match(rootSource, /useAuxiliaryAppSettings\(\)/,
  'Capture feedback must use read-only auxiliary settings initialization');
const notificationPrivacyKey = 'component.settingsNotificationsPanel.captureFeedbackStaysOnDeviceAndNeverExposesCopiedTextImagesFile';
assert.match(panelSource, new RegExp(`translate\\('${notificationPrivacyKey.replaceAll('.', '\\.')}\\'\\)`));
assert.match(englishCatalog[notificationPrivacyKey], /never exposes copied text, images, file names, or paths to system notifications\./);
assert.match(panelSource, /<SettingsPanelNote>/, 'Notifications must use the shared Settings note well');
assert.match(exclusionsSource, /<SettingsPanelNote>/, 'App Exclusions must use the shared Settings note well');
assert.match(panelNoteSource, /theme-surface theme-text-muted rounded-xl border p-4 text-\[11px\] leading-relaxed/,
  'Settings note wells must share one semantic surface and layout');
for (const rule of ['ignoreText', 'ignoreImages', 'ignoreFiles', 'ignoreHotkeys']) {
  assert.match(exclusionsSource, new RegExp(rule), `App Exclusions must expose the ${rule} rule`);
}
for (const kind of ['Text', 'Image', 'Files']) {
  assert.match(monitorSource, new RegExp(`ExcludedCaptureKind::${kind}`),
    `Clipboard capture must enforce the ${kind} App Exclusion rule`);
}
assert.doesNotMatch(monitorSource, /from_str::<Vec<String>>\(&blacklist_json\)/,
  'Clipboard capture must not bypass structured App Exclusion rules with the obsolete string-list parser');
assert.match(hotkeySource, /app_exclusions::should_ignore_hotkeys/,
  'Every native and portal hotkey action must honor App Exclusions before dispatch');
assert.match(exclusionsNativeSource, /explicit_empty_lists_remain_empty/,
  'Removing every App Exclusion must not silently restore defaults');
assert.match(exclusionsNativeSource, /older_object_rules_default_files_to_excluded/,
  'Saved rules from before the Files control must preserve their existing capture protection');
assert.match(pasteTargetSource, /QueryFullProcessImageNameW/,
  'Windows App Exclusions must match the focused executable rather than its changing window title');
assert.match(pasteTargetSource, /getwindowclassname/,
  'X11 App Exclusions must match the focused application class rather than its changing window title');
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
  /setSize\(new LogicalSize\([\s\S]{0,160}MAX_CAPTURE_FEEDBACK_WINDOW_HEIGHT/,
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
  /content|path|text|image|source/,
  'Capture feedback payloads must never expose clipboard data or source metadata',
);

console.log('Capture feedback privacy and window audit passed.');
