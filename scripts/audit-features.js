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
const captureFeedbackWindow = read('src/components/CaptureFeedbackWindow.tsx');
const clipboardMonitor = read('src-tauri/src/clipboard_monitor.rs');

const frontendKeys = [...frontendRegistry.matchAll(/settingKey:\s*'(enable[A-Za-z]+)'/g)]
  .map((match) => match[1]);
const nativeKeys = [...nativePolicy.matchAll(/=>\s*"(enable[A-Za-z]+)"/g)]
  .map((match) => match[1]);

assert.equal(frontendKeys.length, 19, 'The frontend feature registry must include every supported capability');
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
  settingsHook,
  /root\.dataset\.theme = resolvedTheme/,
  'Synchronized appearance settings must update semantic theme tokens',
);

console.log(`Feature capability audit passed for ${frontendKeys.length} shared gates.`);
