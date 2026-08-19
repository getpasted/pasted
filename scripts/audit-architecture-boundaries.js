import assert from 'node:assert/strict';
import fs from 'node:fs';

const read = (path) => fs.readFileSync(path, 'utf8');
const commands = read('src-tauri/src/commands.rs');
const liveApp = read('src-tauri/src/live_app.rs');
const clipboardActions = read('src-tauri/src/clipboard_actions.rs');
const queueActions = read('src-tauri/src/queue_actions.rs');
const platformCapabilities = read('src-tauri/src/platform_capabilities.rs');
const hotkeys = read('src-tauri/src/hotkey_manager.rs');
const settingsService = read('src-tauri/src/settings_service.rs');
const settingsApi = read('src/api/settings.ts');
const transformsApi = read('src/api/transforms.ts');
const localizationRuntime = read('src/localization/runtime.ts');

assert.doesNotMatch(liveApp, /crate::commands::/,
  'The live-app adapter must not call the GUI command adapter');

for (const sharedCall of [
  'clipboard_actions::copy_clip',
  'clipboard_actions::paste_clip',
  'queue_actions::paste_item',
  'queue_actions::paste_all',
]) {
  assert.match(liveApp, new RegExp(sharedCall),
    `The live-app adapter must use ${sharedCall}`);
}

assert.match(commands, /clipboard_actions::copy_clip/,
  'GUI copy must use the shared clipboard workflow');
assert.match(commands, /clipboard_actions::paste_hud_clip/,
  'GUI paste must use the shared clipboard workflow');
assert.match(commands, /queue_actions::paste_item/,
  'GUI Queue paste must use the shared Queue workflow');
assert.match(commands, /queue_actions::paste_all/,
  'GUI Queue paste-all must use the shared Queue workflow');
assert.doesNotMatch(clipboardActions, /crate::commands::/,
  'Shared clipboard workflows must remain independent of GUI commands');
assert.doesNotMatch(queueActions, /crate::commands::/,
  'Shared Queue workflows must remain independent of GUI commands');
assert.doesNotMatch(hotkeys, /(?:crate::)?commands::/,
  'The hotkey adapter must not call the GUI command adapter');
for (const sharedCall of [
  'app_lock::lock_enabled',
  'clipboard_actions::execute_transform',
  'clipboard_actions::paste_hud_clip',
  'hud_window::toggle',
  'keyboard_shortcuts::parse_for_current_layout',
  'queue_actions::paste_item',
]) {
  assert.match(hotkeys, new RegExp(sharedCall),
    `The hotkey adapter must use ${sharedCall}`);
}
assert.match(platformCapabilities, /pub fn accessibility_status/,
  'Platform readiness must be exposed independently of GUI commands');
assert.match(settingsService, /Result<SettingsUpdateOutcome, ApplicationError>/,
  'Shared Settings failures must expose stable structured application errors');
assert.match(settingsApi, /saveMany:[\s\S]*save_app_settings/,
  'Frontend Settings persistence must be centralized in one capability client');
assert.match(transformsApi, /listManual:[\s\S]*get_manual_transforms/,
  'Frontend manual Transform persistence must be centralized in one capability client');
assert.doesNotMatch(read('src/hooks/useAppSettings.ts'), /invoke\([^\n]*save_app_settings/,
  'Settings hooks must not bypass the Settings capability client');
assert.doesNotMatch(read('src/components/TransformationsView.tsx'), /invoke\([^\n]*manual_transform/,
  'Transform views must not bypass the Transform capability client');
assert.doesNotMatch(localizationRuntime, /import\.meta\.glob\([^)]*eager:\s*true/s,
  'Non-English locale catalogs must not inflate the startup bundle');

console.log('Application architecture boundary audit passed.');
