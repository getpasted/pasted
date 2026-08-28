import assert from 'node:assert/strict';
import fs from 'node:fs';
import { readRustModuleTree } from './audit-source-trees.js';

const config = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json', 'utf8'));
const settingsContract = JSON.parse(fs.readFileSync('shared/settings-contract.json', 'utf8'));
const settingDefaults = new Map(settingsContract.settings.map(({ key, default: value }) => [key, value]));
const appSource = fs.readFileSync('src/App.tsx', 'utf8');
const mainRootSource = fs.readFileSync('src/main.tsx', 'utf8');
const rootSource = fs.readFileSync('src/capture-feedback-main.tsx', 'utf8');
const monitorSource = readRustModuleTree(
  'src-tauri/src/clipboard_monitor.rs',
  'src-tauri/src/clipboard_ingestion',
);
const appDataSource = fs.readFileSync('src/hooks/useAppData.ts', 'utf8');
const exclusionsNativeSource = fs.readFileSync('src-tauri/src/app_exclusions.rs', 'utf8');
const privateBrowserNativeSource = fs.readFileSync('src-tauri/src/private_browsing.rs', 'utf8');
const hotkeySource = readRustModuleTree(
  'src-tauri/src/hotkey_manager.rs',
  'src-tauri/src/hotkey_manager',
);
const pasteTargetSource = readRustModuleTree(
  'src-tauri/src/paste_target.rs',
  'src-tauri/src/paste_target',
);
const settingsSource = [
  'src/appSettingsModel.ts',
  'src/appSettingsSectionDefaults.ts',
].map((path) => fs.readFileSync(path, 'utf8')).join('\n');
const capturePolicySettingsSource = fs.readFileSync('src/appSettingsCapturePolicyModel.ts', 'utf8');
const panelSource = fs.readFileSync('src/components/SettingsNotificationsPanel.tsx', 'utf8');
const exclusionsSource = fs.readFileSync('src/components/SettingsBlacklistPanel.tsx', 'utf8');
const securitySource = fs.readFileSync('src/components/SettingsSecurityPanel.tsx', 'utf8');
const privateBrowserSource = fs.readFileSync('src/components/PrivateBrowserExclusionSection.tsx', 'utf8');
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

assert.equal(settingDefaults.get('captureFeedback'), true);
assert.equal(settingDefaults.get('captureFeedbackIgnored'), false);
assert.equal(settingDefaults.get('captureFeedbackPreview'), false);
assert.equal(settingDefaults.get('captureFeedbackPosition'), 'top-right');
assert.equal(settingDefaults.get('captureFeedbackDismissSeconds'), 7);
assert.match(settingsSource, /settingDefault\('captureFeedback'\)/,
  'Capture feedback defaults must use the shared settings contract');
assert.equal(settingDefaults.get('excludePrivateBrowserWindows'), false,
  'Private-browser capture exclusion must remain opt-in');
assert.equal(settingDefaults.get('privateBrowserUnavailablePolicy'), 'capture',
  'Inconclusive private-browser detection must continue capture by default');
assert.match(capturePolicySettingsSource, /settingDefault\('excludePrivateBrowserWindows'\)/,
  'Private-browser policy defaults must use the shared settings contract');
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
assert.match(panelSource, /<SettingsPanelResetNote/, 'Notifications must use the shared resettable Settings note well');
assert.match(exclusionsSource, /<SettingsPanelResetNote/, 'App Exclusions must use the shared resettable Settings note well');
assert.match(securitySource, /<SettingsPanelResetNote/, 'Security must use the shared resettable Settings note well');
for (const [label, source] of [
  ['Notifications', panelSource],
  ['App Exclusions', exclusionsSource],
  ['Security', securitySource],
]) {
  assert.match(source, /<SettingsResetChanges/, `${label} Reset must preview its effective changes`);
}
assert.match(securitySource, /resetPolicy/, 'Security reset must preserve credential ownership behind App Lock');
assert.match(panelNoteSource, /theme-surface theme-text-muted[^"]*rounded-xl border p-4 text-\[11px\] leading-relaxed/,
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
assert.match(monitorSource, /private_browsing::should_exclude/,
  'Clipboard capture must enforce the shared private-browser policy');
for (const browser of ['Safari', 'Google Chrome', 'Microsoft Edge', 'Firefox', 'DuckDuckGo', 'Brave Browser']) {
  assert.match(privateBrowserNativeSource, new RegExp(browser),
    `Private-browser detection must retain its ${browser} adapter`);
}
assert.match(privateBrowserNativeSource, /BrowserWindowState::Unavailable/,
  'Private-browser detection must preserve an explicit unavailable state');
assert.match(privateBrowserSource, /SettingsSwitch/,
  'The built-in private-browser exclusion must remain independently configurable');
assert.match(privateBrowserSource, /privateBrowserUnavailablePolicy/,
  'The settings UI must expose the unavailable-detection policy');
assert.match(privateBrowserSource, /platform-linux-only[^>]*>[\s\S]{0,160}nativeWaylandMayNotExposeTheFocusedBrowser/,
  'The Wayland App Exclusions note must only be visible on Linux');
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

assert.match(
  monitorSource,
  /last_processed_change_marker = clipboard_change_marker\(\)/,
  'Clipboard monitoring must baseline the startup generation instead of recapturing stale contents',
);
assert.match(
  monitorSource,
  /if updated\.is_trashed \{\s*return;\s*\}[\s\S]{0,240}serde_json::json!\(\{ "id": updated\.id \}\)/,
  'Post-capture processing must not publish trashed clips or stale active snapshots',
);
assert.match(
  appDataSource,
  /if \(incoming\.is_trashed\) return clips\.filter\(\(clip\) => clip\.id !== incoming\.id\)/,
  'Active History must defensively discard trashed clip snapshots',
);

assert.match(monitorSource, /serde_json::json!\(\{ "kind": kind, "clip_id": clip_id \}\)/);
assert.doesNotMatch(
  monitorSource.match(/fn capture_feedback_payload[\s\S]*?\n\}/)?.[0] ?? '',
  /content|path|text|image|source/,
  'Capture feedback payloads must never expose clipboard data or source metadata',
);

console.log('Capture feedback privacy and window audit passed.');
