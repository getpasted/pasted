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
const contentInspection = read('src-tauri/src/content_inspection.rs');
const intelligenceConnections = read('src-tauri/src/intelligence_connections.rs');
const queueActions = read('src-tauri/src/queue_actions.rs');
const platformCapabilities = read('src-tauri/src/platform_capabilities.rs');
const hotkeys = read('src-tauri/src/hotkey_manager.rs');
const settingsService = read('src-tauri/src/settings_service.rs');
const activityDatabase = read('src-tauri/src/db/activity.rs');
const analyticsDatabase = read('src-tauri/src/db/analytics.rs');
const binDatabase = read('src-tauri/src/db/bins.rs');
const captureDatabase = read('src-tauri/src/db/capture.rs');
const clipMutationDatabase = read('src-tauri/src/db/clip_mutations.rs');
const clipQueryDatabase = read('src-tauri/src/db/clip_queries.rs');
const clipRecordDatabase = read('src-tauri/src/db/clip_records.rs');
const clipRevisionDatabase = read('src-tauri/src/db/clip_revisions.rs');
const clipSearchDatabase = read('src-tauri/src/db/clip_search.rs');
const classifierDatabase = read('src-tauri/src/db/classifiers.rs');
const contentTypeRegistryDatabase = read('src-tauri/src/db/content_type_registry.rs');
const extractorDatabase = read('src-tauri/src/db/extractors.rs');
const fullBackupDatabase = read('src-tauri/src/db/full_backups.rs');
const intelligenceConnectionDatabase = read('src-tauri/src/db/intelligence_connections.rs');
const lifecycleDatabase = read('src-tauri/src/db/lifecycle.rs');
const operationDatabase = read('src-tauri/src/db/operations.rs');
const retentionDatabase = read('src-tauri/src/db/retention.rs');
const schemaDatabase = read('src-tauri/src/db/schema.rs');
const storedAnalysisDatabase = read('src-tauri/src/db/stored_analysis.rs');
const timestampDatabase = read('src-tauri/src/db/timestamps.rs');
const transferDatabase = read('src-tauri/src/db/transfers.rs');
const transformDatabase = read('src-tauri/src/db/transforms.rs');
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
const clipboardCommands = read('src-tauri/src/commands/clipboard.rs');
const binCommands = read('src-tauri/src/commands/bins.rs');
const captureCommands = read('src-tauri/src/commands/capture.rs');
const cliInstallationCommands = read('src-tauri/src/commands/cli_installation.rs');
const clipPolicyCommands = read('src-tauri/src/commands/clip_policies.rs');
const backupCommands = read('src-tauri/src/commands/backups.rs');
const importCommands = read('src-tauri/src/commands/imports.rs');
const factoryResetCommands = read('src-tauri/src/commands/factory_reset.rs');
const extractionCommands = read('src-tauri/src/commands/extraction.rs');
const filePreviewCommands = read('src-tauri/src/commands/file_previews.rs');
const intelligenceCommands = read('src-tauri/src/commands/intelligence.rs');
const libraryAccessCommands = read('src-tauri/src/commands/library_access.rs');
const manualTransformCommands = read('src-tauri/src/commands/manual_transforms.rs');
const sourceApplicationCommands = read('src-tauri/src/commands/source_apps.rs');
const transformationCommands = read('src-tauri/src/commands/transformations.rs');
const hotkeyCommands = read('src-tauri/src/commands/hotkeys.rs');
const hudCommands = read('src-tauri/src/commands/hud.rs');
const platformCommands = read('src-tauri/src/commands/platform.rs');
const retentionCommands = read('src-tauri/src/commands/retention.rs');
const settingsCommands = read('src-tauri/src/commands/settings.rs');
const appOverlays = read('src/hooks/useAppOverlays.ts');
const clipDragController = read('src/hooks/useClipDragController.ts');
const clipReordering = read('src/hooks/useClipReordering.ts');

assert.doesNotMatch(liveApp, /crate::commands::/,
  'The live-app adapter must not call the GUI command adapter');
assert.doesNotMatch(commands, /#\[cfg\(test\)\]/,
  'Cross-domain regressions must live with their owning subsystem instead of the GUI command root');
assert.match(activityDatabase, /pub fn export_activity_json/,
  'Activity portability must remain in its focused database subsystem');
assert.match(activityDatabase, /MAX_ACTIVITY_IMPORT_BYTES/,
  'Activity imports must remain bounded inside the Activity subsystem');
assert.match(activityDatabase, /duplicate_count/,
  'Activity imports must remain deduplicating inside the Activity subsystem');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn export_activity_json/,
  'The database integration root must not reclaim Activity persistence');
assert.match(analyticsDatabase, /pub fn get_analytics_summary/,
  'Insights aggregation must remain in its focused database subsystem');
assert.match(analyticsDatabase, /get_daily_activity_for_calendar[\s\S]*calendar_modifier/,
  'Insights calendar grouping must keep its explicit local-calendar boundary');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn get_analytics_summary/,
  'The database integration root must not reclaim Insights aggregation');
assert.match(clipQueryDatabase, /pub fn get_clips_page/,
  'Clip collection reads must remain in their focused database subsystem');
assert.match(clipQueryDatabase, /NULL as image_base64/,
  'Clip list reads must keep deferring image payloads to the image endpoint');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn get_clips_page/,
  'The database integration root must not reclaim clip collection reads');
assert.match(clipMutationDatabase, /pub fn batch_trash_clips/,
  'Clip lifecycle mutations must remain in their focused database subsystem');
assert.match(clipMutationDatabase, /effective_clip_protection/,
  'Destructive clip mutations must preserve effective protection checks');
assert.match(clipMutationDatabase, /pub fn batch_assign_bin_clips/,
  'Shared clip organization mutations must remain centralized');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn batch_trash_clips/,
  'The database integration root must not reclaim clip lifecycle mutations');
assert.match(intelligenceConnectionDatabase, /validate_credential_reference/,
  'Connection persistence must validate credential references without storing secrets');
assert.match(intelligenceConnectionDatabase, /pub fn reorder_intelligence_connections/,
  'Connection ordering must remain in its focused database subsystem');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn get_intelligence_connections/,
  'The database integration root must not reclaim Connection persistence');
assert.match(operationDatabase, /pub fn get_library_items/,
  'Library item visibility must remain centralized with operation persistence');
assert.match(operationDatabase, /Operation is used by/,
  'Operation deletion must continue protecting dependent Transforms');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn get_operations/,
  'The database integration root must not reclaim operation persistence');
assert.match(transformDatabase, /pub fn get_transform_definitions/,
  'Unified Transform definitions must remain in their focused database subsystem');
assert.match(transformDatabase, /pub fn begin_transformation_execution/,
  'Transformation execution records must remain with Transform persistence');
assert.match(transformDatabase, /pub fn apply_transform_output_to_clip/,
  'Atomic Transform application must remain with its provenance ledger');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn get_transform_definitions/,
  'The database integration root must not reclaim Transform persistence');
assert.match(contentTypeRegistryDatabase, /pub fn get_content_type_groups/,
  'Content Type Group persistence must remain in its focused registry subsystem');
assert.match(contentTypeRegistryDatabase, /pub fn set_content_type_archived/,
  'Content Type lifecycle policy must remain centralized with registry persistence');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn get_content_type_groups/,
  'The database integration root must not reclaim Content Type registry persistence');
assert.match(extractorDatabase, /pub fn get_content_extractors/,
  'Extractor persistence must remain in its focused database subsystem');
assert.match(extractorDatabase, /insert_extractor_authoring_session/,
  'Extractor authoring history must remain transactional with extractor persistence');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn get_content_extractors/,
  'The database integration root must not reclaim Extractor persistence');
assert.match(classifierDatabase, /pub fn get_content_classifiers/,
  'Classifier persistence must remain in its focused database subsystem');
assert.match(classifierDatabase, /pub fn rescan_content_classification/,
  'Classifier rescans must remain transactional with classifier persistence');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn get_content_classifiers/,
  'The database integration root must not reclaim Classifier persistence');
assert.match(storedAnalysisDatabase, /pub fn get_ocr_backfill_status/,
  'OCR lifecycle state must remain in the stored-analysis subsystem');
assert.match(storedAnalysisDatabase, /pub fn replace_analysis_classifications/,
  'Hash-safe analysis results must remain centralized with stored analysis');
assert.match(storedAnalysisDatabase, /pub fn replace_clip_searchable_text/,
  'Searchable extraction results must remain centralized with stored analysis');
assert.match(storedAnalysisDatabase, /pub fn record_extraction_observations/,
  'Extractor observation history must remain centralized with stored analysis');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn get_ocr_backfill_status/,
  'The database integration root must not reclaim stored analysis persistence');
assert.match(binDatabase, /pub fn get_bins/,
  'Bin reads must remain in the focused organization subsystem');
assert.match(binDatabase, /pub fn matching_smart_bin_transforms/,
  'Smart Bin Transform matching must remain centralized with Bin persistence');
assert.match(binDatabase, /pub fn delete_bin/,
  'Bin deletion policy must remain centralized with Bin persistence');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn get_bins/,
  'The database integration root must not reclaim Bin persistence');
assert.match(clipRevisionDatabase, /pub fn get_clip_versions_page/,
  'Clip revision reads must remain in the focused revision subsystem');
assert.match(clipRevisionDatabase, /pub fn restore_clip_version/,
  'Clip revision restoration must remain centralized with revision reads');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn restore_clip_version/,
  'The database integration root must not reclaim Clip revision persistence');
assert.match(fullBackupDatabase, /pub fn create_full_backup/,
  'Full Backup creation must remain in its focused lifecycle subsystem');
assert.match(fullBackupDatabase, /pub fn restore_full_backup/,
  'Full Restore recovery and activation must remain centralized with Full Backup lifecycle');
assert.match(fullBackupDatabase, /fn open_validated_full_backup/,
  'Full Backup integrity validation must remain private to its lifecycle subsystem');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn create_full_backup/,
  'The database integration root must not reclaim Full Backup lifecycle behavior');
assert.match(transferDatabase, /pub fn export_backup_json/,
  'History and Organization export must remain in its focused transfer subsystem');
assert.match(transferDatabase, /fn preflight_library_archive/,
  'Portable archive validation must complete before transfer mutation begins');
assert.match(transferDatabase, /pub fn import_backup_json/,
  'History and Organization import must remain centralized with transfer preflight');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn import_backup_json/,
  'The database integration root must not reclaim transfer persistence');
assert.match(retentionDatabase, /pub fn enforce_history_limit_internal/,
  'History retention enforcement must remain centralized with retention configuration');
assert.match(retentionDatabase, /pub fn enforce_trash_limit_internal/,
  'Trash retention enforcement must remain centralized with retention configuration');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn enforce_history_limit_internal/,
  'The database integration root must not reclaim retention enforcement');
assert.match(schemaDatabase, /pub\(super\) fn init_tables/,
  'Database schema activation must remain in its focused schema subsystem');
assert.match(schemaDatabase, /fn migrate_legacy_container_schema/,
  'Legacy database migrations must remain centralized with schema activation');
assert.match(schemaDatabase, /fn run_named_migrations/,
  'Atomic named migrations must remain centralized with schema activation');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /fn init_tables|struct NamedMigration|fn migrate_legacy_container_schema/,
  'The database integration root must not reclaim schema activation or migrations');
assert.match(lifecycleDatabase, /pub fn open_pasted_database/,
  'Shared database opening policy must remain in the database lifecycle subsystem');
assert.match(lifecycleDatabase, /pub fn relocate_database/,
  'Database relocation must remain in the database lifecycle subsystem');
assert.match(lifecycleDatabase, /pub fn factory_reset/,
  'Factory Reset persistence must remain in the database lifecycle subsystem');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /fn configure_connection|pub fn relocate_database|pub fn factory_reset/,
  'The database integration root must not reclaim database lifecycle operations');
assert.match(captureDatabase, /pub fn save_clip/,
  'Shared GUI and CLI clip ingestion must remain in the capture subsystem');
assert.match(captureDatabase, /fn persist_capture_structure/,
  'Capture structure persistence must remain atomic with clip ingestion');
assert.match(captureDatabase, /pub fn reattribute_image_capture/,
  'Image capture reattribution must remain hash-safe inside the capture subsystem');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn save_clip|fn persist_capture_structure|pub fn reattribute_image_capture/,
  'The database integration root must not reclaim capture ingestion');
assert.match(clipSearchDatabase, /pub fn search_clips/,
  'Authoritative paginated clip search must remain with the shared search grammar');
assert.match(clipSearchDatabase, /fn clip_search_feature_policy/,
  'Search feature gates must remain centralized with authoritative search');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn search_clips|pub fn get_total_clip_count/,
  'The database integration root must not reclaim authoritative clip search');
assert.match(clipRecordDatabase, /pub struct ClipItem/,
  'The canonical clip record must remain in its focused record subsystem');
assert.match(clipRecordDatabase, /fn clip_item_from_row/,
  'SQLite clip hydration must remain centralized with the canonical clip record');
assert.match(clipRecordDatabase, /fn append_smart_bin_memberships/,
  'Computed clip organization must remain centralized with clip hydration');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub struct ClipItem|fn clip_item_from_row|fn append_smart_bin_memberships/,
  'The database integration root must not reclaim clip records or hydration');
assert.match(timestampDatabase, /fn canonical_utc_timestamp/,
  'Canonical UTC normalization must remain in the timestamp policy subsystem');
assert.match(timestampDatabase, /fn migrate_canonical_timestamps/,
  'Legacy UTC migration must remain centralized with timestamp policy');
assert.match(timestampDatabase, /fn normalize_library_archive_timestamps/,
  'Transfer timestamp normalization must remain centralized with timestamp policy');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /fn canonical_utc_timestamp|fn migrate_canonical_timestamps|fn normalize_library_archive_timestamps/,
  'The database integration root must not reclaim timestamp policy');
assert.match(clipboardActions, /fn ocr_text_never_replaces_an_image_clips_copy_fingerprint/,
  'Clipboard fingerprint regressions must remain with the shared clipboard workflow');
assert.match(contentInspection, /fn file_metadata_reports_availability_without_crawling_directories/,
  'File observation regressions must remain with content inspection');
assert.match(intelligenceConnections, /fn intelligence_credentials_must_remain_references/,
  'Credential-reference validation regressions must remain with intelligence connections');
assert.match(platformCapabilities, /fn accessibility_status_reports_the_build_mode/,
  'Accessibility status regressions must remain with platform capabilities');

for (const sharedCall of [
  'clipboard_actions::copy_clip',
  'clipboard_actions::paste_clip',
  'queue_actions::paste_item',
  'queue_actions::paste_all',
]) {
  assert.match(liveApp, new RegExp(sharedCall),
    `The live-app adapter must use ${sharedCall}`);
}

assert.match(clipboardCommands, /clipboard_actions::copy_clip/,
  'GUI copy must use the shared clipboard workflow');
assert.match(clipboardCommands, /clipboard_actions::paste_hud_clip/,
  'GUI paste must use the shared clipboard workflow');
assert.match(hudCommands, /pub fn toggle_hud_window[\s\S]*hud_window::reveal/,
  'HUD visibility must remain in its focused window adapter');
assert.match(hotkeyCommands, /pub fn register_all_app_shortcuts/,
  'Native shortcut registration must remain in its focused hotkey adapter');
assert.match(hotkeyCommands, /pub fn get_hotkey_capability_status/,
  'Hotkey platform readiness must remain with shortcut registration');
assert.doesNotMatch(commands, /pub fn copy_clip_to_system|pub fn paste_clip_by_id|pub fn toggle_hud_window/,
  'The GUI command root must not reclaim clipboard or HUD operations');
assert.doesNotMatch(commands, /pub fn register_all_app_shortcuts|pub fn get_hotkey_capability_status|pub fn register_app_setting_hotkeys/,
  'The GUI command root must not reclaim hotkey registration or readiness');
assert.match(settingsCommands, /pub fn save_app_setting[\s\S]*settings_service::update_setting/,
  'Settings persistence must remain delegated to the shared Settings service');
assert.match(settingsCommands, /pub fn save_app_settings[\s\S]*settings_service::update_settings/,
  'Batch Settings persistence must remain delegated to the shared Settings service');
assert.match(settingsCommands, /apply_feature_policy_changes[\s\S]*register_all_app_shortcuts/,
  'Settings runtime changes must coordinate feature shutdown and hotkey refresh in one adapter');
assert.match(platformCommands, /pub fn perform_titlebar_double_click[\s\S]*titlebar::perform_titlebar_double_click/,
  'Platform shell commands must delegate titlebar behavior to the shared titlebar service');
assert.match(platformCommands, /pub fn get_installation_diagnostics/,
  'Installation diagnostics must remain in the platform shell adapter');
assert.doesNotMatch(platformCommands, /#\[tauri::command\]\s*#\[tauri::command\]/,
  'Platform commands must not carry duplicate Tauri command attributes');
assert.doesNotMatch(commands, /pub fn save_app_setting|pub fn set_linux_native_menu_theme|pub fn open_backing_page/,
  'The GUI command root must not reclaim Settings or platform shell operations');
assert.match(libraryAccessCommands, /pub async fn search_clips[\s\S]*spawn_blocking/,
  'Authoritative Clip search must remain outside async command dispatch');
assert.match(libraryAccessCommands, /pub fn export_clips_json[\s\S]*db\.export_clips_json/,
  'Clip exports must delegate to the shared database contract');
assert.match(captureCommands, /pub fn toggle_clipboard_pause[\s\S]*emit_clipboard_pause_changed/,
  'Capture pause changes must publish the shared application event');
assert.doesNotMatch(commands, /pub async fn search_clips|pub fn toggle_clipboard_pause|pub fn export_clips_json|pub fn get_analytics_summary/,
  'The GUI command root must not reclaim library access or capture state operations');
assert.match(cliInstallationCommands, /symlink_metadata\(&symlink_path\)/,
  'CLI installation must inspect an existing destination without following its link');
assert.match(cliInstallationCommands, /Refusing to replace existing (?:CLI link|file)/,
  'CLI installation must refuse to overwrite user-owned destinations');
assert.match(cliInstallationCommands, /fn cli_install_is_idempotent_for_its_existing_link/,
  'CLI installation must retain its idempotency regression test beside the adapter');
assert.doesNotMatch(commands, /pub fn install_cli_to_path|fn install_cli_symlink/,
  'The GUI command root must not reclaim CLI installation behavior');
assert.match(binCommands, /pub fn create_bin[\s\S]*refresh_native_app_menu/,
  'Bin lifecycle commands must remain in their focused organization adapter');
assert.match(hotkeyCommands, /pub fn update_clip_hotkey[\s\S]*restore_clip_hotkey_state/,
  'Clip hotkey changes must preserve transactional rollback in the hotkey adapter');
assert.match(clipPolicyCommands, /pub fn batch_protect_clips/,
  'Clip protection mutations must remain with concealment and policy commands');
assert.match(retentionCommands, /pub fn trash_unpinned_clips/,
  'Bulk unpinned cleanup must remain with retention commands');
assert.doesNotMatch(commands, /pub fn get_bins|pub fn update_clip_hotkey|pub fn batch_protect_clips|pub fn trash_unpinned_clips/,
  'The GUI command root must not reclaim Bin, hotkey, protection, or retention operations');
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
  ['src-tauri/src/db.rs', 691],
  ['src-tauri/src/db/activity.rs', 649],
  ['src-tauri/src/db/analytics.rs', 159],
  ['src-tauri/src/db/bins.rs', 652],
  ['src-tauri/src/db/capture.rs', 268],
  ['src-tauri/src/db/clip_mutations.rs', 643],
  ['src-tauri/src/db/clip_queries.rs', 442],
  ['src-tauri/src/db/clip_records.rs', 564],
  ['src-tauri/src/db/clip_revisions.rs', 169],
  ['src-tauri/src/db/clip_search.rs', 532],
  ['src-tauri/src/db/classifiers.rs', 533],
  ['src-tauri/src/db/content_type_registry.rs', 363],
  ['src-tauri/src/db/extractors.rs', 802],
  ['src-tauri/src/db/full_backups.rs', 277],
  ['src-tauri/src/db/intelligence_connections.rs', 231],
  ['src-tauri/src/db/lifecycle.rs', 233],
  ['src-tauri/src/db/operations.rs', 377],
  ['src-tauri/src/db/retention.rs', 293],
  ['src-tauri/src/db/schema.rs', 2_321],
  ['src-tauri/src/db/stored_analysis.rs', 818],
  ['src-tauri/src/db/timestamps.rs', 131],
  ['src-tauri/src/db/transfers.rs', 1_446],
  ['src-tauri/src/db/transforms.rs', 1_084],
  ['src-tauri/src/db/tests/mod.rs', 54],
  ['src-tauri/src/db/tests/bins_and_transforms.rs', 611],
  ['src-tauri/src/db/tests/capture_and_lifecycle.rs', 797],
  ['src-tauri/src/db/tests/migrations_and_intelligence.rs', 1_422],
  ['src-tauri/src/db/tests/retention_and_activity.rs', 366],
  ['src-tauri/src/db/tests/revisions_and_mutations.rs', 495],
  ['src-tauri/src/db/tests/search_and_operations.rs', 1_126],
  ['src-tauri/src/db/tests/transfer_and_portability.rs', 1_069],
  ['src-tauri/src/db/tests/transforms_backup_and_protection.rs', 893],
  ['src-tauri/src/commands.rs', 54],
  ['src-tauri/src/commands/bins.rs', 89],
  ['src-tauri/src/commands/capture.rs', 43],
  ['src-tauri/src/commands/cli_installation.rs', 136],
  ['src-tauri/src/commands/clip_policies.rs', 70],
  ['src-tauri/src/commands/clipboard.rs', 108],
  ['src-tauri/src/commands/hotkeys.rs', 371],
  ['src-tauri/src/commands/hud.rs', 170],
  ['src-tauri/src/commands/platform.rs', 175],
  ['src-tauri/src/commands/retention.rs', 54],
  ['src-tauri/src/commands/settings.rs', 121],
  ['src-tauri/src/commands/backups.rs', 180],
  ['src-tauri/src/commands/imports.rs', 287],
  ['src-tauri/src/commands/factory_reset.rs', 39],
  ['src-tauri/src/commands/extraction.rs', 187],
  ['src-tauri/src/commands/clips.rs', 261],
  ['src-tauri/src/commands/file_previews.rs', 714],
  ['src-tauri/src/commands/intelligence.rs', 271],
  ['src-tauri/src/commands/library_access.rs', 38],
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

for (const domain of ['analytics', 'clip_protection', 'retention', 'settings']) {
  assert.ok(fs.existsSync(`src-tauri/src/db/${domain}.rs`),
    `${domain} persistence must remain outside the database integration root`);
}
for (const adapter of [
  'activity', 'analysis', 'app_lock', 'backups', 'bins', 'capture', 'cli_installation',
  'clip_policies', 'clipboard',
  'content_registry', 'extraction', 'extractors', 'factory_reset', 'hotkeys', 'hud', 'imports',
  'intelligence', 'library_access', 'manual_transforms', 'platform', 'queue', 'retention', 'settings',
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
