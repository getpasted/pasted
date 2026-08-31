import assert from 'node:assert/strict';
import fs from 'node:fs';

const read = (path) => fs.readFileSync(path, 'utf8');
const contract = JSON.parse(read('shared/settings-contract.json'));
const pages = new Map(contract.pages.map((page) => [page.id, page]));
const settings = new Map(contract.settings.map((setting) => [setting.key, setting]));

assert.equal(contract.version, 1, 'The shared settings contract must have a stable version');
assert.equal(contract.factoryReset, 'delete_all', 'Factory reset must delete the complete settings table');
assert.equal(pages.size, contract.pages.length, 'Settings page identifiers must be unique');
assert.equal(settings.size, contract.settings.length, 'Setting keys must be unique');

for (const setting of contract.settings) {
  assert.ok(pages.has(setting.owner), `${setting.key} must name a registered owner`);
  assert.ok(['default', 'preserve', 'factory_only'].includes(setting.reset), `${setting.key} must declare reset behavior`);
  assert.ok(['public', 'private', 'internal'].includes(setting.visibility), `${setting.key} must declare visibility`);
  assert.ok(['direct', 'dedicated', 'internal'].includes(setting.mutation), `${setting.key} must declare mutation ownership`);
  if (setting.reset === 'default') assert.notEqual(setting.default, undefined, `${setting.key} must declare its reset default`);
}

const appSettingsBody = read('src/appSettingsTypes.ts')
  .match(/export interface AppSettings[^{]*\{([\s\S]*?)\n\}/)?.[1] ?? '';
const retentionBody = read('src/appSettingsTypes/retention.ts')
  .match(/export interface RetentionSettings\s*\{([\s\S]*?)\n\}/)?.[1] ?? '';
const interfaceKeys = [appSettingsBody, retentionBody]
  .flatMap((source) => [...source.matchAll(/^\s*([A-Za-z][A-Za-z0-9]*)\??:\s/gm)].map((match) => match[1]));
for (const key of interfaceKeys) {
  assert.ok(settings.has(key), `${key} in AppSettings must be registered in the settings contract`);
  assert.notEqual(settings.get(key).default, undefined, `${key} in AppSettings must declare a first-launch default`);
}

const featureKeys = [...read('src/utils/features.ts').matchAll(/settingKey:\s*'(enable[A-Za-z]+)'/g)]
  .map((match) => match[1]);
for (const key of featureKeys) {
  const setting = settings.get(key);
  assert.equal(setting?.owner, 'functionality', `${key} must belong to Functionality`);
  assert.equal(setting?.default, true, `${key} must default on`);
}

for (const [page, expectedCount] of Object.entries({
  general: 25,
  notifications: 5,
  hotkeys: 17,
  'app-exclusions': 3,
})) {
  assert.equal(pages.get(page)?.resetStrategy, 'settings', `${page} must use the reusable settings reset service`);
  assert.equal(contract.settings.filter((setting) => setting.owner === page && setting.reset === 'default').length,
    expectedCount, `${page} reset ownership changed without updating its contract test`);
}
for (const page of ['security', 'analysis', 'intelligence']) {
  assert.equal(pages.get(page)?.resetStrategy, 'dedicated', `${page} must keep its dedicated domain reset`);
}

const securityDefaults = Object.fromEntries(contract.settings
  .filter((setting) => setting.owner === 'security' && setting.reset === 'default')
  .map((setting) => [setting.key, setting.default]));
assert.deepEqual(securityDefaults, {
  appLockSystemAuthEnabled: false,
  appLockAppleWatchEnabled: false,
  appLockIdleMinutes: 5,
  appLockOnSleep: true,
  appLockOnRestart: true,
  appLockCaptureWhileLocked: true,
});
assert.equal(settings.get('appLockVerifier')?.visibility, 'private');
assert.equal(settings.get('appLockVerifier')?.reset, 'preserve');

const frontendDefaultSources = [
  'src/appSettingsModel.ts', 'src/generalSettingsDefaults.ts', 'src/appSettingsSectionDefaults.ts',
  'src/appSettingsCapturePolicyModel.ts', 'src/appSettingsRetentionModel.ts',
  'src/hotkeySettingsModel.ts', 'src/appExclusionModel.ts',
].map(read).join('\n');
assert.match(frontendDefaultSources, /settingDefault\(/, 'Frontend defaults must read the shared contract');
assert.match(frontendDefaultSources, /rawSettingDefault[^\n]+\('blacklistApps'\)/,
  'App Exclusion defaults must read the shared contract');
assert.match(read('src-tauri/src/settings_contract.rs'), /include_str!\("\.\.\/\.\.\/shared\/settings-contract\.json"\)/,
  'Rust must compile the same settings contract as the frontend');
assert.match(read('src-tauri/src/settings_service.rs'), /settings_contract::reset_defaults\(page\)/,
  'Direct page resets must use the shared settings contract');
const searchHistoryRetention = read('src/components/SettingsGeneralSearchHistorySection.tsx');
assert.match(searchHistoryRetention, /searchHistoryAgeDays[\s\S]*searchHistoryLimit/,
  'General Search History settings must expose both age and count retention limits');
assert.match(read('src/hooks/useAppSettings.ts'), /enforce_search_history_retention[\s\S]{0,180}keepCount:[\s\S]{0,120}keepAgeDays:/,
  'Search History retention enforcement must apply both count and age limits');
assert.doesNotMatch(read('src/components/SettingsTabs.tsx'), /modified|settings-tab-modified-dot/,
  'Applied Settings changes must not look like pending tab state');
assert.match(read('src-tauri/src/app_lock.rs'), /settings_contract::dedicated_reset_defaults\([\s\S]{0,40}"security"/,
  'Security policy reset must use contract defaults while preserving credentials');
for (const [path, pattern] of [
  ['src-tauri/src/hotkey_manager/registration.rs', /settings_contract::default_value\(key\)/],
  ['src-tauri/src/clipboard_ingestion/files.rs', /settings_contract::default_(?:value|u64)\(/],
  ['src-tauri/src/app_tray.rs', /settings_contract::default_value\("menubarIconStyle"\)/],
  ['src-tauri/src/private_browsing.rs', /settings_contract::default_(?:bool|value)\(/],
  ['src-tauri/src/app_lock.rs', /settings_contract::default_(?:bool|u64)\(/],
]) {
  assert.match(read(path), pattern, `${path} runtime fallbacks must use contract defaults`);
}
assert.match(read('src-tauri/src/db/lifecycle.rs'), /DELETE FROM settings;/,
  'Factory reset must delete every contract-owned and internal setting atomically');

console.log(`Settings contract audit passed (${settings.size} settings across ${pages.size} owners).`);
