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
const appLockCommands = read('src-tauri/src/commands/app_lock.rs');
const analysisCommands = read('src-tauri/src/commands/analysis.rs');
const contentRegistryCommands = read('src-tauri/src/commands/content_registry.rs');
const extractorCommands = read('src-tauri/src/commands/extractors.rs');
const queueCommands = read('src-tauri/src/commands/queue.rs');
const storageCommands = read('src-tauri/src/commands/storage.rs');
const liveApp = read('src-tauri/src/live_app.rs');
const clipboardActions = read('src-tauri/src/clipboard_actions.rs');
const queueActions = read('src-tauri/src/queue_actions.rs');
const platformCapabilities = read('src-tauri/src/platform_capabilities.rs');
const hotkeys = read('src-tauri/src/hotkey_manager.rs');
const settingsService = read('src-tauri/src/settings_service.rs');
const settingsApi = read('src/api/settings.ts');
const transformsApi = read('src/api/transforms.ts');
const localizationRuntime = read('src/localization/runtime.ts');
const cliRoot = read('src-tauri/src/bin/pasted.rs');
const appRoot = read('src/App.tsx');
const appNavigation = read('src/hooks/useAppNavigation.ts');
const appShell = read('src/hooks/useAppShell.ts');
const appMenuActions = read('src/hooks/useAppMenuActions.ts');
const clipSelectionController = read('src/hooks/useClipSelectionController.ts');
const clipListViewport = read('src/hooks/useClipListViewport.ts');
const rememberedClipListScroll = read('src/hooks/useRememberedClipListScroll.ts');
const clipCommands = read('src-tauri/src/commands/clips.rs');
const backupCommands = read('src-tauri/src/commands/backups.rs');
const importCommands = read('src-tauri/src/commands/imports.rs');
const factoryResetCommands = read('src-tauri/src/commands/factory_reset.rs');
const extractionCommands = read('src-tauri/src/commands/extraction.rs');
const filePreviewCommands = read('src-tauri/src/commands/file_previews.rs');
const intelligenceCommands = read('src-tauri/src/commands/intelligence.rs');
const manualTransformCommands = read('src-tauri/src/commands/manual_transforms.rs');
const sourceApplicationCommands = read('src-tauri/src/commands/source_apps.rs');
const transformationCommands = read('src-tauri/src/commands/transformations.rs');
const appOverlays = read('src/hooks/useAppOverlays.ts');
const clipDragController = read('src/hooks/useClipDragController.ts');
const clipReordering = read('src/hooks/useClipReordering.ts');

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
assert.match(queueCommands, /queue_actions::paste_item/,
  'GUI Queue paste must use the shared Queue workflow');
assert.match(queueCommands, /queue_actions::paste_all/,
  'GUI Queue paste-all must use the shared Queue workflow');
assert.match(appLockCommands, /app_lock::lock_enabled/,
  'GUI App Lock commands must use the shared App Lock workflow');
assert.match(analysisCommands, /analysis_execution::analyze_(?:text|clip)/,
  'GUI Analyzer commands must use the shared Analysis execution service');
assert.match(extractorCommands, /pub async fn choose_extractor_executable[\s\S]*blocking_pick_file/,
  'Extractor executable selection must keep its native picker behind an async command');
assert.match(contentRegistryCommands, /db\.create_content_classifier/,
  'GUI Content Registry commands must delegate classifier persistence to the shared database domain');
assert.match(storageCommands, /pub async fn move_library[\s\S]*blocking_pick_folder/,
  'Library relocation must keep its native folder picker behind an async command');
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
  'useAppOverlays',
  'useClipDragController',
  'useClipReordering',
]) {
  assert.match(appRoot, new RegExp(`${hook}\\(`), `The application root must delegate to ${hook}`);
}
assert.doesNotMatch(appRoot, /APP_EVENTS\.|writeAppUiState\(|consumePendingBackupClientState\(/,
  'The application root must not reclaim shell, navigation, or native menu infrastructure');
assert.match(appNavigation, /writeAppUiState\(/,
  'Navigation must own persisted route and sidebar state');
assert.match(appMenuActions, /APP_EVENTS\.appMenuAction/,
  'Native menu dispatch must remain in its focused adapter');
assert.doesNotMatch(appRoot, /clipsApi\.setPinned|useClipBinDrag|useStableVerticalReorder/,
  'The application root must not bypass shared batch actions, drag control, or reordering systems');
assert.match(appOverlays, /closeTopmostOverlay/,
  'Overlay dismissal priority must remain centralized');
assert.match(clipDragController, /useClipBinDrag\(/,
  'Clip drag behavior must remain behind its application controller');
assert.match(clipReordering, /useStableVerticalReorder\(/,
  'Queue and Bin ordering must remain behind one shared coordinator');
assert.doesNotMatch(rememberedClipListScroll, /Array\.from\([^)]*querySelectorAll|observeCards/,
  'Scroll restoration must not scan or observe every rendered Clip on navigation');
assert.match(clipCommands, /pub fn get_clips[\s\S]*db\.get_clips_page/,
  'GUI clip retrieval must remain in the focused clip adapter');
assert.match(clipCommands, /bin_assignment::assign_clips_to_bin/,
  'GUI Bin assignment must remain delegated to the shared assignment workflow');
assert.match(filePreviewCommands, /pub async fn get_file_clip_previews[\s\S]*spawn_blocking/,
  'File preview generation must remain outside async command dispatch');
assert.match(filePreviewCommands, /read_bounded_file[\s\S]*MAX_FILE_PREVIEW_OUTPUT_BYTES/,
  'File previews must retain shared input and output bounds');
assert.doesNotMatch(commands, /pub fn get_clips|pub fn update_clip_note|pub fn batch_pin_clips/,
  'The GUI command root must not reclaim clip library operations');
assert.doesNotMatch(commands, /pub async fn get_file_clip_previews|fn collect_file_clip_previews/,
  'The GUI command root must not reclaim file preview generation');
assert.match(manualTransformCommands, /pub async fn preview_manual_transform_steps/,
  'Manual Transform preview must remain in its focused GUI adapter');
assert.match(intelligenceCommands, /pub async fn plan_transformation_intent/,
  'Intelligence-backed Transform planning must remain in its focused GUI adapter');
assert.match(transformationCommands, /pub async fn execute_transformation/,
  'Transform execution must remain in its focused GUI adapter');
assert.doesNotMatch(commands, /pub fn get_manual_transforms|pub async fn execute_transformation/,
  'The GUI command root must not reclaim transformation operations');
assert.match(backupCommands, /pub async fn restore_full_backup_file[\s\S]*spawn_blocking/,
  'Full Restore must remain in its focused asynchronous GUI adapter');
assert.match(importCommands, /pub async fn choose_import_file[\s\S]*spawn_blocking/,
  'Import preflight must remain in its focused asynchronous GUI adapter');
assert.match(factoryResetCommands, /pub fn factory_reset_app/,
  'Factory Reset must remain in its focused lifecycle adapter');
assert.doesNotMatch(commands, /pub async fn export_backup_file|pub async fn choose_import_file|pub fn factory_reset_app/,
  'The GUI command root must not reclaim portability or reset operations');
assert.match(sourceApplicationCommands, /pub async fn get_source_icons[\s\S]*spawn_blocking/,
  'Source icon resolution must remain in its focused asynchronous GUI adapter');
assert.match(sourceApplicationCommands, /pub fn get_installed_applications/,
  'Installed application discovery must remain with the source application adapter');
assert.doesNotMatch(commands, /pub async fn get_source_icons|pub fn get_installed_applications/,
  'The GUI command root must not reclaim source application discovery');
assert.match(extractionCommands, /pub fn extract_ocr_from_clip/,
  'Interactive OCR must remain in its focused extraction adapter');
assert.match(extractionCommands, /pub async fn extract_text_from_file_clip[\s\S]*spawn_blocking/,
  'File extraction must remain in its focused asynchronous adapter');
assert.match(extractionCommands, /pub fn start_ocr_backfill/,
  'OCR backfill control must remain with extraction lifecycle commands');
assert.doesNotMatch(commands, /pub fn extract_ocr_from_clip|pub async fn extract_text_from_file_clip|pub fn start_ocr_backfill/,
  'The GUI command root must not reclaim extraction lifecycle operations');

const sizeRatchets = new Map([
  ['src-tauri/src/db.rs', 20_111],
  ['src-tauri/src/commands.rs', 1_437],
  ['src-tauri/src/commands/backups.rs', 180],
  ['src-tauri/src/commands/imports.rs', 287],
  ['src-tauri/src/commands/factory_reset.rs', 39],
  ['src-tauri/src/commands/extraction.rs', 187],
  ['src-tauri/src/commands/clips.rs', 261],
  ['src-tauri/src/commands/file_previews.rs', 714],
  ['src-tauri/src/commands/intelligence.rs', 271],
  ['src-tauri/src/commands/manual_transforms.rs', 164],
  ['src-tauri/src/commands/source_apps.rs', 463],
  ['src-tauri/src/commands/transformations.rs', 244],
  ['src-tauri/src/commands/analysis.rs', 100],
  ['src-tauri/src/commands/content_registry.rs', 260],
  ['src-tauri/src/commands/extractors.rs', 180],
  ['src-tauri/src/commands/app_lock.rs', 322],
  ['src-tauri/src/commands/queue.rs', 160],
  ['src-tauri/src/commands/storage.rs', 170],
  ['src-tauri/src/bin/pasted.rs', 320],
  ['src/App.tsx', 950],
  ['src/hooks/useAppNavigation.ts', 175],
  ['src/hooks/useAppShell.ts', 130],
  ['src/hooks/useAppMenuActions.ts', 120],
  ['src/hooks/useClipSelectionController.ts', 230],
  ['src/hooks/useClipListViewport.ts', 195],
  ['src/hooks/useAppOverlays.ts', 135],
  ['src/hooks/useClipDragController.ts', 130],
  ['src/hooks/useClipReordering.ts', 100],
  ['src/components/AppDialogLayer.tsx', 210],
  ['src/utils/tauri.ts', 1_305],
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

for (const handler of [
  'activity', 'analytics', 'analysis', 'appState', 'backup', 'bins', 'clips',
  'manualTransforms', 'queue',
]) {
  assert.ok(fs.existsSync(`src/mocks/browser/${handler}.ts`),
    `${handler} browser behavior must remain in a domain handler`);
}
for (const handler of ['appState', 'manualTransforms', 'queue']) {
  assert.ok(lineCount(`src/mocks/browser/${handler}.ts`) <= 160,
    `${handler} browser handler grew beyond 160 lines; split the capability again`);
}

for (const domain of ['clip_protection', 'retention', 'settings']) {
  assert.ok(fs.existsSync(`src-tauri/src/db/${domain}.rs`),
    `${domain} persistence must remain outside the database integration root`);
}
for (const adapter of [
  'activity', 'analysis', 'app_lock', 'backups', 'content_registry', 'extraction', 'extractors',
  'factory_reset', 'imports', 'intelligence', 'manual_transforms', 'queue', 'retention',
  'source_apps', 'storage', 'transformations',
]) {
  assert.ok(fs.existsSync(`src-tauri/src/commands/${adapter}.rs`),
    `${adapter} GUI commands must remain outside the command integration root`);
}
const cliAdapters = [
  'activity', 'analyzer', 'app_lock', 'bins', 'classifiers', 'clips', 'connections',
  'extractors', 'history', 'inspectors', 'live_app', 'maintenance', 'operations',
  'portability', 'registry', 'retention', 'settings', 'storage', 'suggestions', 'transforms',
];
for (const adapter of cliAdapters) {
  assert.ok(fs.existsSync(`src-tauri/src/cli/commands/${adapter}.rs`),
    `${adapter} CLI commands must remain outside the CLI integration root`);
  assert.ok(lineCount(`src-tauri/src/cli/commands/${adapter}.rs`) <= 400,
    `${adapter} CLI adapter grew beyond 400 lines; split the capability again`);
}
for (const support of ['app_lock_support', 'common', 'extractor_support', 'retention_support', 'transform_support']) {
  assert.ok(fs.existsSync(`src-tauri/src/cli/commands/${support}.rs`),
    `${support} must keep shared CLI parsing and presentation outside the integration root`);
  assert.ok(lineCount(`src-tauri/src/cli/commands/${support}.rs`) <= 300,
    `${support} grew beyond 300 lines; separate its helper responsibilities`);
}
for (const dispatch of cliAdapters) {
  assert.match(cliRoot, new RegExp(`cli_commands::${dispatch}::`),
    `The CLI integration root must delegate ${dispatch} behavior`);
}
assert.doesNotMatch(commands, /pub fn (?:save_setting|configure_clip_retention)/,
  'Database domain persistence must not move into the GUI command adapter');
assert.doesNotMatch(
  commands,
  /pub (?:async )?fn (?:analyze_content|get_content_classifiers|get_content_extractors|create_content_type|create_content_classifier)/,
  'The GUI command integration root must not reclaim Analysis capability adapters',
);

console.log('Application architecture boundary audit passed.');
