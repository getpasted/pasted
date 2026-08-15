import assert from 'node:assert/strict';
import fs from 'node:fs';

const read = (path) => fs.readFileSync(path, 'utf8');
const cli = read('src-tauri/src/bin/pasted_cli.rs');
const help = read('src/components/HelpView.tsx');
const database = read('src-tauri/src/db.rs');
const commands = read('src-tauri/src/commands.rs');
const analysis = read('src-tauri/src/content_analysis.rs');
const analysisContract = read('src-tauri/src/analysis_contract.rs');
const analysisExecution = read('src-tauri/src/analysis_execution.rs');
const analysisArchitecture = read('docs/ANALYSIS_ARCHITECTURE.md');
const extractionExecution = read('src-tauri/src/extraction_execution.rs');
const detectionExecution = read('src-tauri/src/detection_execution.rs');
const inspection = read('src-tauri/src/content_inspection.rs');
const inspectionExecution = read('src-tauri/src/inspection_execution.rs');
const enrichment = read('src-tauri/src/content_enrichment.rs');
const enrichmentExecution = read('src-tauri/src/enrichment_execution.rs');
const clipPreview = read('src/components/ClipPreview.tsx');
const extraction = read('src-tauri/src/content_extraction.rs');
const clipboardMonitor = read('src-tauri/src/clipboard_monitor.rs');
const ocr = read('src-tauri/src/ocr.rs');
const actions = read('src/hooks/useClipActions.ts');
const storageSettings = read('src/components/SettingsSyncPanel.tsx');
const tauriMock = read('src/utils/tauri.ts');
const extractorManager = read('src/components/ContentExtractorManagerDialog.tsx');
const detectorManager = read('src/components/SettingsDetectionPanel.tsx');

assert.match(commands, /pub async fn choose_extractor_model_file/,
  'The native Extractor model picker must not block the app command thread');

const documentedCommands = [
  'pasted copy',
  'pasted list',
  'pasted search',
  'pasted import',
  'pasted import sources',
  'pasted retention',
  'pasted settings list|get|set',
  'pasted recording status|pause|resume',
  'pasted queue status|start|stop|add|remove|order|paste|paste-all',
  'pasted activity list',
  'pasted activity export',
  'pasted activity import',
  'pasted activity clear',
  'pasted transfer export',
  'pasted transfer inspect',
  'pasted transfer import',
  'pasted analyzer run',
  'pasted database location',
  'pasted database move',
  'pasted database default',
  'pasted backup create',
  'pasted backup inspect',
  'pasted backup restore',
  'pasted clear',
  'pasted clip get',
  'pasted clip note',
  'pasted clip revisions',
  'pasted clip restore-revision',
  'pasted clip provenance',
  'pasted clip copy|paste',
  'pasted clip export',
  'pasted clip import',
  'pasted clip pin|unpin',
  'pasted clip order-pinned',
  'pasted clip protect|unprotect',
  'pasted clip trash|restore',
  'pasted clip restore-all',
  'pasted clip purge',
  'pasted clip empty-trash',
  'pasted clip assign',
  'pasted enricher list',
  'pasted enricher get',
  'pasted enricher run',
  'pasted bin list',
  'pasted bin get',
  'pasted bin create',
  'pasted bin update',
  'pasted bin duplicate',
  'pasted bin delete',
  'pasted bin clips',
  'pasted bin order',
  'pasted transform list',
  'pasted transform get',
  'pasted transform plan',
  'pasted transform test',
  'pasted transform create',
  'pasted transform update',
  'pasted transform duplicate',
  'pasted transform delete',
  'pasted transform run',
  'pasted operation list',
  'pasted operation get',
  'pasted operation create',
  'pasted operation update',
  'pasted operation duplicate',
  'pasted operation delete',
  'pasted operation run',
  'pasted connection list',
  'pasted connection get',
  'pasted connection detect',
  'pasted connection create',
  'pasted connection update',
  'pasted connection delete',
  'pasted connection order',
  'pasted insights summary',
  'pasted diagnostics',
  'pasted licenses',
  'pasted type list',
  'pasted type create',
  'pasted type archive|restore',
  'pasted type group-list',
  'pasted type group-create',
  'pasted type group-archive|group-restore',
  'pasted type group-delete',
  'pasted registry',
  'pasted inspector list',
  'pasted inspector get',
  'pasted inspector run',
  'pasted extractor list',
  'pasted extractor get',
  'pasted extractor create',
  'pasted extractor update',
  'pasted extractor duplicate',
  'pasted extractor delete',
  'pasted extractor run',
  'pasted extractor restore-defaults',
  'pasted detector list',
  'pasted detector get',
  'pasted detector create',
  'pasted detector update',
  'pasted detector duplicate',
  'pasted detector delete',
  'pasted detector run',
  'pasted detector rescan',
  'pasted ocr status',
  'pasted ocr scan',
  'pasted ocr retry',
  'pasted ocr cancel',
  'pasted reset',
];

for (const command of documentedCommands) {
  assert.ok(help.includes(command), `Help & Docs must document ${command}`);
}
assert.doesNotMatch(help, /pasted-cli/, 'Help & Docs must expose the stable pasted command, not an implementation alias');

for (const route of ['copy', 'list', 'search', 'import', 'retention', 'settings', 'recording', 'queue', 'activity', 'transfer', 'archive', 'backup', 'clear', 'clip', 'bin', 'transform', 'operation', 'connection', 'insights', 'database', 'library', 'registry', 'type', 'inspector', 'extractor', 'detector', 'enricher', 'diagnostics', 'licenses', 'ocr', 'reset']) {
  assert.match(cli, new RegExp(`"${route}"`), `The CLI must retain its ${route} route`);
}
for (const method of ['export_backup_json', 'inspect_library_archive_json', 'import_backup_json']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(cli, new RegExp(`(?:db\\.|DbState::)${method}`), `${method} must be reused by the CLI`);
}
assert.match(commands, /inspect_import_file_path[\s\S]*?inspect_library_archive_json/, 'The GUI must preflight portable transfers through the inspected-file workflow');
for (const method of ['create_full_backup', 'restore_full_backup']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(commands, new RegExp(`\\w+\\s*\\.\\s*${method}`), `${method} must be reused by the GUI`);
  assert.match(cli, new RegExp(`db\\.${method}`), `${method} must be reused by the CLI`);
}
assert.match(cli, /db\.inspect_full_backup/, 'The CLI must expose non-mutating Full Backup inspection');
assert.match(cli, /enforce_revision_retention/, 'The CLI must manage revision retention through the shared domain service');
assert.match(cli, /execute_plan/, 'The CLI must test unsaved Transform plans through the shared executor');
assert.match(cli, /reset_failed_ocr/, 'The CLI must expose failed OCR retry');
assert.match(cli, /reorder_pinned_clips/, 'The CLI must expose validated pinned ordering');
assert.match(cli, /live_app::send/, 'Running-app controls must use the bounded live-app bridge');
for (const method of ['export_activity_json', 'export_activity_csv', 'import_activity_json', 'import_activity_csv']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(cli, new RegExp(`db\\.${method}`), `${method} must be reused by the CLI`);
  if (method.startsWith('export_')) {
    assert.match(commands, new RegExp(`pub fn ${method}`), `${method} must be exposed to the GUI`);
    assert.match(storageSettings, new RegExp(`['"]${method}['"]`), `${method} must be reachable from Storage`);
  }
}
for (const method of ['export_clips_json', 'export_clips_csv', 'import_clips_json', 'import_clips_csv']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(cli, new RegExp(`db\\.${method}`), `${method} must be reused by the CLI`);
  if (method.startsWith('export_')) {
    assert.match(commands, new RegExp(`pub fn ${method}`), `${method} must be exposed to the GUI`);
  }
}
assert.match(storageSettings, /['"]import_inspected_file['"]/, 'Validated clip and Activity imports must be reachable from Storage');
for (const method of ['import_activity_json', 'import_activity_csv', 'import_clips_json', 'import_clips_csv']) {
  assert.match(commands, new RegExp(`\\w+\\s*\\.\\s*${method}`), `${method} must be reused by the inspected-import GUI command`);
}

assert.match(cli, /if matches!\(command, "licenses" \| "license"\)/, 'Legal notices must be available before database initialization');
assert.match(commands, /pub fn get_third_party_licenses/, 'The GUI must expose the shared generated license document');

for (const mutation of ['batch_pin_clips', 'batch_protect_clips', 'batch_trash_clips', 'restore_all_trashed_clips']) {
  assert.match(database, new RegExp(`pub fn ${mutation}`), `${mutation} must live in the shared database domain layer`);
  assert.match(commands, new RegExp(`pub fn ${mutation}`), `${mutation} must be exposed to the GUI`);
  assert.match(cli, new RegExp(`db\\.${mutation}`), `${mutation} must be reused by the CLI`);
}

assert.match(commands, /bin_assignment::assign_clips_to_bin/, 'GUI Bin assignment must use the shared workflow');
assert.match(cli, /assign_clips_to_bin/, 'CLI Bin assignment must use the shared workflow, including attached Transforms');

assert.match(actions, /invoke\('batch_protect_clips'/, 'GUI batch protection must be one explicit mutation, not a loop of toggles');
assert.match(read('src/App.tsx'), /invoke<ClipMutationSummary>\('restore_all_trashed_clips'\)/,
  'GUI bulk recovery must use the shared restore-all mutation');
assert.doesNotMatch(actions, /Promise\.all\(idsToChange\.map\(\(clipId\) => invoke\('toggle_clip_protected'/, 'GUI batch protection must not race toggle calls');
assert.match(database, /pub struct ClipMutationSummary/, 'GUI and CLI mutations must share a stable result contract');
assert.match(commands, /pub async fn import_external_history/, 'GUI migration must use the shared external import service');
assert.match(cli, /external_import::import_history/, 'CLI migration must use the shared external import service');
assert.match(database, /pub fn configure_clip_retention/, 'Retention policy must live in the shared database domain layer');
assert.match(commands, /db\.enforce_clip_retention/, 'GUI retention must use the shared domain policy');
assert.match(cli, /db\.configure_clip_retention/, 'CLI retention must use the shared domain policy');
for (const scope of ['trash', 'activity']) {
  assert.match(database, new RegExp(`pub fn configure_${scope}_retention`), `${scope} retention must live in the shared database domain layer`);
  assert.match(commands, new RegExp(`db\\.enforce_${scope}_retention`), `GUI ${scope} retention must use the shared domain policy`);
  assert.match(cli, new RegExp(`db\\.configure_${scope}_retention`), `CLI ${scope} retention must use the shared domain policy`);
}
assert.match(commands, /db\.rescan_content_detection\(\)/, 'GUI history rescans must use the shared detector domain service');
assert.match(cli, /db\.rescan_content_detection\(\)/, 'CLI history rescans must use the shared detector domain service');
assert.match(analysis, /fn schedule/, 'Analysis participants must share the bounded scheduler');
assert.doesNotMatch(analysis, /pub(?:\(crate\))? fn schedule/,
  'Callers must enter Analysis through typed requests rather than invoke scheduler passes directly');
assert.match(analysisContract, /pub enum RepresentationKind/,
  'Analysis representations must use the shared typed contract');
for (const contract of ['AnalysisPass', 'AnalysisPolicy', 'AnalysisTargetKind', 'ParticipantContract', 'ParticipantOutcome', 'AnalysisFailure', 'ParticipantRun', 'ClipApplication', 'AnalysisEnvelope']) {
  assert.match(analysisContract, new RegExp(`pub (?:enum|struct) ${contract}`),
    `${contract} must live in the shared Analysis contract`);
}
assert.match(analysisContract, /ANALYSIS_CONTRACT_VERSION:\s*u32\s*=\s*1/,
  'Public Analysis envelopes must expose an explicit format version');
assert.match(analysis, /pub\(crate\) fn analyze\(request: AnalysisRequest/,
  'All participants must enter the scheduler through one typed Analysis request');
assert.match(analysis, /participants\.retain\([\s\S]*?through\.includes/,
  'Analysis policies must bound participant work before scheduling');
assert.match(ocr, /AnalysisPolicy::Background/,
  'Background OCR must explicitly use the non-enriching Analysis policy');
assert.match(database, /AnalysisPolicy::Rescan/,
  'History rescans must explicitly use the non-enriching Analysis policy');
assert.match(inspection, /STRUCTURE_INSPECTOR_REF:\s*&str\s*=\s*"inspector:structure-v1"/,
  'The structural Inspector must have a stable versioned reference');
assert.match(inspection, /pub struct StructuralMetadata/,
  'Structural inspection must expose a typed content-free result');
assert.match(inspection, /MEDIA_INSPECTOR_REF:\s*&str\s*=\s*"inspector:media-metadata-v1"/,
  'The Media Metadata Inspector must have an engine-neutral stable versioned reference');
assert.match(inspection, /Command::new\(&executable\)[\s\S]*?wait_bounded\(&mut child, remaining\)/,
  'ffprobe must use direct bounded external-tool execution');
assert.match(inspection, /MAX_MEDIA_PROBE_FILES/,
  'Media inspection must bound the number of referenced files it probes');
assert.match(extraction, /"aac"\s*\|\s*"m4a"\s*=>\s*Some\(WhisperAudioPreparation::FfmpegWav\)/,
  'Whisper must route M4A and AAC audio through bounded FFmpeg preparation');
assert.match(extraction, /Command::new\(ffmpeg\)[\s\S]*?wait_bounded\(&mut child, remaining\)/,
  'Whisper audio preparation must use direct bounded FFmpeg execution');
assert.match(extraction, /MAX_TRANSCRIPTION_AUDIO_BYTES/,
  'Whisper audio preparation must bound staged audio');
assert.doesNotMatch(inspection, /pub struct MediaMetadata[\s\S]*?(?:content|path):\s*(?:String|Vec<String>)/,
  'Media metadata must not retain clipboard contents or file paths');
assert.match(inspectionExecution, /pub struct ClipInspectionResult/,
  'Focused inspection must expose one stable application result');
assert.match(inspectionExecution, /record_structural_inspection/,
  'Applied inspection must use hash-checked shared persistence');
assert.match(inspectionExecution, /content_analysis::analyze\(AnalysisRequest/,
  'Inspector scheduling must live in the shared execution boundary');
assert.doesNotMatch(inspection, /content_analysis::analyze\(/,
  'Structural metadata helpers must not schedule Analyzer work directly');
assert.match(database, /CREATE TABLE IF NOT EXISTS clip_analysis_results/,
  'Stable Inspector results must use durable clip-owned storage');
assert.match(cli, /inspection_execution::inspect_clip/,
  'CLI structural inspection must use the shared execution service');
assert.match(cli, /inspection_execution::inspect_text/,
  'CLI raw-text inspection must use the shared execution service');
assert.doesNotMatch(inspection, /pub struct StructuralMetadata[\s\S]*?(?:content|path):\s*(?:String|Vec<String>)/,
  'Durable structural metadata must not include clipboard contents or file paths');
assert.match(enrichment, /SMART_ACTIONS_ENRICHER_REF:\s*&str\s*=\s*"enricher:smart-actions-v1"/,
  'Smart Actions must have a stable versioned Enricher reference');
assert.match(enrichment, /pub struct SmartActionRecommendations/,
  'Smart Actions must expose a typed recommendation result');
assert.match(enrichmentExecution, /pub struct SmartActionEnrichmentResult/,
  'Focused enrichment must expose one stable application result');
assert.match(cli, /enrichment_execution::enrich_(?:text|clip)/,
  'CLI Smart Actions must use the shared Enricher execution service');
assert.doesNotMatch(tauriMock, /reasons:\s*signals/,
  'Mock Smart Actions must preserve per-recommendation reasons');
assert.match(tauriMock, /hasText === hasClipId/,
  'Mock Smart Actions must reject ambiguous input combinations like the native command');
assert.match(tauriMock, /\.slice\(0, 256\)[\s\S]*?\.slice\(0, 12\)/,
  'Mock Smart Actions must preserve native candidate and output bounds');
assert.match(clipPreview, /transformedText === null[\s\S]*?\{ clipId: clip\.id, includeExtractor: false \}/,
  'Clip Preview must prefer clip identity over resending stored clipboard text');
for (const surface of [database, tauriMock]) {
  assert.match(surface, /analyzable_text\+classification\+structural_metadata/,
    'Smart Actions registry surfaces must expose every declared input representation');
}
assert.doesNotMatch(clipPreview, /smartPipelineDetector|detectSmartPipelineRecommendations/,
  'Clip Preview must not maintain a parallel Smart Actions detector');
assert.doesNotMatch(enrichment, /pub struct SmartActionRecommendations[\s\S]*?(?:content|input|text):\s*String/,
  'Enricher results must not retain clipboard content');
assert.match(analysisExecution, /pub struct AnalyzerSnapshot/,
  'The whole Analyzer must expose one typed snapshot contract');
assert.match(database, /pub fn save_text_clip[\s\S]*?analysis_execution::analyze_text/,
  'Text capture must reuse the whole-Analyzer execution service');
const textCapture = database.slice(
  database.indexOf('pub fn save_text_clip'),
  database.indexOf('pub(crate) fn merge_external_text_clips'),
);
assert.doesNotMatch(textCapture, /detection_execution::analyze_detectors_with_policy/,
  'Text capture must not schedule a parallel Detector-only Analysis request');
assert.match(database, /save_clip_with_structure[\s\S]*?record_structural_inspection/,
  'Text capture must persist its precomputed structural snapshot instead of re-running inspection');
assert.match(commands, /analysis_execution::analyze_(?:text|clip)/,
  'GUI whole-Analyzer previews must use the shared execution service');
assert.match(cli, /analysis_execution::analyze_(?:text|clip)/,
  'CLI whole-Analyzer previews must use the shared execution service');
assert.match(tauriMock, /case 'analyze_content'/,
  'The frontend mock must preserve the whole-Analyzer contract');
assert.match(clipPreview, /invoke<AnalyzerPreview>\('analyze_content'/,
  'Clip Preview must request structure and recommendations through one Analyzer call');
assert.match(clipPreview, /includeDetectors: includeEnricher/,
  'Clip Preview must skip classification when its Enricher consumer is disabled');
assert.doesNotMatch(clipPreview, /invoke<StructuralInspection>\('inspect_clip_structure'|invoke<SmartActionEnrichment>\('enrich_smart_actions'/,
  'Clip Preview must not schedule Analyzer participants through parallel IPC calls');
assert.doesNotMatch(analysisExecution, /pub struct AnalyzerSnapshot[\s\S]*?pub (?:text|content|paths|image_bytes):/,
  'Whole-Analyzer snapshots must not expose clipboard contents, OCR text, paths, or image bytes');
assert.doesNotMatch(analysis, /pub (?:enum|struct) (?:AnalysisPass|ParticipantContract|ParticipantOutcome|AnalysisFailure|ParticipantRun)/,
  'The scheduler must consume shared Analysis contracts instead of redefining them');
for (const boundary of ['resolve_participant', 'AnalysisTargetKind', 'ClipApplication', 'enrichment_execution.rs']) {
  assert.ok(analysisArchitecture.includes(boundary), `Analysis extension guidance must document ${boundary}`);
}
assert.match(extraction, /pub fn representation_contract/,
  'Extractors must parse metadata through the shared representation contract');
assert.match(extraction, /code: "invalid_contract"/,
  'Extractor engines must fail closed for unsupported representation contracts');
assert.doesNotMatch(database, /extractor\.input_contract == "image"/,
  'Active Extractor selection must not compare representation metadata ad hoc');
assert.match(analysisContract, /MAX_ANALYSIS_PASSES:\s*usize\s*=\s*4/, 'Analysis must remain bounded to four ordered passes');
assert.match(extraction, /pub trait ExtractorEngine:\s*Sync/, 'Extractor engines must use the shared runtime contract');
assert.match(extraction, /pub enum ExtractionOutcome/, 'Extractor execution must return a typed outcome');
assert.match(analysis, /ExtractorEngineRegistry/, 'Analysis must dispatch Extractors through the shared engine registry');
assert.match(extractionExecution, /pub struct ExtractionResult/,
  'Image Analysis must expose one shared execution-result contract');
assert.match(extractionExecution, /pub metadata: AnalysisMetadata/,
  'Extractor results must carry the shared version and policy metadata');
assert.match(extractionExecution, /pub participants: Vec<ParticipantRun>/,
  'Image Analysis results must expose privacy-safe participant summaries');
assert.match(extractionExecution, /resolve_participant/,
  'Extractor results must use shared participant normalization');
assert.match(extractionExecution, /pub application: ClipApplication/,
  'Extractor results must use shared clip-application state');
assert.doesNotMatch(extractionExecution, /pub enum AnalysisTargetKind/,
  'Extractor targets must use the shared Analysis target kind');
assert.doesNotMatch(analysis, /pub fn analyze_image\(/,
  'Raw Image Analysis reports must not remain a public execution path');
assert.match(extractionExecution, /pub struct ExtractionApplicationResult/,
  'Extractor application must expose one shared serializable result contract');
assert.match(extractionExecution, /pub fn apply_image_analysis/,
  'User-initiated Extractor application must use one shared application service');
assert.match(extractionExecution, /pub fn persist_claimed_image_analysis/,
  'Claimed background Extractor work must use the shared persistence service');
assert.match(cli, /ExtractionApplicationResult::preview/,
  'CLI Extractor previews must use the shared application result shape');
assert.match(cli, /extraction_execution::apply_image_analysis/,
  'CLI Extractor apply must use the shared application service');
assert.match(commands, /extraction_execution::apply_image_analysis/,
  'GUI Extractor apply must use the shared application service');
assert.match(ocr, /extraction_execution::persist_claimed_image_analysis/,
  'Background OCR must use the shared claimed-work application service');
assert.doesNotMatch(cli, /output\["appliedClipId"\]/,
  'CLI Extractor JSON must not synthesize application state');
assert.match(cli, /serde_json::json!\(&result\)/,
  'CLI Extractor JSON must serialize the shared application result');
assert.match(database, /pub fn complete_or_reset_ocr_attempt/,
  'OCR runtimes must share failure-safe attempt persistence');
assert.match(extractionExecution, /analysis\s*\.failure\s*\.as_ref\(\)/,
  'Shared Image Analysis persistence must preserve Extractor failure codes');
assert.match(extractionExecution, /complete_or_reset_ocr_attempt/,
  'Shared Image Analysis persistence must reset claimed work on failure');
assert.match(ocr, /extraction_execution::analyze_image_with_registry/,
  'Background OCR must use the shared Image Analysis execution result');
assert.match(cli, /extraction_execution::analyze_image/,
  'CLI OCR must use the shared Image Analysis execution result');
assert.match(commands, /extraction_execution::analyze_image/,
  'GUI OCR must use the shared Image Analysis execution result');
assert.doesNotMatch(ocr, /record_analysis_classification/,
  'Background OCR must not persist derived classifications independently');
assert.doesNotMatch(cli, /record_analysis_classification/,
  'CLI OCR must not persist derived classifications independently');
assert.doesNotMatch(commands, /record_analysis_classification/,
  'GUI OCR must not persist derived classifications independently');
assert.doesNotMatch(cli, /perform_ocr_on_image_bytes/, 'The CLI must not bypass the shared Extractor engine registry');
assert.match(clipboardMonitor, /save_text_clip/, 'GUI capture must use the shared text-capture service');
assert.match(detectionExecution, /pub struct DetectionResult/,
  'Detection must expose one shared execution-result contract');
assert.match(detectionExecution, /pub metadata: AnalysisMetadata/,
  'Detector results must carry the shared version and policy metadata');
assert.match(detectionExecution, /pub struct DetectionApplicationResult/,
  'Detector application must expose one shared mutation result');
assert.match(detectionExecution, /pub participants: Vec<ParticipantRun>/,
  'Detection results must expose privacy-safe participant summaries');
assert.match(detectionExecution, /resolve_participant/,
  'Detector results must use shared participant normalization');
assert.match(detectionExecution, /pub application: ClipApplication/,
  'Detector results must use shared clip-application state');
assert.doesNotMatch(detectionExecution, /pub enum DetectionTargetKind/,
  'Detector targets must use the shared Analysis target kind');
assert.doesNotMatch(analysis, /pub fn analyze_text\(/,
  'Raw text Analysis reports must not remain a public execution path');
assert.match(database, /detection_execution::analyze_detectors/,
  'Text capture and Detector rescans must use the shared Detection result');
assert.match(database, /detection_execution::analyze_detector\(&text, &detector\)/,
  'Detector application must execute transactionally through the shared result');
assert.match(cli, /DetectionApplicationResult::preview/,
  'CLI Detector previews must use the shared application result shape');
assert.match(cli, /serde_json::json!\(&result\)/,
  'CLI Detector JSON must serialize the shared Detection result');
assert.match(commands, /detection_execution::analyze_detector/,
  'GUI Detector tests must use the shared Detection result');
assert.match(tauriMock, /case 'test_content_detector':[\s\S]*?formatVersion:\s*1,[\s\S]*?policy:\s*'interactive',[\s\S]*?through:\s*'enrich'/,
  'The frontend Detector mock must preserve shared Analysis metadata');
assert.doesNotMatch(database, /content_analysis::classify_text/,
  'Database classification paths must not infer results from raw Analyzer reports');
assert.doesNotMatch(cli, /content_analysis::analyze_text/,
  'CLI Detector runs must not infer results from raw Analyzer reports');
assert.doesNotMatch(commands, /content_analysis::classify_text/,
  'GUI Detector tests must not infer results from raw Analyzer reports');
assert.match(cli, /db\.save_text_clip/, 'CLI capture must use the shared text-capture service');
assert.doesNotMatch(ocr, /content_analysis::analyze_image/, 'Background OCR must not infer results directly from Analyzer reports');
assert.doesNotMatch(cli, /content_analysis::analyze_image/, 'CLI OCR must not infer results directly from Analyzer reports');
assert.doesNotMatch(commands, /content_analysis::analyze_image/, 'GUI OCR must not infer results directly from Analyzer reports');
for (const method of ['get_content_extractors', 'create_content_extractor', 'update_content_extractor_definition', 'duplicate_content_extractor', 'delete_content_extractor', 'restore_default_content_extractors']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(commands, new RegExp(`pub fn ${method}`), `${method} must be exposed to the GUI`);
  assert.match(cli, new RegExp(`db\\s*\\.${method}`), `${method} must be reused by the CLI`);
}
assert.match(tauriMock, /function mockBuiltinExtractors\(\)/,
  'The frontend mock must retain canonical shipped Extractor definitions');
assert.match(tauriMock, /case 'extract_ocr_from_clip':[\s\S]*?appliedClipId:[\s\S]*?ocrUpdated:/,
  'The frontend mock must preserve the shared Extractor application result');
assert.match(tauriMock, /case 'extract_ocr_from_clip':[\s\S]*?formatVersion:\s*1,[\s\S]*?policy:\s*'interactive',[\s\S]*?through:\s*'enrich'/,
  'The frontend Extractor mock must preserve shared Analysis metadata');
assert.match(tauriMock, /case 'restore_default_content_extractors':[\s\S]*?mockBuiltinExtractors\(\)[\s\S]*?builtinRefs\.has\(extractor\.stableRef\)/,
  'Mock Restore Defaults must recreate all shipped Extractors without replacing custom Extractors');
assert.match(database, /pub fn get_content_detector/, 'Resolving one Detector must live in the shared database domain layer');
assert.match(cli, /db\s*\.get_content_detector/, 'Detector references must use the shared database domain layer');
for (const method of ['create_content_detector', 'update_content_detector', 'duplicate_content_detector', 'delete_content_detector']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(commands, new RegExp(`pub fn ${method}`), `${method} must be exposed to the GUI`);
  assert.match(cli, new RegExp(`db\\s*\\.${method}`), `${method} must be reused by the CLI`);
}
assert.match(database, /pub fn apply_content_detector/, 'Applying one Detector must live in the shared database domain layer');
assert.match(cli, /db\s*\.apply_content_detector/, 'Detector --apply must use the shared database domain layer');
for (const family of ['extractor', 'detector', 'transform']) {
  for (const verb of ['list', 'get', 'create', 'update', 'duplicate', 'delete', 'run']) {
    assert.ok(help.includes(`pasted ${family} ${verb}`), `Help & Docs must cover pasted ${family} ${verb}`);
  }
}
for (const method of ['get_operation', 'create_operation', 'update_operation', 'duplicate_operation', 'delete_operation']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(cli, new RegExp(`db\\s*\\.${method}`), `${method} must be reused by the CLI`);
  if (method !== 'get_operation') {
    assert.match(commands, new RegExp(`pub fn ${method}`), `${method} must be exposed to the GUI`);
  }
}
for (const verb of ['list', 'get', 'create', 'update', 'duplicate', 'delete', 'run']) {
  assert.ok(help.includes(`pasted operation ${verb}`), `Help & Docs must cover pasted operation ${verb}`);
}
for (const method of ['get_intelligence_connections', 'get_intelligence_connection', 'create_intelligence_connection', 'update_intelligence_connection', 'delete_intelligence_connection', 'reorder_intelligence_connections']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(cli, new RegExp(`db\\s*\\.${method}`), `${method} must be reused by the CLI`);
}
for (const method of ['get_bin', 'create_bin', 'update_bin', 'delete_bin', 'update_bin_shortcut', 'set_bin_transform_ref']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(cli, new RegExp(`db\\s*\\.${method}`), `${method} must be reused by the CLI`);
}
for (const method of ['update_clip_note', 'get_clip_versions_page', 'restore_clip_version', 'get_clip_transformation_provenance', 'purge_clip_permanently', 'empty_trash', 'get_analytics_summary']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(cli, new RegExp(`db\\s*\\.${method}`), `${method} must be reused by the CLI`);
}
assert.match(cli, /intelligence_executor::plan_intent/, 'CLI Transform planning must use the shared intelligence executor');
assert.match(commands, /db\.get_library_items/, 'GUI library metadata must use the shared domain service');
assert.match(cli, /db\.get_library_items/, 'CLI library metadata must use the shared domain service');
assert.match(commands, /db\.set_library_item_enabled/, 'GUI lifecycle toggles must use the shared domain service');
assert.match(cli, /db\.set_library_item_enabled/, 'CLI lifecycle toggles must use the shared domain service');
assert.match(extractorManager, /invoke\('set_library_item_enabled',[\s\S]*?kind: 'extractor'/,
  'GUI Extractor toggles must use the shared lifecycle service');
assert.match(detectorManager, /invoke\('set_library_item_enabled',[\s\S]*?kind: 'detector'/,
  'GUI Detector toggles must use the shared lifecycle service');
assert.match(tauriMock, /case 'set_library_item_enabled':[\s\S]*?let matched = false;[\s\S]*?if \(!matched\) throw new Error/,
  'Frontend lifecycle mocks must reject missing items like the shared domain service');
for (const method of ['get_content_types', 'create_content_type', 'update_content_type', 'set_content_type_archived', 'restore_default_content_types']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(commands, new RegExp(`pub async fn ${method}|pub fn ${method}`), `${method} must be exposed to the GUI`);
  assert.match(cli, new RegExp(`db\\s*\\.${method}`), `${method} must be reused by the CLI`);
}
for (const method of ['get_content_type_groups', 'create_content_type_group', 'update_content_type_group', 'set_content_type_group_archived', 'delete_content_type_group', 'restore_default_content_type_groups']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(commands, new RegExp(`pub fn ${method}`), `${method} must be exposed to the GUI`);
  assert.match(cli, new RegExp(`db\\s*\\.${method}`), `${method} must be reused by the CLI`);
}

console.log('GUI and CLI parity audit passed.');
