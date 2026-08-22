import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { readRustModuleTree } from './audit-source-trees.js';

const read = (path) => readFileSync(path, 'utf8');
const lineCount = (path) => read(path).trimEnd().split(/\r?\n/).length;
const englishCatalog = JSON.parse(read('src/locales/en.json'));
const fixture = (name) => JSON.parse(read(`contracts/analysis/v1/${name}.json`));
const frontendMock = [
  read('src/utils/tauri.ts'),
  ...readdirSync('src/mocks/browser').filter((name) => name.endsWith('.ts')).sort()
    .map((name) => read(`src/mocks/browser/${name}`)),
].join('\n');
const cliTests = read('src-tauri/tests/cli_integration.rs');
const clipPreview = [
  'src/components/ClipPreview.tsx',
  'src/components/ClipPreviewContent.tsx',
  'src/components/ClipPreviewFooter.tsx',
  'src/components/ClipPreviewNotesPanel.tsx',
  'src/components/clipPreviewModel.ts',
  'src/hooks/useClipPreviewAnalysis.ts',
  'src/hooks/useClipPreviewNotes.ts',
  'src/hooks/useClipPreviewRevisions.ts',
].map(read).join('\n');
const clipPreviewContent = read('src/components/ClipPreviewContent.tsx');
const clipCard = read('src/components/ClipCard.tsx');
const analytics = read('src/components/AnalyticsView.tsx');
const clipOrder = read('src/utils/clipOrder.ts');
const database = readRustModuleTree('src-tauri/src/db.rs', 'src-tauri/src/db');
const types = read('src/types.ts');
const analysisSettingsShell = read('src/components/SettingsAnalysisPanel.tsx');
const analysisLifecycle = read('src/components/AnalysisLifecycleSequence.tsx');
const analysisMaintenance = read('src/hooks/useAnalysisMaintenance.ts');
const classifierManagerFiles = [
  'src/hooks/useClassifierManager.ts',
  'src/components/ClassifierManagerDialog.tsx',
  'src/components/classifierModel.ts',
];
const classifierManager = classifierManagerFiles.map(read).join('\n');
const analysisSettings = [
  analysisSettingsShell,
  analysisLifecycle,
  analysisMaintenance,
  classifierManager,
].join('\n');
const settingsModal = read('src/components/SettingsModal.tsx');
const analysisExecution = read('src-tauri/src/analysis_execution.rs');
const commands = readRustModuleTree('src-tauri/src/commands.rs', 'src-tauri/src/commands');
const builtinLifecycleManager = read('src/components/BuiltinLifecycleManagerDialog.tsx');
const analysisApi = read('src/api/analysis.ts');
const extractorManagerFiles = [
  'src/hooks/useContentExtractorManager.ts',
  'src/components/ContentExtractorManagerDialog.tsx',
  'src/components/ExtractorAuthoringHistoryDialog.tsx',
  'src/components/ExtractorRecipeEditor.tsx',
  'src/components/ExtractorRegistryPanel.tsx',
  'src/components/contentExtractorModel.ts',
  'src/components/contentExtractorPolicy.ts',
];
const extractorManager = extractorManagerFiles.map(read).join('\n');
const contentTypeManager = read('src/components/ContentTypeManagerDialog.tsx');
const contentTypeGroupManager = read('src/components/ContentTypeGroupManagerDialog.tsx');
const registryPanelHeader = read('src/components/RegistryPanelHeader.tsx');
const registryPanelFooter = read('src/components/RegistryPanelFooter.tsx');
const architecture = read('docs/ANALYSIS_ARCHITECTURE.md');
const releaseChecklist = read('docs/RELEASE_CHECKLIST_1.0.0.md');

for (const [step, participant, icon] of [
  [1, 'Capture', 'Clipboard'],
  [2, 'Inspect', 'ScanSearch'],
  [3, 'Extract', 'ScanText'],
]) {
  const participantKey = `component.settingsAnalysisPanel.${participant.toLowerCase()}`;
  assert.match(
    analysisLifecycle,
    new RegExp(`step=\\{${step}\\}[\\s\\S]{0,80}icon=\\{${icon}\\}[\\s\\S]{0,120}translate\\('${participantKey.replaceAll('.', '\\.')}\\'\\)`),
    `Analysis Settings must present ${participant} as ordered lifecycle step ${step} with its icon`,
  );
  assert.equal(englishCatalog[participantKey], participant);
}
assert.match(
  analysisLifecycle,
  /searchEnabled && <AnalysisManagerRow[\s\S]{0,100}step=\{4\}[\s\S]{0,80}icon=\{Search\}[\s\S]{0,140}settingsAnalysisPanel\.index/,
  'Analysis Settings must present Index after Extract when Clip Search is enabled',
);
assert.equal(englishCatalog['component.settingsAnalysisPanel.index'], 'Index');
assert.match(
  analysisLifecycle,
  /step=\{searchEnabled \? 5 : 4\}[\s\S]{0,80}icon=\{Radar\}[\s\S]{0,140}settingsAnalysisPanel\.classify/,
  'Classify must follow the optional Index lifecycle stage',
);
assert.match(
  analysisLifecycle,
  /step=\{4 \+ Number\(searchEnabled\) \+ Number\(classificationEnabled\)\}[\s\S]{0,80}icon=\{Lightbulb\}/,
  'Suggest must follow the enabled lifecycle stages',
);
assert.match(
  analysisLifecycle,
  /translate\('component\.settingsAnalysisPanel\.notAllStepsRunForAllClipsSomeStepsMayBeLong'\)/,
  'Analysis Settings must explain that the ordered passes are conditional',
);
assert.match(settingsModal, /activeTab === 'analysis' && \(\s*<SettingsAnalysisPanel/,
  'Analysis Settings must remain available when optional participants are disabled');
assert.doesNotMatch(settingsModal, /showAnalysis=/,
  'Functionality gates must not hide Analysis configuration');
assert.match(analysisLifecycle, /step=\{3\}[\s\S]{0,220}translate\('component\.settingsAnalysisPanel\.extract'\)/,
  'Extractor management must remain visible for user-defined recipes');
assert.match(analysisLifecycle, /\{classificationEnabled && <AnalysisManagerRow[\s\S]{0,220}translate\('component\.settingsAnalysisPanel\.classify'\)/,
  'Classifiers must remain visible for either Content Classification or Types');
assert.match(settingsModal, /typesEnabled=\{settings\.enableTypes\}/,
  'Analysis Settings must receive the Types feature gate');
assert.match(settingsModal, /sourcesEnabled=\{settings\.enableSources\}/,
  'Analysis Settings must receive the Sources feature gate');
assert.match(analysisLifecycle, /step=\{1\}[\s\S]{0,220}translate\('component\.settingsAnalysisPanel\.capture'\)/,
  'Capture must remain visible independently of optional presentation features');
assert.match(builtinLifecycleManager, /stableRef !== 'capture:source-attribution-v1'/,
  'Disabling Sources must hide Source Attribution without hiding Clip Type');
assert.match(clipCard, /features\.clipTypes \|\| \(features\.types[\s\S]{0,260}structuralClipType/,
  'Clip cards must independently gate structural and detected type chrome');
assert.match(clipCard, /features\.sources && <span className="font-medium theme-text-main/,
  'Clip cards must hide Source chrome when Sources is disabled');
assert.match(clipPreview, /features\.clipTypes && <span[\s\S]{0,500}contentTypeLabel\(structuralClipType\(clip\.content_type\)\)/,
  'Clip Preview must hide structural Clip Type chrome when disabled');
assert.match(clipPreview, /features\.types && visibleContentTypes\.map/,
  'Clip Preview must hide detected Content Types when Content Types is disabled');
assert.match(clipPreview, /features\.sources && <OverflowText text=\{localizedSourceName\(clip\.source\)\}/,
  'Clip Preview must hide its Source label when Sources is disabled');
assert.match(analytics, /features\.sources && <div className="theme-panel[\s\S]{0,1000}translate\('component\.analyticsView\.topSourceInHistory'\)/,
  'Insights must hide Source summaries when Sources is disabled');
assert.match(analytics, /features\.clipTypes && <div className="theme-panel[\s\S]{0,1000}translate\('component\.analyticsView\.clipsByClipType'\)/,
  'Insights must hide structural Clip Type summaries when Clip Types is disabled');
assert.match(analytics, /translate\('component\.analyticsView\.clipsByFileFormat'\)/,
  'Insights must present file-format summaries separately');
assert.match(analytics, /features\.types && <div className="theme-panel[\s\S]{0,1000}translate\('component\.analyticsView\.clipsByContentType'\)/,
  'Insights must hide semantic Content Type summaries when Content Types is disabled');
assert.match(database, /get_daily_activity_for_calendar[\s\S]{0,1800}date\(clips\.created_at, \?2\)/,
  'Insights daily activity must group stored UTC instants through an explicit calendar modifier');
assert.match(database, /get_daily_activity_for_calendar\([\s\S]{0,180}"localtime"/,
  'Insights must request the machine-local calendar from the shared domain summary');
assert.match(analytics, /listen\(APP_EVENTS\.clipAdded, refresh\)/,
  'Insights must refresh when History receives a clip');
assert.match(analytics, /listen\(APP_EVENTS\.clipLibraryChanged, refresh\)/,
  'Insights must refresh after other History mutations');
assert.match(analytics, /nextMidnight[\s\S]{0,500}setTimeout/,
  'Insights must refresh across the local midnight boundary');
assert.match(clipOrder, /parseDbDate\(left\.created_at\)[\s\S]{0,300}rightTimestamp - leftTimestamp/,
  'History ordering must compare timestamp instants rather than mixed timestamp strings');
assert.match(analysisLifecycle, /\{transformationsEnabled && <AnalysisManagerRow[\s\S]{0,220}translate\('component\.settingsAnalysisPanel\.suggest'\)/,
  'Disabling Transformations must hide Smart Action Suggestions');
assert.match(analysisExecution, /let run_classifiers = allow_text_participants && options\.include_classifiers;/,
  'Suggestion must not implicitly enable Classifiers');
assert.match(commands, /include_classifiers: include_classifiers\.unwrap_or\(true\)[\s\S]{0,100}Feature::ContentClassification/,
  'The live Analyzer command must honor the Content Classification feature gate');
assert.match(extractorManager, /extractor:apple-vision-ocr'[\s\S]{0,100}extractor:tesseract-ocr'[\s\S]{0,80}ocrEnabled/,
  'Disabling OCR must hide only the shipped OCR Extractors');
assert.match(extractorManager, /extractor:whisper-transcription'[\s\S]{0,80}transcriptionsEnabled/,
  'Disabling Transcriptions must hide only the shipped transcription Extractor');
assert.match(extractorManager, /value: 'original_text',[\s\S]{0,100}translate\('component\.contentExtractorManagerDialog\.text'\)[\s\S]{0,40}disabled: true/,
  'Extractor settings must explain that Text clips are already searchable');
assert.match(extractorManager, /value: 'image',[\s\S]{0,100}translate\('component\.contentExtractorManagerDialog\.image'\)/,
  'Extractor settings must present the Image Clip Type instead of its wire contract');
assert.match(extractorManager, /value: 'file_references',[\s\S]{0,100}translate\('component\.contentExtractorManagerDialog\.files'\)/,
  'Extractor settings must present the Files Clip Type instead of its wire contract');
assert.match(extractorManager, /value: 'searchable_text',[\s\S]{0,100}translate\('component\.contentExtractorManagerDialog\.searchableText'\)/,
  'Extractor settings must present the searchable-text output contract readably');
assert.doesNotMatch(extractorManager, />Pass<|>extract<\/strong>/,
  'Extractor settings must not repeat the enclosing Analysis step');
assert.ok(lineCount('src/components/ContentExtractorManagerDialog.tsx') <= 215,
  'The Extractor manager coordinator must stay within its extracted size boundary');
assert.match(read('src/hooks/useContentExtractorManager.ts'), /export function useContentExtractorManager/,
  'Extractor persistence and authoring state must remain in its focused controller');
assert.match(read('src/components/ExtractorRecipeEditor.tsx'), /export function ExtractorRecipeEditor/,
  'Advanced Extractor recipe editing must remain in its focused surface');
assert.match(read('src/components/contentExtractorPolicy.ts'), /export function visibleContentExtractors/,
  'Extractor feature visibility policy must remain independent from its React controller');
assert.match(read('src/components/contentExtractorPolicy.ts'), /export function canSaveExtractorRecipe/,
  'Extractor recipe validation policy must remain independently testable');
assert.ok(lineCount('src/components/SettingsAnalysisPanel.tsx') <= 129,
  'The Analysis settings coordinator must stay within its extracted size boundary');
assert.ok(lineCount('src/components/ClassifierManagerDialog.tsx') <= 232,
  'The Classifier manager surface must stay within its extracted size boundary');
assert.ok(lineCount('src/hooks/useClassifierManager.ts') <= 297,
  'The Classifier controller must stay within its extracted size boundary');
assert.ok(lineCount('src/components/AnalysisLifecycleSequence.tsx') <= 113,
  'The Analysis lifecycle surface must stay within its extracted size boundary');
assert.ok(lineCount('src/hooks/useAnalysisMaintenance.ts') <= 126,
  'Analysis maintenance workflows must stay within their extracted size boundary');
assert.ok(lineCount('src/components/classifierModel.ts') <= 95,
  'Classifier data and draft policy must stay within its extracted size boundary');
assert.match(analysisSettingsShell, /<ClassifierManagerDialog/,
  'Analysis Settings must delegate Classifier management to its focused dialog');
assert.match(analysisSettingsShell, /useAnalysisMaintenance\(/,
  'Analysis Settings must delegate reset and rescan workflows to their focused controller');
assert.doesNotMatch(classifierManager, /String\(error\)/,
  'Classifier management must preserve structured backend error messages');
assert.match(classifierManager, /errorMessage\(error\)/,
  'Classifier management must use the shared structured error presenter');
assert.match(analysisSettingsShell, /<ContentExtractorManagerDialog[\s\S]{0,220}ocrEnabled=\{ocrEnabled\}[\s\S]{0,100}transcriptionsEnabled=\{transcriptionsEnabled\}/,
  'Analysis Settings must pass both extraction feature gates to Extractor management');
assert.match(commands, /extract_text_from_file_clip[\s\S]{0,400}active_file_text_extractors_for_features\(transcriptions_enabled\)/,
  'Native file extraction must preserve custom Extractors when Transcriptions is disabled');
assert.match(clipPreviewContent, /transcriptionsEnabled && <section className="theme-panel overflow-hidden rounded-xl border shadow-lg">/,
  'Clip Preview must hide transcription controls when Transcriptions is disabled');
for (const command of [
  'restore_default_content_extractors',
  'restore_default_content_classifiers',
  'restore_default_content_types',
  'restore_default_content_type_groups',
]) {
  assert.match(
    analysisMaintenance,
    new RegExp(`invoke(?:<[^>]+>)?\\('${command}'\\)`),
    `The global Analysis restore must include ${command}`,
  );
}
assert.match(analysisSettingsShell, /onReset=\{restoreAnalysis\}/,
  'Analysis Settings must connect the global restore workflow to the lifecycle surface');
assert.match(analysisLifecycle, /<ActionButton onClick=\{onReset\}[\s\S]{0,180}translate\('component\.settingsAnalysisPanel\.reset'\)/,
  'Analysis Settings must expose one global Reset action');

function commandBlock(command) {
  const start = frontendMock.indexOf(`case '${command}':`);
  assert.notEqual(start, -1, `Frontend mock must implement ${command}`);
  const end = frontendMock.indexOf("\n    case '", start + 1);
  return frontendMock.slice(start, end === -1 ? undefined : end);
}

const fixtures = {
  analyzer: fixture('analyzer-interactive-text'),
  analyzerCapture: fixture('analyzer-capture-text'),
  inspector: fixture('inspector-interactive-text'),
  extractor: fixture('extractor-interactive-produced'),
  extractorUnavailable: fixture('extractor-interactive-unavailable'),
  classifier: fixture('classifier-interactive-matched'),
  classifierNoMatch: fixture('classifier-interactive-no-match'),
  suggestion: fixture('suggestion-interactive-empty'),
};

for (const [surface, value] of Object.entries(fixtures)) {
  assert.equal(value.formatVersion, 1, `${surface} fixture must use Analysis contract v1`);
  assert.ok(['capture', 'interactive'].includes(value.policy), `${surface} fixture must name its policy`);
  assert.equal(value.through, value.policy === 'capture' ? 'classify' : 'suggest',
    `${surface} fixture must name its policy's final pass`);
  assert.ok(Array.isArray(value.participants), `${surface} fixture must include participant runs`);
  for (const run of value.participants) {
    for (const field of ['stableRef', 'pass', 'outcome']) {
      assert.ok(Object.hasOwn(run, field), `${surface} participant must include ${field}`);
    }
    for (const field of Object.keys(run)) {
      assert.doesNotMatch(field, /content|credential|input|output|password|path|secret|text|token/i,
        `${surface} participant field ${field} must remain content-free`);
    }
  }
  assert.equal(value.appliedClipId, null, `${surface} fixture must describe a non-mutating preview`);
  assert.doesNotMatch(JSON.stringify(value), /token|password|secret|credential|private\//i,
    `${surface} fixture must remain privacy-safe`);
}

const analyzerMock = commandBlock('analyze_content');
assert.match(analyzerMock, /const includeClassifiers = request\.includeClassifiers !== false/,
  'Frontend Analyzer mock must not implicitly enable Classifiers for Suggestions');
for (const field of ['formatVersion', 'policy', 'through', 'participants']) {
  assert.match(analyzerMock, new RegExp(`\\b${field}\\b`), `Frontend Analyzer mock must include ${field}`);
}
const extractorMock = commandBlock('extract_ocr_from_clip');
for (const field of Object.keys(fixtures.extractor)) {
  assert.match(
    extractorMock,
    new RegExp(`${field}:`),
    `Frontend Extractor mock must preserve the canonical ${field} field`,
  );
}
for (const field of Object.keys(fixtures.analyzer.result)) {
  assert.match(
    analyzerMock,
    new RegExp(`\\b${field}\\b`),
    `Frontend Analyzer mock must preserve the canonical result.${field} field`,
  );
}
for (const name of [
  'analyzer-interactive-text',
  'analyzer-capture-text',
  'inspector-interactive-text',
  'suggestion-interactive-empty',
  'extractor-interactive-unavailable',
  'classifier-interactive-no-match',
]) {
  assert.ok(cliTests.includes(`${name}.json`), `CLI integration must consume ${name}.json`);
}

for (const field of ['structure', 'mediaMetadata', 'suggestions']) {
  assert.match(clipPreview, new RegExp(`result\\.${field}`),
    `Clip Preview must consume whole-Analyzer ${field}`);
}
assert.match(clipPreview, /translate\('component\.clipPreview\.smartActionsSignals'/, 'Clip Preview must present Smart Action suggestions contextually');
assert.match(clipPreviewContent, /translate\('component\.clipPreviewContent\.extractedByName'/,
  'Clip Preview must identify the Extractor that produced OCR text');
for (const field of ['ocr_extractor_ref', 'ocr_extractor_name', 'ocr_engine_version']) {
  assert.match(database, new RegExp(`pub ${field}: Option<String>`),
    `Shared ClipItem must expose OCR provenance field ${field}`);
  assert.match(types, new RegExp(`${field}\\?: string \\| null`),
    `Frontend ClipItem must expose OCR provenance field ${field}`);
}
for (const title of ['Capture', 'Inspect', 'Extract', 'Classify', 'Suggest']) {
  assert.match(analysisLifecycle, new RegExp(`translate\\('component\\.settingsAnalysisPanel\\.${title.toLowerCase()}'\\)`),
    `Analysis settings must expose ${title} behind a compact manager row`);
}
assert.match(builtinLifecycleManager, /get_library_items/,
  'Capture, Inspector, and Suggestion managers must consume the shared registry');
assert.match(builtinLifecycleManager, /participantContract/,
  'Inspector and Suggestion managers must render typed participant contracts');
assert.match(builtinLifecycleManager, /typeRelations/,
  'Inspector and Suggestion managers must render registered Type relations');
assert.match(builtinLifecycleManager, /analysisApi\.listInspectors/,
  'Inspector management must load engine availability through the centralized Analysis client');
assert.match(analysisApi, /get_content_inspectors/,
  'The Analysis client must load shared Inspector engine availability');
assert.match(builtinLifecycleManager, /translate\('common\.technicalDetails'\)/,
  'Internal participant contracts must remain behind contextual technical details');
assert.match(builtinLifecycleManager, /captureStableReferenceUsage[\s\S]{0,200}pasted registry list --kind capture --json[\s\S]{0,300}stableReferenceUsage[\s\S]{0,200}get <ref> --json/,
  'Stable references must explain their CLI and API purpose');
for (const [label, manager] of [['Extractor', extractorManager], ['Classifier', classifierManager]]) {
  assert.equal((manager.match(/<RegistryPanelFooter/g) ?? []).length, 2,
    `${label} management must keep item actions and form actions in their owning panels`);
  assert.match(manager, /<AppDialogFooter[\s\S]*translate\('component\.(?:contentExtractorManagerDialog|settingsAnalysisPanel)\.reset'\)[\s\S]*translate\('common\.close'\)/,
    `${label} management must keep scoped restore and close actions in the modal footer`);
  assert.match(manager, /discardDraftThen[\s\S]*ConfirmationDialog/,
    `${label} management must protect edited drafts with the shared confirmation UI`);
  assert.doesNotMatch(manager, /window\.confirm/,
    `${label} management must not fall back to a browser confirmation prompt`);
}
for (const [label, manager] of [
  ['Content Type', contentTypeManager],
  ['Content Type Group', contentTypeGroupManager],
]) {
  assert.match(manager, /ConfirmationDialog/,
    `${label} management must use the shared nested confirmation UI`);
  assert.doesNotMatch(manager, /window\.confirm/,
    `${label} management must not fall back to a browser confirmation prompt`);
}
assert.match(registryPanelHeader, /min-h-\[49px\]/,
  'Registry panel headers must match the rendered action-header height');
for (const [label, panelEdge] of [['header', registryPanelHeader], ['footer', registryPanelFooter]]) {
  assert.match(panelEdge, /shrink-0/,
    `Registry panel ${label}s must not shrink below their aligned minimum height`);
}
assert.match(registryPanelFooter, /min-h-12/,
  'Registry panel footers must retain their aligned minimum height with and without actions');
assert.match(architecture, /## Version 1 contract/, 'Analysis architecture must declare the version 1 contract');
assert.match(releaseChecklist, /## Content Analysis/, 'The release checklist must retain Analysis acceptance');

console.log('Analysis JSON contract audit passed.');
