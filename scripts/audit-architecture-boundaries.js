import assert from 'node:assert/strict';
import fs from 'node:fs';

const read = (path) => fs.readFileSync(path, 'utf8');
const lineCount = (path) => read(path).trimEnd().split(/\r?\n/).length;
const readSourceTree = (directory) => fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
  const path = `${directory}/${entry.name}`;
  if (entry.isDirectory()) return readSourceTree(path);
  return /\.(?:ts|tsx)$/.test(entry.name) ? [{ path, source: read(path) }] : [];
});
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
const cliRoot = read('src-tauri/src/bin/pasted_cli.rs');
const appRoot = read('src/App.tsx');
const appNavigation = read('src/hooks/useAppNavigation.ts');
const appShell = read('src/hooks/useAppShell.ts');
const appMenuActions = read('src/hooks/useAppMenuActions.ts');
const clipSelectionController = read('src/hooks/useClipSelectionController.ts');
const clipListViewport = read('src/hooks/useClipListViewport.ts');

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
assert.match(localizationRuntime, /catalogReady:\s*Boolean\(catalogs\[locale\]\)/,
  'Lazy locale catalogs must expose explicit readiness');
assert.match(appShell, /!catalogReady \|\| !settingsHydrated \|\| !initialDataLoaded/,
  'Application readiness must wait for the selected locale catalog');
for (const hook of [
  'useAppShell',
  'useAppNavigation',
  'useAppMenuActions',
  'useClipSelectionController',
  'useClipListViewport',
]) {
  assert.match(appRoot, new RegExp(`${hook}\\(`), `The application root must delegate to ${hook}`);
}
assert.doesNotMatch(appRoot, /APP_EVENTS\.|writeAppUiState\(|consumePendingBackupClientState\(/,
  'The application root must not reclaim shell, navigation, or native menu infrastructure');
assert.match(appNavigation, /writeAppUiState\(/,
  'Navigation must own persisted route and sidebar state');
assert.match(appMenuActions, /APP_EVENTS\.appMenuAction/,
  'Native menu dispatch must remain in its focused adapter');

const sizeRatchets = new Map([
  ['src-tauri/src/db.rs', 20_111],
  ['src-tauri/src/commands.rs', 5_218],
  ['src-tauri/src/bin/pasted_cli.rs', 320],
  ['src/App.tsx', 1_245],
  ['src/hooks/useAppNavigation.ts', 175],
  ['src/hooks/useAppShell.ts', 130],
  ['src/hooks/useAppMenuActions.ts', 120],
  ['src/hooks/useClipSelectionController.ts', 230],
  ['src/hooks/useClipListViewport.ts', 195],
  ['src/utils/tauri.ts', 1_569],
]);
for (const [path, maximum] of sizeRatchets) {
  assert.ok(lineCount(path) <= maximum,
    `${path} grew beyond its ${maximum}-line architecture ratchet; extract a capability instead`);
}

const centralizedCommands = [
  'get_activity_logs', 'clear_activity_logs', 'export_activity_json', 'export_activity_csv',
  'get_analytics_summary', 'export_backup_file', 'export_full_backup_file',
  'restore_full_backup_file', 'choose_import_file', 'import_inspected_file',
  'consume_pending_full_restore_client_state', 'get_clips', 'get_trashed_clips',
  'get_clip_collection_summary', 'get_bins',
];
const presentationSource = readSourceTree('src')
  .filter(({ path }) => !path.startsWith('src/api/')
    && !path.startsWith('src/mocks/')
    && path !== 'src/utils/tauri.ts')
  .map(({ source }) => source)
  .join('\n');
for (const command of centralizedCommands) {
  assert.doesNotMatch(presentationSource, new RegExp(`invoke(?:<[^;\\n]+?>)?\\(['"]${command}['"]`),
    `${command} must be reached through its domain capability client`);
}

for (const handler of ['activity', 'analytics', 'analysis', 'backup', 'bins', 'clips']) {
  assert.ok(fs.existsSync(`src/mocks/browser/${handler}.ts`),
    `${handler} browser behavior must remain in a domain handler`);
}

for (const domain of ['clip_protection', 'retention', 'settings']) {
  assert.ok(fs.existsSync(`src-tauri/src/db/${domain}.rs`),
    `${domain} persistence must remain outside the database integration root`);
}
for (const adapter of ['activity', 'retention']) {
  assert.ok(fs.existsSync(`src-tauri/src/commands/${adapter}.rs`),
    `${adapter} GUI commands must remain outside the command integration root`);
}
const cliAdapters = [
  'activity', 'analyzer', 'app_lock', 'bins', 'classifiers', 'clips', 'connections',
  'extractors', 'history', 'inspectors', 'live_app', 'maintenance', 'operations',
  'portability', 'registry', 'retention', 'settings', 'storage', 'suggestions', 'transforms',
];
for (const adapter of cliAdapters) {
  assert.ok(fs.existsSync(`src-tauri/src/bin/pasted_cli/commands/${adapter}.rs`),
    `${adapter} CLI commands must remain outside the CLI integration root`);
  assert.ok(lineCount(`src-tauri/src/bin/pasted_cli/commands/${adapter}.rs`) <= 400,
    `${adapter} CLI adapter grew beyond 400 lines; split the capability again`);
}
for (const support of ['app_lock_support', 'common', 'extractor_support', 'retention_support', 'transform_support']) {
  assert.ok(fs.existsSync(`src-tauri/src/bin/pasted_cli/commands/${support}.rs`),
    `${support} must keep shared CLI parsing and presentation outside the integration root`);
  assert.ok(lineCount(`src-tauri/src/bin/pasted_cli/commands/${support}.rs`) <= 300,
    `${support} grew beyond 300 lines; separate its helper responsibilities`);
}
for (const dispatch of cliAdapters) {
  assert.match(cliRoot, new RegExp(`cli_commands::${dispatch}::`),
    `The CLI integration root must delegate ${dispatch} behavior`);
}
assert.doesNotMatch(commands, /pub fn (?:save_setting|configure_clip_retention)/,
  'Database domain persistence must not move into the GUI command adapter');

console.log('Application architecture boundary audit passed.');
