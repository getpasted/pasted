import assert from 'node:assert/strict';
import fs from 'node:fs';
import { readRustModuleTree } from './audit-source-trees.js';

const read = (path) => fs.readFileSync(path, 'utf8');
const lineCount = (path) => read(path).trimEnd().split(/\r?\n/).length;
const readSourceTree = (directory) => fs.readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
  const path = `${directory}/${entry.name}`;
  if (entry.isDirectory()) return readSourceTree(path);
  return /\.(?:ts|tsx)$/.test(entry.name) ? [{ path, source: read(path) }] : [];
});
const nativeAppRoot = read('src-tauri/src/lib.rs');
const applicationArchitecture = read('docs/APPLICATION_ARCHITECTURE.md');
const analysisArchitecture = read('docs/ANALYSIS_ARCHITECTURE.md');
const transformationsArchitecture = read('docs/TRANSFORMATIONS.md');
const nativeAppRuntime = read('src-tauri/src/app_runtime.rs');
const nativeAppTray = read('src-tauri/src/app_tray.rs');
const nativeAppWindows = read('src-tauri/src/app_windows.rs');
const clipboardCapturePolicy = read('src-tauri/src/clipboard_capture_policy.rs');
const clipboardIngestion = readRustModuleTree(
  'src-tauri/src/clipboard_ingestion/mod.rs',
  'src-tauri/src/clipboard_ingestion',
);
const clipboardMonitor = read('src-tauri/src/clipboard_monitor.rs');
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
const hotkeys = readRustModuleTree(
  'src-tauri/src/hotkey_manager.rs',
  'src-tauri/src/hotkey_manager',
);
const pasteTarget = read('src-tauri/src/paste_target.rs');
const appLock = read('src-tauri/src/app_lock.rs');
const settingsService = read('src-tauri/src/settings_service.rs');
const activityDatabase = read('src-tauri/src/db/activity.rs');
const analysisActivityDatabase = read('src-tauri/src/db/analysis_activity.rs');
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
const contractDatabase = read('src-tauri/src/db/contracts.rs');
const extractorDatabase = read('src-tauri/src/db/extractors.rs');
const fullBackupDatabase = read('src-tauri/src/db/full_backups.rs');
const intelligenceConnectionDatabase = readRustModuleTree(
  'src-tauri/src/db/intelligence_connections.rs',
  'src-tauri/src/db/intelligence_connections',
);
const lifecycleDatabase = read('src-tauri/src/db/lifecycle.rs');
const maintenanceDatabase = read('src-tauri/src/db/maintenance.rs');
const operationDatabase = read('src-tauri/src/db/operations.rs');
const retentionDatabase = read('src-tauri/src/db/retention.rs');
const schemaDatabase = readRustModuleTree('src-tauri/src/db/schema.rs', 'src-tauri/src/db/schema');
const sourceQueryDatabase = read('src-tauri/src/db/source_queries.rs');
const storedAnalysisFacade = read('src-tauri/src/db/stored_analysis.rs');
const storedAnalysisDatabase = readRustModuleTree(
  'src-tauri/src/db/stored_analysis.rs',
  'src-tauri/src/db/stored_analysis',
);
const timestampDatabase = read('src-tauri/src/db/timestamps.rs');
const timestampMigrations = read('src-tauri/src/db/timestamps/migrations.rs');
const transferDatabase = readRustModuleTree(
  'src-tauri/src/db/transfers.rs',
  'src-tauri/src/db/transfers',
);
const transformDatabase = readRustModuleTree(
  'src-tauri/src/db/transforms.rs',
  'src-tauri/src/db/transforms',
);
const settingsApi = read('src/api/settings.ts');
const transformsApi = read('src/api/transforms.ts');
const localizationRuntime = read('src/localization/runtime.ts');
const cliRoot = read('src-tauri/src/bin/pasted.rs');
const appRoot = [
  'src/App.tsx',
  'src/hooks/useAppController.ts',
  'src/hooks/useAppLibraryActions.ts',
  'src/components/AppShellView.tsx',
  'src/components/AppDestinationView.tsx',
].map(read).join('\n');
const appNavigation = read('src/hooks/useAppNavigation.ts');
const appShell = read('src/hooks/useAppShell.ts');
const appMenuActions = read('src/hooks/useAppMenuActions.ts');
const clipSelectionController = read('src/hooks/useClipSelectionController.ts');
const clipListViewport = read('src/hooks/useClipListViewport.ts');
const rememberedClipListScroll = read('src/hooks/useRememberedClipListScroll.ts');
const browserRuntime = read('src/mocks/browser/runtime.ts');
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
const ocrBackfillCommands = read('src-tauri/src/commands/extraction/ocr_backfill.rs');
const filePreviewCommands = read('src-tauri/src/commands/file_previews.rs');
const fileReferenceHealth = read('src-tauri/src/file_reference_health.rs');
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

assert.match(applicationArchitecture, /## Native ownership map/,
  'Application architecture must document the final native ownership map');
for (const [root, adapter, applicationService, persistence, platform] of [
  ['Native crate bootstrap', 'lib.rs', 'app_runtime.rs', 'db::DbState', 'Tauri window'],
  ['Clipboard monitor', 'clipboard_monitor.rs', 'clipboard_capture_policy.rs', 'db/capture.rs', 'arboard'],
  ['Extraction runtime', 'content_extraction.rs', 'extraction_execution.rs', 'db/stored_analysis/', 'engine_runtime/'],
  ['Intelligence executor', 'intelligence_executor.rs', 'intelligence_executor/', 'db/intelligence_connections.rs', 'intelligence_provider.rs'],
  ['Transformation service', 'transformation_service.rs', 'transformation_service/', 'db/transforms/', 'clipboard_actions.rs'],
  ['Database schema', 'db/schema.rs', 'db/schema/canonical.rs', 'db/schema/', 'db/lifecycle.rs'],
  ['Database transfers', 'db/transfers.rs', 'library_validation', 'library_import', 'native file pickers'],
  ['Database Transforms', 'db/transforms.rs', 'transformation_service.rs', 'applications', 'No platform behavior'],
  ['Stored Analysis persistence', 'db/stored_analysis.rs', 'Participant execution modules', 'classifications', 'live observations'],
]) {
  const row = applicationArchitecture.split(/\r?\n/)
    .find((line) => line.includes(`| ${root} (`));
  assert.ok(row, `Application architecture must include the ${root} epic root`);
  for (const owner of [adapter, applicationService, persistence, platform]) {
    assert.ok(row.includes(owner), `${root} ownership must identify ${owner}`);
  }
}
assert.match(applicationArchitecture,
  /`content_extraction\.rs` is the deliberate size exception[\s\S]*cohesive contract and definition module[\s\S]*It is not the engine registry/,
  'Application architecture must document the cohesive content-extraction exception');
assert.match(analysisArchitecture,
  /`db\/stored_analysis\.rs` is a declaration-only facade[\s\S]*`classifications\.rs`[\s\S]*`inspections\.rs`[\s\S]*`extractions\.rs`[\s\S]*`attempts\.rs`[\s\S]*`types\.rs`[\s\S]*`searchable_text\.rs`[\s\S]*`ocr\.rs`/,
  'Analysis architecture must identify every stored Analysis persistence owner');
assert.match(transformationsArchitecture,
  /## Native ownership[\s\S]*`transformation_service\.rs` is the shared application-service facade[\s\S]*`db\/transforms\.rs` is the persistence facade[\s\S]*`applications\.rs`/,
  'Transformation documentation must distinguish execution from persistence ownership');

const epicBaselineRootLines = new Map([
  ['src-tauri/src/lib.rs', 803],
  ['src-tauri/src/clipboard_monitor.rs', 955],
  ['src-tauri/src/content_extraction.rs', 1114],
  ['src-tauri/src/intelligence_executor.rs', 1393],
  ['src-tauri/src/transformation_service.rs', 1166],
  ['src-tauri/src/db/schema.rs', 2321],
  ['src-tauri/src/db/transfers.rs', 1446],
  ['src-tauri/src/db/transforms.rs', 1084],
  ['src-tauri/src/db/stored_analysis.rs', 818],
]);
for (const [path, baseline] of epicBaselineRootLines) {
  assert.ok(lineCount(path) < baseline,
    `${path} must remain smaller than its ${baseline}-line native architecture epic baseline`);
}

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
const transformFacade = read('src-tauri/src/db/transforms.rs');
for (const capability of [
  'applications', 'definitions', 'executions', 'manual',
  'operation_compatibility', 'repository', 'types',
]) {
  assert.match(transformFacade, new RegExp(`mod ${capability}`),
    `The Transform facade must compose the ${capability} capability`);
}
assert.doesNotMatch(transformFacade, /impl DbState|SELECT |INSERT |UPDATE |DELETE /,
  'The Transform facade must not reclaim persistence implementation');
for (const [path, method, owner] of [
  ['applications.rs', 'apply_transform_output_to_clip', 'application and provenance'],
  ['definitions.rs', 'get_transform_definitions', 'definition lifecycle'],
  ['executions.rs', 'begin_transformation_execution', 'execution lifecycle'],
  ['manual.rs', 'validate_pipeline_steps', 'manual compatibility'],
  ['operation_compatibility.rs', 'operation_storage_fields', 'operation compatibility'],
  ['repository.rs', 'saved_transform_by_id', 'shared row decoding'],
]) {
  assert.match(read(`src-tauri/src/db/transforms/${path}`), new RegExp(method),
    `Transform ${owner} must remain in ${path}`);
}
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
for (const owner of ['ocr', 'classifications', 'searchable_text', 'inspections', 'extractions']) {
  assert.match(storedAnalysisFacade, new RegExp(`mod ${owner};`),
    `Stored Analysis must delegate ${owner} persistence to its focused owner`);
}
assert.doesNotMatch(storedAnalysisFacade, /pub fn|impl DbState/,
  'The Stored Analysis facade must remain a declaration-only integration surface');
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
const transferFacade = read('src-tauri/src/db/transfers.rs');
for (const capability of [
  'clip_transfer', 'library_export', 'library_import', 'library_validation',
]) {
  assert.match(transferFacade, new RegExp(`mod ${capability}`),
    `The transfer facade must compose the ${capability} capability`);
}
assert.doesNotMatch(transferFacade, /impl DbState|pub fn|fn preflight_library_archive/,
  'The transfer facade must not reclaim Clip or History and Organization persistence');
assert.match(read('src-tauri/src/db/transfers/library_export.rs'), /pub fn export_backup_json/,
  'History and Organization export must have one focused owner');
assert.match(read('src-tauri/src/db/transfers/library_validation.rs'), /fn preflight_library_archive/,
  'History and Organization validation must have one focused owner');
assert.match(read('src-tauri/src/db/transfers/library_import.rs'), /pub fn import_backup_json/,
  'History and Organization transactional merge must have one focused owner');
assert.match(retentionDatabase, /pub fn enforce_history_limit_internal/,
  'History retention enforcement must remain centralized with retention configuration');
assert.match(retentionDatabase, /pub fn enforce_trash_limit_internal/,
  'Trash retention enforcement must remain centralized with retention configuration');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub fn enforce_history_limit_internal/,
  'The database integration root must not reclaim retention enforcement');
assert.match(schemaDatabase, /pub\(in crate::db\) fn init_tables/,
  'Database schema activation must remain in its focused schema subsystem');
assert.match(schemaDatabase, /fn migrate_legacy_container_schema/,
  'Legacy database migrations must remain centralized with schema activation');
assert.match(schemaDatabase, /fn run_named_migrations/,
  'Atomic named migrations must remain centralized with schema activation');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /fn init_tables|struct NamedMigration|fn migrate_legacy_container_schema/,
  'The database integration root must not reclaim schema activation or migrations');
const schemaFacade = read('src-tauri/src/db/schema.rs');
for (const capability of ['canonical', 'helpers', 'library_items', 'migrations', 'registry', 'transformation_tables']) {
  assert.match(schemaFacade, new RegExp(`mod ${capability}`),
    `The schema facade must compose the ${capability} capability`);
}
assert.doesNotMatch(schemaFacade, /CREATE TABLE|fn init_tables|fn run_named_migrations/,
  'The schema facade must not reclaim schema definitions or migration execution');
const canonicalSchema = read('src-tauri/src/db/schema/canonical.rs');
for (const phase of [
  'initialize_clip_schema', 'initialize_organization_schema', 'init_transformation_tables',
  'initialize_content_registry', 'initialize_extractor_registry', 'finalize_content_registry',
]) {
  assert.match(canonicalSchema, new RegExp(`${phase}\\(&conn\\)`),
    `Canonical schema activation must retain the ordered ${phase} phase`);
}
const migrationRegistry = read('src-tauri/src/db/schema/registry.rs');
assert.match(migrationRegistry, /const MIGRATIONS: &\[NamedMigration\]/,
  'Named migration registration must have one ordered declarative owner');
for (const key of ['appExclusionHotkeysV1', 'transformTerminologyV1', 'currentTransformationBackfillV1']) {
  assert.match(migrationRegistry, new RegExp(key), `Named migration ${key} must remain registered`);
}
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
assert.match(timestampMigrations, /fn migrate_canonical_timestamps/,
  'Legacy UTC migration must remain centralized with timestamp policy');
assert.match(timestampMigrations, /fn migrate_analysis_transform_timestamps/,
  'Analysis and Transform UTC migration must remain centralized with timestamp policy');
assert.match(read('src-tauri/src/db/schema/registry.rs'),
  /analysisTransformCanonicalTimestampsV1/,
  'Analysis and Transform UTC normalization must remain a named migration');
for (const path of [
  'src-tauri/src/db/stored_analysis/classifications.rs',
  'src-tauri/src/db/stored_analysis/extractions.rs',
  'src-tauri/src/db/stored_analysis/inspections.rs',
  'src-tauri/src/db/stored_analysis/searchable_text.rs',
  'src-tauri/src/db/transforms/applications.rs',
  'src-tauri/src/db/transforms/definitions.rs',
  'src-tauri/src/db/transforms/executions.rs',
  'src-tauri/src/db/transforms/manual.rs',
]) {
  assert.doesNotMatch(read(path), /CURRENT_TIMESTAMP/,
    `${path} must write canonical UTC timestamps explicitly`);
}
assert.match(timestampDatabase, /fn normalize_library_archive_timestamps/,
  'Transfer timestamp normalization must remain centralized with timestamp policy');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /fn canonical_utc_timestamp|fn migrate_canonical_timestamps|fn normalize_library_archive_timestamps/,
  'The database integration root must not reclaim timestamp policy');
assert.match(analysisActivityDatabase, /fn log_analysis_participant_update/,
  'Analysis participant lifecycle events must remain in the Activity adapter');
assert.match(maintenanceDatabase, /pub fn clear_history/,
  'History clearing must remain in the focused maintenance subsystem');
assert.match(maintenanceDatabase, /pub fn rescan_file_formats/,
  'File-format rescans must remain in the focused maintenance subsystem');
assert.match(sourceQueryDatabase, /pub fn get_distinct_sources/,
  'Source discovery must remain in its focused query subsystem');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /fn log_analysis_participant_update|pub fn clear_history|pub fn rescan_file_formats|pub fn get_distinct_sources/,
  'The database integration root must not reclaim focused runtime operations');
assert.match(contractDatabase, /pub struct DbState/,
  'Shared database state must remain in the database contract subsystem');
assert.match(contractDatabase, /pub struct BackupPayload/,
  'Portable backup data contracts must remain in the database contract subsystem');
assert.match(contractDatabase, /pub struct ClipMutationSummary/,
  'Shared mutation results must remain in the database contract subsystem');
assert.doesNotMatch(read('src-tauri/src/db.rs'), /pub struct DbState|pub struct BackupPayload|pub struct ClipMutationSummary/,
  'The database integration root must not reclaim shared data contracts');
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
assert.match(pasteTarget, /mod platform;/,
  'Paste targeting must delegate operating-system integration to focused platform modules');
assert.doesNotMatch(pasteTarget, /AXUIElement|GetForegroundWindow|xdotool/,
  'The Paste Target coordinator must not reclaim platform APIs');
assert.match(appLock, /mod platform_auth;/,
  'App Lock must delegate native authentication to its focused adapter');
assert.doesNotMatch(appLock, /LAContext|UserConsentVerifier|Windows Hello/,
  'The App Lock domain must not reclaim native authentication APIs');
assert.ok(lineCount('src-tauri/src/hotkey_manager.rs') <= 228,
  'The hotkey coordinator must stay within its extracted size boundary');
assert.match(read('src-tauri/src/hotkey_manager/action_dispatch.rs'), /pub fn dispatch/,
  'Hotkey action execution must remain in its focused subsystem');
assert.match(read('src-tauri/src/hotkey_manager/registration.rs'), /pub fn register_all/,
  'Hotkey configuration assembly must remain in its focused subsystem');
assert.match(read('src-tauri/src/hotkey_manager/native_backend.rs'), /fn register_native/,
  'Native hotkey registration must remain in its focused backend');
assert.match(read('src-tauri/src/hotkey_manager/wayland_backend.rs'), /fn prepare_xdg_hotkeys/,
  'Wayland portal preparation must remain in its focused backend');
assert.match(read('src-tauri/src/hotkey_manager/x11_backend.rs'), /fn rebuild_x11_shortcuts/,
  'X11 registration lifecycle must remain in its focused backend');
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
assert.match(nativeAppRoot, /\.setup\(app_runtime::setup\)/,
  'The native crate root must delegate runtime initialization');
assert.match(nativeAppRoot, /\.on_window_event\(app_windows::handle_window_event\)/,
  'The native crate root must delegate native window lifecycle events');
assert.match(nativeAppRoot, /app_runtime::handle_single_instance/,
  'The native crate root must delegate single-instance activation');
assert.match(nativeAppRoot, /app_windows::mark_main_page_loaded/,
  'The native crate root must delegate the startup readiness handshake');
assert.match(nativeAppRoot, /app_runtime::handle_run_event/,
  'The native crate root must delegate application run events');
assert.doesNotMatch(nativeAppRoot, /DbState::new|TrayIconBuilder|MAIN_PAGE_LOADED|STARTUP_SETUP_READY/,
  'The native crate root must not reclaim runtime, tray, or window state ownership');
assert.match(nativeAppRuntime, /app_windows::configure_initial_windows[\s\S]*app_tray::install[\s\S]*app_windows::mark_startup_setup_ready/,
  'Runtime initialization must preserve window, service, tray, and ready ordering');
assert.match(nativeAppTray, /fn build_menu[\s\S]*pub\(crate\) fn install/,
  'The tray module must own menu construction and installation');
assert.match(nativeAppWindows, /MAIN_PAGE_LOADED[\s\S]*STARTUP_SETUP_READY[\s\S]*MAIN_WINDOW_REVEALED/,
  'The window module must own the atomic startup reveal handshake');
assert.match(clipboardMonitor, /use crate::clipboard_capture_policy::/,
  'The clipboard monitor must delegate deterministic capture policy');
assert.doesNotMatch(clipboardMonitor, /fn inferred_screenshot_source|fn resolved_capture_source|fn should_coalesce_recent_image/,
  'The clipboard monitor must not reclaim source attribution or coalescing policy');
assert.match(clipboardCapturePolicy, /fn resolved_capture_source[\s\S]*fn should_prefer_composite_image[\s\S]*fn should_coalesce_recent_image/,
  'Capture policy must own source attribution and composite/recent-image decisions');
assert.match(clipboardCapturePolicy, /cfg\(target_os = "macos"\)[\s\S]*fn clipboard_change_marker[\s\S]*cfg\(not\(target_os = "macos"\)\)/,
  'Platform pasteboard generation must retain explicit portable gating');
for (const handler of ['ingest_files', 'ingest_text', 'ingest_image']) {
  assert.match(clipboardMonitor, new RegExp(handler),
    `The clipboard monitor must delegate ${handler}`);
  assert.match(clipboardIngestion, new RegExp(`fn ${handler}`),
    `Clipboard ingestion must own ${handler}`);
}
assert.doesNotMatch(clipboardMonitor, /save_text_clip|save_clip\("(?:file|image)"|rgba_to_encoded_image/,
  'The clipboard monitor must not reclaim payload persistence or image encoding');
assert.match(clipboardIngestion, /struct CaptureContext[\s\S]*fn capture_preflight/,
  'Clipboard payload handlers must share one context and deduplication preflight');
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
const appEntry = read('src/App.tsx');
assert.match(appEntry, /useAppController\(\)[\s\S]*<AppShellView controller=\{controller\}/,
  'The application entry must compose the controller and shell view boundaries');
assert.doesNotMatch(appEntry, /QuickHudWindow|CaptureFeedbackWindow/,
  'The main application entry must not reclaim auxiliary windows');
assert.doesNotMatch(appEntry, /useAppData|useClipActions|<Sidebar|<ClipCard/,
  'The application entry must not reclaim controller or workspace internals');
assert.doesNotMatch(read('src/hooks/useAppController.ts'), /<Sidebar|<ClipCard|<SettingsModal/,
  'The application controller must remain presentation-free');
assert.match(read('src/components/AppShellView.tsx'), /<AppDestinationView controller=\{controller\} renderClipWorkspace=/,
  'The application shell must delegate non-Clip destinations to their router');
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
assert.match(filePreviewCommands, /resolve_file_reference_health/,
  'File previews must delegate reference checks to the shared health service');
assert.match(fileReferenceHealth, /clip_file_reference_health[\s\S]*retry_interval/,
  'File reference failures must use persistent bounded retry state');
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
assert.match(browserRuntime, /const handlers: BrowserHandler\[\]/,
  'Browser IPC must compose focused domain handlers through one dispatcher');
assert.doesNotMatch(read('src/utils/tauri.ts'), /switch \(cmd\)|case '/,
  'The native transport facade must not reclaim browser-domain behavior');
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
assert.match(extractionCommands, /mod ocr_backfill/,
  'The extraction adapter must compose focused OCR backfill commands');
assert.match(ocrBackfillCommands, /pub fn start_ocr_backfill/,
  'OCR backfill control must remain with extraction lifecycle commands');
assert.doesNotMatch(commands, /pub fn extract_ocr_from_clip|pub async fn extract_text_from_file_clip|pub fn start_ocr_backfill/,
  'The GUI command root must not reclaim extraction lifecycle operations');

const contentAnalysisTests = read('src-tauri/src/content_analysis/tests.rs');
for (const owner of ['fixtures', 'extraction_classification', 'failure_contracts', 'scheduler_policy']) {
  assert.match(contentAnalysisTests, new RegExp(`mod ${owner};`),
    `Content Analysis tests must compose the focused ${owner} owner`);
}
assert.doesNotMatch(contentAnalysisTests, /#\[test\]|fn analyze_test_|struct .*Engine/,
  'The Content Analysis test facade must not reclaim fixtures or behavior tests');

const cliIntegrationTestFacade = read('src-tauri/tests/cli_integration.rs');
assert.match(cliIntegrationTestFacade, /cli_integration\/mod\.rs[\s\S]*mod cli_integration;/,
  'The CLI integration test target must delegate to its focused module tree');
assert.doesNotMatch(cliIntegrationTestFacade, /#\[test\]|fn success_json|fn temporary_path/,
  'The CLI integration test target must not reclaim shared support or domain contracts');
const cliIntegrationTests = read('src-tauri/tests/cli_integration/mod.rs');
for (const owner of ['analysis', 'lifecycle_policy', 'portability_library', 'registry_authoring', 'support']) {
  assert.match(cliIntegrationTests, new RegExp(`mod ${owner};`),
    `CLI integration tests must compose the focused ${owner} owner`);
}
assert.doesNotMatch(cliIntegrationTests, /#\[test\]|fn success_json|fn temporary_path/,
  'The CLI integration module facade must not reclaim shared support or domain contracts');

for (const [facade, owners] of [
  ['bins_and_transforms', ['bins_and_settings', 'legacy_transforms']],
  ['capture_and_lifecycle', [
    'capture_and_origin', 'database_lifecycle', 'payload_and_protection', 'smart_bins',
  ]],
  ['history_and_organization_transfer', [
    'analysis_lifecycle', 'archive_identity', 'transfer_roundtrip', 'validation_and_rollback',
  ]],
  ['migrations_and_intelligence', [
    'classification_runtime', 'contracts_and_smoke', 'intelligence_registry',
    'legacy_intelligence_migrations',
  ]],
  ['search_and_operations', [
    'clip_lifecycle', 'operations_and_connections', 'search_indexing',
    'taxonomy_and_ordering',
  ]],
  ['transforms_backup_and_protection', ['collections_and_protection', 'transforms']],
]) {
  const databaseTestFacade = read(`src-tauri/src/db/tests/${facade}.rs`);
  for (const owner of owners) {
    assert.match(databaseTestFacade, new RegExp(`mod ${owner};`),
      `${facade} tests must compose the focused ${owner} owner`);
  }
  assert.doesNotMatch(databaseTestFacade, /#\[test\]|#\[ignore\]|\bfn\s+/,
    `${facade} must remain a declaration-only test facade`);
}

const sizeRatchets = new Map([
  ['src-tauri/src/lib.rs', 400],
  ['src-tauri/src/app_runtime.rs', 180],
  ['src-tauri/src/app_tray.rs', 190],
  ['src-tauri/src/app_windows.rs', 165],
  ['src-tauri/src/clipboard_capture_policy.rs', 380],
  ['src-tauri/src/clipboard_monitor.rs', 325],
  ['src-tauri/src/clipboard_ingestion/mod.rs', 175],
  ['src-tauri/src/clipboard_ingestion/files.rs', 115],
  ['src-tauri/src/clipboard_ingestion/text.rs', 100],
  ['src-tauri/src/clipboard_ingestion/image.rs', 145],
  ['src-tauri/src/db.rs', 210],
  ['src-tauri/src/content_analysis.rs', 186],
  ['src-tauri/src/content_analysis/pipeline.rs', 413],
  ['src-tauri/src/content_analysis/pipeline/file_extraction.rs', 35],
  ['src-tauri/src/content_analysis/pipeline/file_inspection.rs', 26],
  ['src-tauri/src/content_analysis/tests.rs', 9],
  ['src-tauri/src/content_analysis/tests/fixtures.rs', 123],
  ['src-tauri/src/content_analysis/tests/scheduler_policy.rs', 170],
  ['src-tauri/src/content_analysis/tests/extraction_classification.rs', 165],
  ['src-tauri/src/content_analysis/tests/failure_contracts.rs', 90],
  ['src-tauri/src/content_extraction.rs', 785],
  ['src-tauri/src/content_extraction/engine_runtime.rs', 70],
  ['src-tauri/src/content_extraction/engine_runtime/apple_vision.rs', 220],
  ['src-tauri/src/content_extraction/engine_runtime/custom_command.rs', 220],
  ['src-tauri/src/content_extraction/engine_runtime/discovery.rs', 100],
  ['src-tauri/src/content_extraction/engine_runtime/tesseract.rs', 165],
  ['src-tauri/src/content_extraction/engine_runtime/whisper.rs', 425],
  ['src-tauri/src/content_extraction/file_routing.rs', 30],
  ['src-tauri/src/content_extraction/format_defaults.rs', 34],
  ['src-tauri/src/content_extraction/preset_tests.rs', 30],
  ['src-tauri/src/content_extraction/outcome.rs', 20],
  ['src-tauri/src/intelligence_executor.rs', 60],
  ['src-tauri/src/intelligence_executor/connections.rs', 85],
  ['src-tauri/src/intelligence_executor/execution.rs', 325],
  ['src-tauri/src/intelligence_executor/extractor_authoring.rs', 240],
  ['src-tauri/src/intelligence_executor/extractor_repair.rs', 200],
  ['src-tauri/src/intelligence_executor/extractor_repair/prompt.rs', 55],
  ['src-tauri/src/intelligence_executor/extractor_repair/setup_guidance.rs', 75],
  ['src-tauri/src/intelligence_executor/extractor_repair/setup_guidance/actions.rs', 40],
  ['src-tauri/src/intelligence_executor/extractor_repair/setup_guidance/matching.rs', 30],
  ['src-tauri/src/intelligence_executor/extractor_repair/setup_guidance/tests.rs', 40],
  ['src-tauri/src/intelligence_executor/extractor_repair/tests.rs', 20],
  ['src-tauri/src/intelligence_executor/planning.rs', 205],
  ['src-tauri/src/intelligence_executor/saved_transforms.rs', 175],
  ['src-tauri/src/intelligence_executor/tests.rs', 455],
  ['src-tauri/src/commands/intelligence/connections.rs', 140],
  ['src-tauri/src/commands/intelligence/extractor_repair.rs', 40],
  ['src-tauri/src/transformation_service.rs', 30],
  ['src-tauri/src/transformation_service/cancellation.rs', 85],
  ['src-tauri/src/transformation_service/compatibility.rs', 115],
  ['src-tauri/src/transformation_service/contracts.rs', 155],
  ['src-tauri/src/transformation_service/operations.rs', 180],
  ['src-tauri/src/transformation_service/orchestration.rs', 300],
  ['src-tauri/src/transformation_service/tests.rs', 450],
  ['src-tauri/src/commands/extraction/ocr_backfill.rs', 60],
  ['src-tauri/src/db/extractors/runtime.rs', 122],
  ['src-tauri/src/db/tests/extractor_recipes.rs', 105],
  ['src-tauri/src/db/tests/analytics.rs', 105],
  ['src-tauri/src/db/tests/analytics_boundaries.rs', 70],
  ['src-tauri/src/content_extraction/tests.rs', 458],
  ['src-tauri/src/db/activity.rs', 649],
  ['src-tauri/src/db/analysis_activity.rs', 71],
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
  ['src-tauri/src/db/contracts.rs', 300],
  ['src-tauri/src/db/extractors.rs', 802],
  ['src-tauri/src/db/full_backups.rs', 277],
  ['src-tauri/src/db/intelligence_connections.rs', 231],
  ['src-tauri/src/db/intelligence_connections/reset.rs', 75],
  ['src-tauri/src/db/lifecycle.rs', 233],
  ['src-tauri/src/db/maintenance.rs', 80],
  ['src-tauri/src/db/operations.rs', 377],
  ['src-tauri/src/db/retention.rs', 293],
  ['src-tauri/src/settings_contract.rs', 260],
  ['src-tauri/src/settings_contract/tests.rs', 100],
  ['src-tauri/src/cli/commands/settings.rs', 200],
  ['src-tauri/src/cli/commands/settings/reset_preview.rs', 120],
  ['src-tauri/src/db/schema.rs', 35],
  ['src-tauri/src/db/schema/canonical.rs', 60],
  ['src-tauri/src/db/schema/canonical/clips.rs', 230],
  ['src-tauri/src/db/schema/canonical/analysis_history.rs', 65],
  ['src-tauri/src/db/schema/canonical/content_compatibility.rs', 70],
  ['src-tauri/src/db/schema/canonical/content_registry.rs', 205],
  ['src-tauri/src/db/schema/canonical/extractors.rs', 265],
  ['src-tauri/src/db/schema/canonical/organization.rs', 175],
  ['src-tauri/src/db/schema/helpers.rs', 65],
  ['src-tauri/src/db/schema/helpers/tests.rs', 30],
  ['src-tauri/src/db/schema/library_items.rs', 300],
  ['src-tauri/src/db/schema/registry.rs', 65],
  ['src-tauri/src/db/schema/registry/tests.rs', 50],
  ['src-tauri/src/db/schema/transformation_tables.rs', 340],
  ['src-tauri/src/db/schema/migrations/analysis.rs', 220],
  ['src-tauri/src/db/schema/migrations/core.rs', 85],
  ['src-tauri/src/db/schema/migrations/settings.rs', 55],
  ['src-tauri/src/db/schema/migrations/transforms.rs', 415],
  ['src-tauri/src/db/source_queries.rs', 16],
  ['src-tauri/src/db/stored_analysis.rs', 15],
  ['src-tauri/src/db/stored_analysis/classifications.rs', 130],
  ['src-tauri/src/db/stored_analysis/classifications/tests.rs', 75],
  ['src-tauri/src/db/stored_analysis/extractions.rs', 150],
  ['src-tauri/src/db/stored_analysis/attempts.rs', 150],
  ['src-tauri/src/db/stored_analysis/attempt_writes.rs', 80],
  ['src-tauri/src/db/stored_analysis/types.rs', 80],
  ['src-tauri/src/analysis_attempt_policy.rs', 165],
  ['src-tauri/src/extraction_reuse.rs', 165],
  ['src-tauri/src/db/stored_analysis/extractions/tests.rs', 165],
  ['src-tauri/src/db/stored_analysis/inspections.rs', 150],
  ['src-tauri/src/db/stored_analysis/ocr.rs', 340],
  ['src-tauri/src/db/stored_analysis/searchable_text.rs', 115],
  ['src-tauri/src/db/timestamps.rs', 65],
  ['src-tauri/src/db/timestamps/migrations.rs', 155],
  ['src-tauri/src/db/timestamps/migrations/tests.rs', 140],
  ['src-tauri/src/db/timestamps/migrations/tests/registry.rs', 85],
  ['src-tauri/src/db/transfers.rs', 10],
  ['src-tauri/src/db/transfers/clip_transfer.rs', 395],
  ['src-tauri/src/db/transfers/library_export.rs', 155],
  ['src-tauri/src/db/transfers/library_import.rs', 510],
  ['src-tauri/src/db/transfers/library_validation.rs', 475],
  ['src-tauri/src/db/transforms.rs', 20],
  ['src-tauri/src/db/transforms/applications.rs', 195],
  ['src-tauri/src/db/transforms/definitions.rs', 210],
  ['src-tauri/src/db/transforms/executions.rs', 135],
  ['src-tauri/src/db/transforms/manual.rs', 300],
  ['src-tauri/src/db/transforms/operation_compatibility.rs', 105],
  ['src-tauri/src/db/transforms/repository.rs', 50],
  ['src-tauri/src/db/transforms/tests.rs', 175],
  ['src-tauri/src/db/transforms/tests/fixtures.rs', 40],
  ['src-tauri/src/db/transforms/types.rs', 225],
  ['src-tauri/src/db/tests/mod.rs', 54],
  ['src-tauri/src/db/tests/bins_and_transforms.rs', 5],
  ['src-tauri/src/db/tests/bins_and_transforms/bins_and_settings.rs', 115],
  ['src-tauri/src/db/tests/bins_and_transforms/legacy_transforms.rs', 480],
  ['src-tauri/src/db/tests/capture_and_lifecycle.rs', 5],
  ['src-tauri/src/db/tests/capture_and_lifecycle/capture_and_origin.rs', 245],
  ['src-tauri/src/db/tests/capture_and_lifecycle/database_lifecycle.rs', 235],
  ['src-tauri/src/db/tests/capture_and_lifecycle/payload_and_protection.rs', 70],
  ['src-tauri/src/db/tests/capture_and_lifecycle/smart_bins.rs', 230],
  ['src-tauri/src/db/tests/full_backups.rs', 280],
  ['src-tauri/src/db/tests/migrations_and_intelligence.rs', 5],
  ['src-tauri/src/db/tests/migrations_and_intelligence/classification_runtime.rs', 350],
  ['src-tauri/src/db/tests/migrations_and_intelligence/contracts_and_smoke.rs', 100],
  ['src-tauri/src/db/tests/migrations_and_intelligence/intelligence_registry.rs', 455],
  ['src-tauri/src/db/tests/migrations_and_intelligence/legacy_intelligence_migrations.rs', 265],
  ['src-tauri/src/db/tests/portability_boundaries.rs', 50],
  ['src-tauri/src/db/tests/retention_and_activity.rs', 366],
  ['src-tauri/src/db/tests/revisions_and_mutations.rs', 495],
  ['src-tauri/src/db/tests/search_and_operations.rs', 5],
  ['src-tauri/src/db/tests/search_and_operations/clip_lifecycle.rs', 265],
  ['src-tauri/src/db/tests/search_and_operations/operations_and_connections.rs', 370],
  ['src-tauri/src/db/tests/search_and_operations/search_indexing.rs', 380],
  ['src-tauri/src/db/tests/search_and_operations/taxonomy_and_ordering.rs', 110],
  ['src-tauri/src/db/tests/clip_transfer.rs', 200],
  ['src-tauri/src/db/tests/history_and_organization_transfer.rs', 5],
  ['src-tauri/src/db/tests/history_and_organization_transfer/analysis_lifecycle.rs', 65],
  ['src-tauri/src/db/tests/history_and_organization_transfer/archive_identity.rs', 160],
  ['src-tauri/src/db/tests/history_and_organization_transfer/transfer_roundtrip.rs', 295],
  ['src-tauri/src/db/tests/history_and_organization_transfer/validation_and_rollback.rs', 105],
  ['src-tauri/src/db/tests/timestamps.rs', 160],
  ['src-tauri/src/db/tests/transforms_backup_and_protection.rs', 5],
  ['src-tauri/src/db/tests/transforms_backup_and_protection/collections_and_protection.rs', 270],
  ['src-tauri/src/db/tests/transforms_backup_and_protection/transforms.rs', 360],
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
  ['src-tauri/src/commands/retention/analysis.rs', 16],
  ['src-tauri/src/commands/retention/revisions.rs', 16],
  ['src-tauri/src/commands/settings.rs', 121],
  ['src-tauri/src/commands/backups.rs', 180],
  ['src-tauri/src/commands/imports.rs', 287],
  ['src-tauri/src/commands/factory_reset.rs', 39],
  ['src-tauri/src/commands/extraction.rs', 187],
  ['src-tauri/src/commands/clips.rs', 261],
  ['src-tauri/src/commands/file_previews.rs', 623],
  ['src-tauri/src/commands/file_preview_cache.rs', 191],
  ['src-tauri/src/file_reference_health.rs', 288],
  ['src-tauri/src/commands/intelligence.rs', 271],
  ['src-tauri/src/commands/library_access.rs', 38],
  ['src-tauri/src/commands/manual_transforms.rs', 164],
  ['src-tauri/src/commands/source_apps.rs', 463],
  ['src-tauri/src/commands/transformations.rs', 244],
  ['src-tauri/src/commands/analysis.rs', 100],
  ['src-tauri/src/commands/content_registry.rs', 260],
  ['src-tauri/src/commands/extractors.rs', 180],
  ['src-tauri/src/extractor_recipe/diagnostics.rs', 170],
  ['src-tauri/src/extractor_recipe/diagnostics/invalid_tests.rs', 20],
  ['src-tauri/src/extractor_recipe/diagnostics/tests.rs', 40],
  ['src-tauri/src/cli/commands/extractors.rs', 400],
  ['src-tauri/src/cli/commands/extractors/diagnostics.rs', 75],
  ['src-tauri/src/commands/app_lock.rs', 322],
  ['src-tauri/src/commands/app_lock/tests.rs', 40],
  ['src-tauri/src/app_lock.rs', 490],
  ['src-tauri/src/app_lock/platform_auth.rs', 245],
  ['src-tauri/src/paste_target.rs', 285],
  ['src-tauri/src/paste_target/platform/mod.rs', 95],
  ['src-tauri/src/paste_target/platform/macos.rs', 180],
  ['src-tauri/src/paste_target/platform/windows.rs', 175],
  ['src-tauri/src/paste_target/platform/linux.rs', 120],
  ['src-tauri/src/private_browsing.rs', 240],
  ['src-tauri/src/cli/commands/private_browsing.rs', 110],
  ['src-tauri/src/commands/queue.rs', 160],
  ['src-tauri/src/commands/storage.rs', 170],
  ['src-tauri/src/bin/pasted.rs', 320],
  ['src-tauri/src/cli/help.rs', 100],
  ['src-tauri/tests/cli_integration.rs', 10],
  ['src-tauri/tests/cli_integration/mod.rs', 10],
  ['src-tauri/tests/cli_integration/support.rs', 100],
  ['src-tauri/tests/cli_integration/analysis.rs', 360],
  ['src-tauri/tests/cli_integration/extractor_diagnostics.rs', 30],
  ['src-tauri/tests/cli_integration/app_lock_policy.rs', 100],
  ['src-tauri/tests/cli_integration/portability_library.rs', 280],
  ['src-tauri/tests/cli_integration/settings_page_reset.rs', 90],
  ['src-tauri/tests/cli_integration/settings_reset_dry_run.rs', 60],
  ['src-tauri/tests/cli_integration/lifecycle_policy.rs', 235],
  ['src-tauri/tests/cli_integration/registry_authoring.rs', 300],
  ['src/App.tsx', 16],
  ['src/hud-main.tsx', 36],
  ['src/capture-feedback-main.tsx', 36],
  ['src/hooks/useAuxiliaryWindowReady.ts', 20],
  ['src/hooks/useAuxiliaryAppSettings.ts', 70],
  ['src/appSettingsModel.ts', 145],
  ['src/appSettingsCapturePolicyModel.ts', 30],
  ['src/appSettingsSectionDefaults.ts', 35],
  ['src/appExclusionModel.ts', 50],
  ['src/utils/appTheme.ts', 28],
  ['src/components/ClipImageThumbnail.tsx', 76],
  ['src/hooks/useAppController.ts', 499],
  ['src/hooks/useAppLibraryActions.ts', 59],
  ['src/hooks/appControllerModel.ts', 22],
  ['src/components/AppShellView.tsx', 467],
  ['src/components/AppDestinationView.tsx', 79],
  ['src/components/SettingsBlacklistPanel.tsx', 250],
  ['src/components/PrivateBrowserExclusionSection.tsx', 115],
  ['src/components/SettingsNotificationsPanel.tsx', 145],
  ['src/components/SettingsSecurityPanel.tsx', 285],
  ['src/components/SettingsPanelResetNote.tsx', 35],
  ['src/components/SettingsResetChanges.tsx', 65],
  ['src/components/SettingsHotkeysPanel.tsx', 410],
  ['src/hotkeySettingsModel.ts', 75],
  ['src/appLockResetChanges.ts', 45],
  ['src/appExclusionResetChanges.ts', 70],
  ['src/notificationSettingsModel.ts', 50],
  ['src/analysisResetChanges.ts', 70],
  ['src/hooks/useAnalysisReset.ts', 65],
  ['src/generalSettingsDefaults.ts', 30],
  ['src/settingsContract.ts', 50],
  ['src/generalSettingsResetChanges.ts', 115],
  ['src/hooks/useGeneralSettingsReset.ts', 45],
  ['src/components/SettingsGeneralResetFooter.tsx', 30],
  ['src/intelligenceResetChanges.ts', 50],
  ['src/components/IntelligenceConnectionsPanel.tsx', 270],
  ['src/hooks/useAppNavigation.ts', 175],
  ['src/hooks/useAppShell.ts', 130],
  ['src/hooks/useAppMenuActions.ts', 120],
  ['src/hooks/useClipSelectionController.ts', 230],
  ['src/hooks/useClipListViewport.ts', 195],
  ['src/hooks/useAppOverlays.ts', 135],
  ['src/hooks/useClipDragController.ts', 130],
  ['src/hooks/useClipReordering.ts', 100],
  ['src/components/AppDialogLayer.tsx', 210],
  ['src/components/ClipPreview.tsx', 410],
  ['src/components/ClipPreviewContent.tsx', 425],
  ['src/components/ClipPreviewEmptyState.tsx', 15],
  ['src/components/FileClipPreviewPanel.tsx', 186],
  ['src/components/FileReferenceFooter.tsx', 48],
  ['src/components/ClipPreviewHeader.tsx', 250],
  ['src/components/ClipPreviewOrganization.tsx', 93],
  ['src/components/ExtractorAiAuthoringPanel.tsx', 65],
  ['src/components/ExtractorAiSetupPanel.tsx', 75],
  ['src/extractorAiAuthoring.ts', 35],
  ['src/hooks/useExtractorAiAuthoring.ts', 115],
  ['src/components/extractorFileFormats.ts', 20],
  ['src/components/ocrStatusModel.ts', 25],
  ['src/components/menuMultiSelectModel.ts', 30],
  ['src-tauri/src/extractor_recipe/local_configuration.rs', 45],
  ['src-tauri/src/extractor_recipe/local_configuration/tests.rs', 25],
  ['src/components/ClipPreviewTransformControls.tsx', 159],
  ['src/components/ClipPreviewWorkspace.tsx', 84],
  ['src/components/SettingsSyncPanel.tsx', 456],
  ['src/components/SettingsSyncLibrarySection.tsx', 66],
  ['src/components/SettingsSyncExportSection.tsx', 129],
  ['src/components/SettingsSyncImportSection.tsx', 124],
  ['src/components/settingsSyncModel.ts', 62],
  ['src/components/SettingsGeneralPanel.tsx', 410],
  ['src/components/SettingsGeneralHistoryLimits.tsx', 90],
  ['src/components/SettingsGeneralAppearanceSection.tsx', 82],
  ['src/components/SettingsGeneralLayoutSection.tsx', 83],
  ['src/components/SettingsGeneralRetentionSections.tsx', 125],
  ['src/components/ActivityLogView.tsx', 269],
  ['src/components/ActivityEventBadge.tsx', 431],
  ['src/hooks/useClipActions.ts', 316],
  ['src/hooks/useClipPropertyActions.ts', 240],
  ['src/hooks/useClipBinActions.ts', 201],
  ['src/components/ClipCard.tsx', 171],
  ['src/components/ClipCardActions.tsx', 194],
  ['src/components/ClipCardContent.tsx', 76],
  ['src/components/ClipCardHeader.tsx', 143],
  ['src/components/ClipCardThumbnails.tsx', 175],
  ['src/components/HighlightedClipText.tsx', 26],
  ['src/components/clipCardModel.ts', 109],
  ['src/hooks/useClipCardPointerDrag.ts', 130],
  ['src/components/ClipPreviewFooter.tsx', 118],
  ['src/components/ClipPreviewNotesPanel.tsx', 87],
  ['src/components/Sidebar.tsx', 303],
  ['src/components/SidebarClipSection.tsx', 100],
  ['src/components/SidebarFacetSections.tsx', 93],
  ['src/components/SidebarToolsSection.tsx', 48],
  ['src/components/sidebarNavigationModel.tsx', 82],
  ['src/components/CollapsedSidebar.tsx', 165],
  ['src/components/SidebarBinsSection.tsx', 206],
  ['src/components/SidebarSearchFooter.tsx', 224],
  ['src/components/BinModal.tsx', 130],
  ['src/components/BinModalBehaviorFields.tsx', 120],
  ['src/components/BinModalIdentityFields.tsx', 170],
  ['src/components/BinModalSmartRules.tsx', 184],
  ['src/components/BinModalSmartConditionInputs.tsx', 219],
  ['src/components/binModalEmoji.ts', 20],
  ['src/components/binModalModel.ts', 110],
  ['src/components/binModalTargets.ts', 109],
  ['src/components/HelpView.tsx', 385],
  ['src/components/HelpCliTopic.tsx', 115],
  ['src/components/helpCliCatalog.ts', 159],
  ['src/components/helpCliAnalysisCatalog.ts', 50],
  ['src/components/ManualTransformEditorModal.tsx', 218],
  ['src/components/ManualTransformStepEditor.tsx', 168],
  ['src/components/manualTransformStepModel.ts', 100],
  ['src/components/CaptureFeedbackWindow.tsx', 472],
  ['src/components/CaptureFeedbackCard.tsx', 137],
  ['src/components/captureFeedbackModel.ts', 42],
  ['src/types.ts', 495],
  ['src/appSettingsTypes.ts', 102],
  ['src/appSettingsRetentionModel.ts', 20],
  ['src/appSettingsTypes/retention.ts', 15],
  ['src/components/clipPreviewModel.ts', 162],
  ['src/components/clipExtractionModel.ts', 24],
  ['src/components/fileClipPreviewLoader.ts', 53],
  ['src/components/fileClipPreviewModel.ts', 9],
  ['src/hooks/useClipPreviewAnalysis.ts', 299],
  ['src/hooks/useFileClipPreviews.ts', 60],
  ['src/hooks/useClipPreviewNotes.ts', 129],
  ['src/hooks/useClipPreviewRevisions.ts', 149],
  ['src/hooks/useClipPreviewTransforms.ts', 289],
  ['src/hooks/useSidebarHoverState.ts', 67],
  ['src/hooks/useSidebarFacets.ts', 83],
  ['src/hooks/useBinModalForm.ts', 204],
  ['src/utils/tauri.ts', 10],
  ['src/mocks/browser/runtime.ts', 30],
  ['src/mocks/browser/contentRuntime.ts', 372],
  ['src/mocks/browser/extractors.ts', 60],
  ['src/mocks/browser/intelligenceRuntime.ts', 243],
  ['src/mocks/browser/intelligenceDetectedConnections.ts', 10],
  ['src/mocks/browser/libraryRuntime.ts', 568],
  ['src/mocks/browser/systemRuntime.ts', 183],
  ['src/mocks/browser/retentionRuntime.ts', 20],
]);
for (const [path, maximum] of sizeRatchets) {
  assert.ok(lineCount(path) <= maximum,
    `${path} grew beyond its ${maximum}-line architecture ratchet; extract a capability instead`);
}

const settingsSyncShell = read('src/components/SettingsSyncPanel.tsx');
for (const section of ['SettingsSyncLibrarySection', 'SettingsSyncExportSection', 'SettingsSyncImportSection']) {
  assert.match(settingsSyncShell, new RegExp(`<${section}`),
    `Storage must compose its focused ${section} view`);
}
assert.doesNotMatch(settingsSyncShell,
  /library-location-title|export-title|import-title|SettingsSwitch|checkingFile|supportedFormatsValue/,
  'The Storage coordinator must not reclaim section-owned presentation');
const settingsSyncViews = [
  read('src/components/SettingsSyncLibrarySection.tsx'),
  read('src/components/SettingsSyncExportSection.tsx'),
  read('src/components/SettingsSyncImportSection.tsx'),
].join('\n');
assert.doesNotMatch(settingsSyncViews,
  /safeInvoke|backupApi|activityApi|collectBackupClientState|restoreFull|importInspected|move_library/,
  'Storage presentation sections must remain independent from persistence and native commands');

const settingsGeneralShell = read('src/components/SettingsGeneralPanel.tsx');
for (const section of ['SettingsGeneralAppearanceSection', 'SettingsGeneralLayoutSection', 'SettingsGeneralRetentionSections']) {
  assert.match(settingsGeneralShell, new RegExp(`<${section}`),
    `General Settings must compose its focused ${section} view`);
}
assert.doesNotMatch(settingsGeneralShell,
  /appearanceModes|appearanceGroups|APP_ZOOM_STEPS|applicationZoom|keepTrashedClipsFor|keepActivityFor/,
  'The General Settings coordinator must not reclaim extracted presentation');
const settingsGeneralPassiveViews = [
  read('src/components/SettingsGeneralAppearanceSection.tsx'),
  read('src/components/SettingsGeneralRetentionSections.tsx'),
].join('\n');
assert.doesNotMatch(settingsGeneralPassiveViews,
  /safeInvoke|useToast|localStorage|window\.location|onRestoreAllTrashedClips/,
  'General Settings presentation views must communicate mutations through callbacks');

const activityView = read('src/components/ActivityLogView.tsx');
const activityBadge = read('src/components/ActivityEventBadge.tsx');
assert.match(activityView, /<ActivityEventBadge type=\{log\.event_type\} description=\{log\.description\} \/>/,
  'Activity must delegate event presentation to its badge system');
assert.doesNotMatch(activityView, /case 'recording_manually_paused'|case 'clip_trashed'|case 'transform_executed'/,
  'The Activity lifecycle view must not reclaim event badge mapping');
assert.doesNotMatch(activityBadge, /activityApi|useEffect|useState|IntersectionObserver/,
  'Activity badge presentation must remain independent from loading and lifecycle state');

const clipActionsFacade = read('src/hooks/useClipActions.ts');
for (const controller of ['useClipPropertyActions', 'useClipBinActions']) {
  assert.match(clipActionsFacade, new RegExp(`${controller}\\(`),
    `Clip Actions must compose the ${controller} controller`);
}
assert.doesNotMatch(clipActionsFacade,
  /clipsApi\.setPinned|clipsApi\.setProtected|clipsApi\.setConcealed|clipsApi\.assignManyToBin|clipsApi\.removeBin/,
  'The Clip Actions facade must not reclaim property or Bin persistence');
const clipPropertyActions = read('src/hooks/useClipPropertyActions.ts');
const clipBinActions = read('src/hooks/useClipBinActions.ts');
assert.doesNotMatch(clipPropertyActions, /assignManyToBin|assignBin|removeBin/,
  'Clip property mutations must remain separate from Bin membership');
assert.doesNotMatch(clipBinActions, /setPinned|setProtected|setConcealed|toggle_clip_protected/,
  'Clip Bin mutations must remain separate from direct property mutations');

const clipPreviewShell = read('src/components/ClipPreview.tsx');
for (const controller of ['useClipPreviewAnalysis', 'useClipPreviewNotes', 'useClipPreviewTransforms']) {
  assert.match(clipPreviewShell, new RegExp(`${controller}\\(`),
    `Clip Preview must compose the ${controller} controller`);
}
assert.doesNotMatch(clipPreviewShell,
  /get_clip_versions|get_clip_extraction_results|update_clip_note|get_clip_content_matches|startTransformation|apply_transform_preview_to_clip/,
  'Clip Preview must not reclaim controller-owned persistence, analysis, or Transform commands');
const clipPreviewTransforms = read('src/hooks/useClipPreviewTransforms.ts');
assert.match(clipPreviewTransforms, /useClipPreviewRevisions\(\{/,
  'The Transform controller must coordinate revision preview and restore state');
assert.match(clipPreviewTransforms, /requestIdRef\.current \+= 1;[\s\S]{0,100}activeExecutionRef\.current\?\.cancel\(\)/,
  'Unmounting the Transform controller must invalidate stale responses and cancel active work');
assert.match(clipPreviewTransforms, /if \(requestId !== requestIdRef\.current\) return;/,
  'Transform responses must be rejected after a newer request or reset');
assert.match(clipPreviewTransforms, /const invalidateActiveExecution[\s\S]{0,100}requestIdRef\.current \+= 1;[\s\S]{0,100}activeExecutionRef\.current\?\.cancel\(\)/,
  'The Transform controller must centralize stale-response invalidation and cancellation');
assert.match(clipPreviewTransforms, /const resetTransform = \(\) => \{[\s\S]{0,80}invalidateActiveExecution\(\);/,
  'Resetting a Transform preview must invalidate and cancel active work');
assert.match(clipPreviewTransforms, /if \(canRunManualTransforms\) return;[\s\S]{0,100}invalidateActiveExecution\(\);/,
  'Losing Transform permission must cancel active work and invalidate its response');
assert.match(clipPreviewTransforms, /onBeforeRestore:[\s\S]{0,100}invalidateActiveExecution\(\);/,
  'Restoring a revision must cancel active Transform work before replacing preview state');
for (const subsystem of [
  'ClipPreviewHeader',
  'ClipPreviewOrganization',
  'ClipPreviewTransformControls',
  'ClipPreviewWorkspace',
]) {
  assert.match(clipPreviewShell, new RegExp(`<${subsystem}`),
    `Clip Preview must compose the ${subsystem} presentation subsystem`);
}
assert.doesNotMatch(clipPreviewShell, /startWindowDrag|ClipBinPicker|MenuSelect|smart-actions-bar/,
  'Clip Preview must not reclaim extracted header, organization, or Transform presentation internals');
assert.doesNotMatch(clipPreviewShell, /useMinuteTick/,
  'Clip Preview must not rerender its full controller tree for relative-time updates');
for (const timestampSurface of ['ClipPreviewWorkspace.tsx', 'ClipPreviewFooter.tsx']) {
  assert.match(read(`src/components/${timestampSurface}`), /useMinuteTick\(\)/,
    `${timestampSurface} must own its scoped relative-time refresh`);
}

const clipCardShell = read('src/components/ClipCard.tsx');
for (const subsystem of ['ClipCardActions', 'ClipCardContent', 'ClipCardHeader', 'useClipCardPointerDrag']) {
  assert.match(clipCardShell, new RegExp(`${subsystem}`),
    `ClipCard must compose the ${subsystem} subsystem`);
}
assert.doesNotMatch(clipCardShell, /get_clip_image|get_file_clip_previews|addEventListener\('pointermove'/,
  'ClipCard must not reclaim preview loading or pointer-drag internals');
assert.doesNotMatch(clipCardShell, /useMinuteTick/,
  'ClipCard must not rerender its full content and action tree for relative-time updates');
assert.match(read('src/components/ClipCardHeader.tsx'), /useMinuteTick\(\)/,
  'Relative-time updates must remain scoped to ClipCard metadata');

const sidebarShell = read('src/components/Sidebar.tsx');
for (const subsystem of [
  'CollapsedSidebar',
  'SidebarBinsSection',
  'SidebarClipSection',
  'SidebarFacetSections',
  'SidebarSearchFooter',
  'SidebarToolsSection',
  'useSidebarFacets',
  'useSidebarHoverState',
]) {
  assert.match(sidebarShell, new RegExp(`${subsystem}`),
    `Sidebar must compose the ${subsystem} subsystem`);
}
assert.doesNotMatch(sidebarShell, /get_clip_extraction_history|SEARCH_HELPERS|data-stable-reorder-id/,
  'Sidebar must not reclaim search or Bin interaction internals');
assert.doesNotMatch(sidebarShell, /get_source_icons|sourceIconsRef|ContentTypeIcon|SafeRasterImage/,
  'Sidebar must not reclaim facet loading or presentation internals');
assert.match(read('src/hooks/useSidebarFacets.ts'), /get_source_icons/,
  'Sidebar facet data must own asynchronous source-icon loading');
assert.match(read('src/components/SidebarClipSection.tsx'), /data-clip-drop-action/,
  'Sidebar clip navigation must preserve its drop-action contract');

const binModalShell = read('src/components/BinModal.tsx');
assert.match(binModalShell, /useBinModalForm\(/,
  'BinModal must delegate form lifecycle and persistence to its controller');
for (const subsystem of [
  'BinModalBehaviorFields',
  'BinModalIdentityFields',
  'BinModalSmartRules',
  'buildBinModalTargets',
]) {
  assert.match(binModalShell, new RegExp(`${subsystem}`),
    `BinModal must compose the ${subsystem} subsystem`);
}
assert.doesNotMatch(binModalShell, /binsApi\.|get_bin_transform_ref|normalizeSmartCondition/,
  'BinModal must not reclaim controller-owned persistence and rule normalization');
assert.doesNotMatch(binModalShell, /open_emoji_picker|SmartConditionTargetSelect|SmartConditionValueInput/,
  'BinModal must not reclaim identity or Smart Bin rule-editor internals');
assert.match(read('src/components/BinModalSmartRules.tsx'),
  /SmartConditionTargetSelect[\s\S]*SmartConditionValueInput/,
  'Smart Bin rules must compose the shared condition controls');
assert.match(read('src/components/BinModalIdentityFields.tsx'), /open_emoji_picker/,
  'Bin identity fields must retain the native emoji-picker integration');
assert.match(read('src/components/BinModalIdentityFields.tsx'),
  /ref=\{nativeEmojiTriggerRef\}[\s\S]*desktopPlatform === 'macos' \? nativeEmojiTriggerRef : emojiTriggerRef/,
  'The in-app emoji fallback must anchor to the visible platform-specific trigger');

const helpViewShell = read('src/components/HelpView.tsx');
assert.match(helpViewShell, /<HelpCliTopic/,
  'Help must compose the focused CLI topic presentation');
assert.doesNotMatch(helpViewShell, /pasted classifier create|pasted activity import|pasted bin create/,
  'Help must not reclaim the CLI command catalog');
assert.doesNotMatch(helpViewShell, /symlinkInUsrLocalBin|commandReferenceDescription/,
  'Help must not reclaim CLI installation or command-reference presentation');
assert.match(read('src/components/HelpCliTopic.tsx'), /helpCliCatalog/,
  'The Help CLI topic must compose its command catalog');
assert.match(read('src/components/helpCliCatalog.ts'), /CLI_ANALYSIS_COMMAND_GROUP[\s\S]*pasted activity import/,
  'The Help CLI catalog must compose analysis and retain Activity command coverage');
assert.match(read('src/components/helpCliAnalysisCatalog.ts'), /pasted analyzer run[\s\S]*pasted classifier create/,
  'The focused Help analysis catalog must retain Analyzer and Classifier command coverage');

const manualTransformShell = read('src/components/ManualTransformEditorModal.tsx');
assert.match(manualTransformShell, /<ManualTransformStepEditor/,
  'The Manual Transform editor must compose its focused step editor');
assert.doesNotMatch(manualTransformShell, /OPERATION_CATEGORIES|findPattern:[\s\S]*replacePattern:/,
  'The Manual Transform shell must not reclaim operation presentation or step serialization');
assert.match(read('src/components/manualTransformStepModel.ts'), /compileManualTransformStep/,
  'Manual Transform step serialization must remain a testable domain boundary');

const captureFeedbackShell = read('src/components/CaptureFeedbackWindow.tsx');
assert.match(captureFeedbackShell, /<CaptureFeedbackCard/,
  'Capture Feedback must compose its focused card presentation');
assert.doesNotMatch(captureFeedbackShell, /SafeRasterImage|FloatingActionStrip|capturedClipPreview/,
  'Capture Feedback window lifecycle must not reclaim card presentation');

const contentAnalysisFacade = read('src-tauri/src/content_analysis.rs');
assert.match(contentAnalysisFacade, /mod pipeline;/,
  'Content Analysis must keep scheduler participants behind its pipeline module');
assert.doesNotMatch(contentAnalysisFacade, /fn schedule\(|fn extractor_participant\(/,
  'Content Analysis contracts must not reclaim scheduler implementation');
const contentExtractionContracts = read('src-tauri/src/content_extraction.rs');
assert.match(contentExtractionContracts, /mod engine_runtime;/,
  'Content Extraction must keep operating-system engines behind its runtime module');
assert.doesNotMatch(contentExtractionContracts, /perform_tesseract_ocr|perform_whisper_cpp_transcription|execute_custom_command/,
  'Content Extraction definitions must not reclaim engine process execution');
const extractionEngineRuntime = read('src-tauri/src/content_extraction/engine_runtime.rs');
for (const adapter of ['apple_vision', 'custom_command', 'discovery', 'tesseract', 'whisper']) {
  assert.match(extractionEngineRuntime, new RegExp(`mod ${adapter};`),
    `The extraction engine registry must compose the ${adapter} adapter`);
}
assert.doesNotMatch(extractionEngineRuntime, /Command::new|fn perform_apple_vision_ocr|fn execute_custom_command/,
  'The extraction engine registry must not reclaim adapter execution');
assert.match(read('src-tauri/src/content_extraction/engine_runtime/apple_vision.rs'), /impl ExtractorEngine for AppleVisionOcrEngine/,
  'Apple Vision extraction must remain in its focused adapter');
assert.match(read('src-tauri/src/content_extraction/engine_runtime/tesseract.rs'), /impl ExtractorEngine for TesseractOcrEngine/,
  'Tesseract extraction must remain in its focused adapter');
assert.match(read('src-tauri/src/content_extraction/engine_runtime/whisper.rs'), /impl ExtractorEngine for WhisperCppEngine/,
  'Whisper extraction must remain in its focused adapter');
assert.match(read('src-tauri/src/content_extraction/engine_runtime/custom_command.rs'), /impl ExtractorEngine for CustomCommandEngine/,
  'Custom command extraction must remain in its focused adapter');
const intelligenceExecutorFacade = read('src-tauri/src/intelligence_executor.rs');
for (const capability of ['connections', 'execution', 'extractor_authoring', 'extractor_repair', 'planning', 'saved_transforms']) {
  assert.match(intelligenceExecutorFacade, new RegExp(`mod ${capability};`),
    `The intelligence executor must compose the ${capability} capability`);
}
assert.doesNotMatch(intelligenceExecutorFacade, /fn planning_prompt|fn execute_plan_steps|fn extractor_recipe_schema/,
  'The intelligence executor facade must not reclaim planning or execution implementation');
assert.match(read('src-tauri/src/intelligence_executor/extractor_authoring.rs'), /pub fn propose_extractor_recipe/,
  'Extractor recipe authoring must remain in its focused capability');
assert.match(read('src-tauri/src/intelligence_executor/planning.rs'), /pub fn plan_intent/,
  'Transform planning must remain in its focused capability');
assert.match(read('src-tauri/src/intelligence_executor/execution.rs'), /pub fn execute_plan/,
  'Transform plan execution must remain in its focused capability');
assert.match(read('src-tauri/src/intelligence_executor/saved_transforms.rs'), /pub fn execute_saved_transform/,
  'Saved and Smart Bin Transform execution must remain in its focused capability');
const transformationServiceFacade = read('src-tauri/src/transformation_service.rs');
for (const capability of ['cancellation', 'compatibility', 'contracts', 'operations', 'orchestration']) {
  assert.match(transformationServiceFacade, new RegExp(`mod ${capability};`),
    `The Transformation service must compose the ${capability} capability`);
}
assert.doesNotMatch(transformationServiceFacade, /fn execute_custom_operation|fn execute_direct_operation|static EXECUTION_CANCELLATIONS/,
  'The Transformation service facade must not reclaim cancellation or execution implementation');
assert.match(read('src-tauri/src/transformation_service/cancellation.rs'), /pub struct CancellationRegistration/,
  'Transformation cancellation registration must remain in its focused capability');
assert.match(read('src-tauri/src/transformation_service/operations.rs'), /fn execute_custom_operation/,
  'Custom Operation execution must remain in the shared Operation capability');
assert.match(read('src-tauri/src/transformation_service/orchestration.rs'), /pub fn execute_with_cancellation/,
  'Transform execution orchestration must remain in its focused capability');
assert.match(read('src-tauri/src/transformation_service/compatibility.rs'), /pub fn execute_shortcut_manual_transform/,
  'Shortcut and last-Transform entry points must remain compatibility adapters');

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
