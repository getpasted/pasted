import assert from 'node:assert/strict';
import fs from 'node:fs';

const read = (path) => fs.readFileSync(path, 'utf8');
const frontendRegistry = read('src/utils/features.ts');
const settingsType = read('src/types.ts');
const settingsHook = read('src/hooks/useAppSettings.ts');
const nativePolicy = read('src-tauri/src/features.rs');
const nativeRoot = read('src-tauri/src/lib.rs');
const nativeCommands = read('src-tauri/src/commands.rs');
const settingsModal = read('src/components/SettingsModal.tsx');
const settingsFeaturesPanel = read('src/components/SettingsFeaturesPanel.tsx');
const sidebar = read('src/components/Sidebar.tsx');
const nativeMenu = read('src-tauri/src/app_menu.rs');
const captureFeedbackWindow = read('src/components/CaptureFeedbackWindow.tsx');
const clipboardMonitor = read('src-tauri/src/clipboard_monitor.rs');
const hotkeyManager = read('src-tauri/src/hotkey_manager.rs');
const clipPreview = read('src/components/ClipPreview.tsx');
const settingsHotkeys = read('src/components/SettingsHotkeysPanel.tsx');
const cli = read('src-tauri/src/bin/pasted_cli.rs');
const frontendDefinitions = frontendRegistry.match(/export const FEATURE_DEFINITIONS[\s\S]*?\n\] as const;/)?.[0] ?? '';

const frontendKeys = [...frontendRegistry.matchAll(/settingKey:\s*'(enable[A-Za-z]+)'/g)]
  .map((match) => match[1]);
const nativeKeys = [...nativePolicy.matchAll(/=>\s*"(enable[A-Za-z]+)"/g)]
  .map((match) => match[1]);

assert.equal(frontendKeys.length, 23, 'The frontend feature registry must include every supported capability');
const frontendGroups = [...frontendRegistry.matchAll(/group:\s*'([A-Za-z]+)'/g)]
  .map((match) => match[1]);
assert.equal(frontendGroups.length, frontendKeys.length, 'Every feature must belong to a Functionality group');
assert.deepEqual(
  [...new Set(frontendGroups)].sort(),
  ['app', 'discovery', 'library', 'workflow'],
  'Functionality must keep the expected feature groups',
);
assert.match(
  settingsFeaturesPanel,
  /FEATURE_GROUPS\.map\(\(group\)/,
  'Settings → Functionality must render features in their logical groups',
);
assert.match(
  settingsFeaturesPanel,
  /useLocalization\(\)/,
  'Settings → Functionality must react to language changes',
);
for (const featureId of [...frontendDefinitions.matchAll(/id:\s*'([A-Za-z]+)'/g)].map((match) => match[1])) {
  assert.match(
    settingsFeaturesPanel,
    new RegExp(`feature\\.${featureId}\\.label`),
    `${featureId} must have a localized Functionality label`,
  );
  assert.match(
    settingsFeaturesPanel,
    new RegExp(`feature\\.${featureId}\\.description`),
    `${featureId} must have a localized Functionality description`,
  );
}
const toolOrder = ['transformations', 'analytics', 'activity', 'help', 'settings'];
let previousToolIndex = -1;
for (const tab of toolOrder) {
  const index = sidebar.indexOf(`{ tab: '${tab}'`, previousToolIndex + 1);
  assert.ok(index > previousToolIndex, `Tools must keep ${tab} in the intended navigation order`);
  previousToolIndex = index;
}
assert.ok(
  nativeMenu.indexOf('item(&transforms_menu)') < nativeMenu.indexOf('text("view.analytics", t("native.tools.insights"))')
    && nativeMenu.indexOf('text("view.analytics", t("native.tools.insights"))') < nativeMenu.indexOf('text("view.activity", t("native.tools.activity"))'),
  'The native Tools menu must mirror Transformations, Insights, and Activity order',
);
assert.deepEqual(
  [...new Set(nativeKeys)].sort(),
  [...new Set(frontendKeys)].sort(),
  'Frontend and native feature setting keys must stay in sync',
);

for (const key of frontendKeys) {
  assert.match(settingsType, new RegExp(`\\b${key}\\??:\\s*boolean`), `${key} must be typed in AppSettings`);
  assert.match(settingsHook, new RegExp(`\\b${key}:\\s*true`), `${key} must default on for existing installations`);
  assert.match(settingsHook, new RegExp(`(?:['\"]${key}['\"]|saved\\.${key})`), `${key} must hydrate from persisted settings`);
}

assert.match(nativeRoot, /pub mod features;/, 'The native policy must be shared with the CLI crate');
assert.doesNotMatch(
  read('src/components/SettingsGeneralPanel.tsx'),
  /onUpdateSettings\(\{\s*enable(?:Trash|ActivityLog):/,
  'Feature switches belong only on Settings → Functionality',
);

assert.match(
  nativeCommands,
  /"app-setting-changed"/,
  'Native settings writes must notify every open window',
);
assert.match(
  settingsHook,
  /listen<AppSettingChanged>\('app-setting-changed'/,
  'Each window must synchronize settings changed elsewhere',
);
assert.match(
  settingsModal,
  /settings\.enableNotifications && activeTab === 'notifications'/,
  'The Notifications feature must own its Settings surface',
);
assert.match(sidebar, /id: 'clipTypes'[\s\S]{0,180}enabled: features\.clipTypes/,
  'Clip Types must own their sidebar collection surface');
assert.match(
  captureFeedbackWindow,
  /currentSettings\.enableNotifications/,
  'The capture feedback window must honor the shared Notifications feature gate',
);
assert.match(
  clipboardMonitor,
  /Feature::Notifications/,
  'Clipboard capture must suppress notification events at the native policy boundary',
);
assert.match(
  hotkeyManager,
  /Feature::Hotkeys[\s\S]{0,500}state:\s*"disabled"/,
  'The native hotkey boundary must unregister and report disabled state when Hotkeys is off',
);
assert.match(
  hotkeyManager,
  /PasteClipById\(clip_id\) =>[\s\S]{0,900}paste_clip_from_hotkey/,
  'Direct clip hotkeys must not depend on the HUD feature gate',
);
assert.match(hotkeyManager, /get_all_settings\(\)/,
  'Hotkey rebuilds must load settings in one database snapshot');
assert.match(hotkeyManager, /get_bin_hotkeys\(\)/,
  'Hotkey rebuilds must not load full Bin records');
assert.match(hotkeyManager, /get_pipeline_hotkeys\(\)/,
  'Hotkey rebuilds must not load full Transform plans');
assert.doesNotMatch(settingsHotkeys, /setInterval\(/,
  'Hotkey Settings status must remain event-driven instead of polling');
assert.match(
  settingsModal,
  /settings\.enableHotkeys && activeTab === 'hotkeys'/,
  'The Hotkeys feature must own its Settings surface',
);
assert.match(
  clipPreview,
  /features\.protection && features\.hotkeys/,
  'Clip hotkey assignment must honor both Protection and Hotkeys',
);
assert.match(
  cli,
  /"hotkey" => \{[\s\S]{0,160}Feature::Hotkeys/,
  'CLI hotkey mutations must honor the Hotkeys feature gate',
);
assert.match(
  settingsHook,
  /root\.dataset\.theme = resolvedTheme/,
  'Synchronized appearance settings must update semantic theme tokens',
);

console.log(`Feature capability audit passed for ${frontendKeys.length} shared gates.`);
