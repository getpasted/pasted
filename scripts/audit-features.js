import assert from 'node:assert/strict';
import fs from 'node:fs';

const read = (path) => fs.readFileSync(path, 'utf8');
const frontendRegistry = read('src/utils/features.ts');
const settingsType = read('src/types.ts');
const settingsHook = read('src/hooks/useAppSettings.ts');
const nativePolicy = read('src-tauri/src/features.rs');
const nativeRoot = read('src-tauri/src/lib.rs');

const frontendKeys = [...frontendRegistry.matchAll(/settingKey:\s*'(enable[A-Za-z]+)'/g)]
  .map((match) => match[1]);
const nativeKeys = [...nativePolicy.matchAll(/=>\s*"(enable[A-Za-z]+)"/g)]
  .map((match) => match[1]);

assert.equal(frontendKeys.length, 16, 'The frontend feature registry must include every supported capability');
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
  'Feature switches belong only on Settings → Features',
);

console.log(`Feature capability audit passed for ${frontendKeys.length} shared gates.`);
