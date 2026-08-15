import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(path, 'utf8');
const fixture = (name) => JSON.parse(read(`contracts/analysis/v1/${name}.json`));
const frontendMock = read('src/utils/tauri.ts');
const cliTests = read('src-tauri/tests/cli_integration.rs');
const clipPreview = read('src/components/ClipPreview.tsx');
const clipPreviewContent = read('src/components/ClipPreviewContent.tsx');
const clipCard = read('src/components/ClipCard.tsx');
const analytics = read('src/components/AnalyticsView.tsx');
const database = read('src-tauri/src/db.rs');
const types = read('src/types.ts');
const analysisSettings = read('src/components/SettingsDetectionPanel.tsx');
const settingsModal = read('src/components/SettingsModal.tsx');
const analysisExecution = read('src-tauri/src/analysis_execution.rs');
const commands = read('src-tauri/src/commands.rs');
const builtinLifecycleManager = read('src/components/BuiltinLifecycleManagerDialog.tsx');
const extractorManager = read('src/components/ContentExtractorManagerDialog.tsx');
const registryPanelHeader = read('src/components/RegistryPanelHeader.tsx');
const registryPanelFooter = read('src/components/RegistryPanelFooter.tsx');
const architecture = read('docs/ANALYSIS_ARCHITECTURE.md');
const releaseChecklist = read('docs/RELEASE_CHECKLIST_1.0.0.md');

for (const [step, participant, icon] of [
  [1, 'Capture', 'Clipboard'],
  [2, 'Inspect', 'ScanSearch'],
  [3, 'Extract', 'ScanText'],
  [4, 'Detect', 'Radar'],
  [5, 'Enrich', 'Lightbulb'],
]) {
  assert.match(
    analysisSettings,
    new RegExp(`step=\\{${step}\\}[\\s\\S]{0,80}icon=\\{${icon}\\}[\\s\\S]{0,80}title="${participant}"`),
    `Analysis Settings must present ${participant} as ordered lifecycle step ${step} with its icon`,
  );
}
assert.match(
  analysisSettings,
  /Not all steps run for all clips\. Some steps may be long-running\./,
  'Analysis Settings must explain that the ordered passes are conditional',
);
assert.match(settingsModal, /activeTab === 'analysis' && \(\s*<SettingsDetectionPanel/,
  'Analysis Settings must remain available when optional participants are disabled');
assert.doesNotMatch(settingsModal, /showAnalysis=/,
  'Functionality gates must not hide Analysis configuration');
assert.match(analysisSettings, /\{\(ocrEnabled \|\| transcriptionsEnabled\) && <AnalysisManagerRow[\s\S]{0,220}title="Extract"/,
  'Extractors must remain visible for either OCR or Transcriptions');
assert.match(analysisSettings, /\{\(contentDetectionEnabled \|\| typesEnabled\) && <AnalysisManagerRow[\s\S]{0,220}title="Detect"/,
  'Detectors must remain visible for either Content Detection or Types');
assert.match(settingsModal, /typesEnabled=\{settings\.enableTypes\}/,
  'Analysis Settings must receive the Types feature gate');
assert.match(settingsModal, /sourcesEnabled=\{settings\.enableSources\}/,
  'Analysis Settings must receive the Sources feature gate');
assert.match(analysisSettings, /step=\{1\}[\s\S]{0,220}title="Capture"/,
  'Capture must remain visible independently of optional presentation features');
assert.match(builtinLifecycleManager, /stableRef !== 'capture:source-attribution-v1'/,
  'Disabling Sources must hide Source Attribution without hiding Clip Type');
assert.match(clipCard, /features\.types[\s\S]{0,100}structuralClipType/,
  'Clip cards must fall back to structural Clip Type when Content Types is disabled');
assert.match(clipCard, /features\.sources && <span className="font-medium theme-text-main/,
  'Clip cards must hide Source chrome when Sources is disabled');
assert.match(clipPreview, /features\.types[\s\S]{0,180}structuralClipType/,
  'Clip Preview must fall back to structural Clip Type when Content Types is disabled');
assert.match(clipPreview, /features\.sources && <OverflowText text=\{clip\.source\}/,
  'Clip Preview must hide its Source label when Sources is disabled');
assert.match(analytics, /features\.sources && <div className="theme-panel[\s\S]{0,1000}Top source in History/,
  'Insights must hide Source summaries when Sources is disabled');
assert.match(analytics, /Clips by Clip Type/,
  'Insights must always present structural Clip Type summaries');
assert.match(analytics, /Clips by File Format/,
  'Insights must present file-format summaries separately');
assert.match(analytics, /features\.types && <div className="theme-panel[\s\S]{0,1000}Clips by Content Type/,
  'Insights must hide semantic Content Type summaries when Content Types is disabled');
assert.match(analysisSettings, /\{transformationsEnabled && <AnalysisManagerRow[\s\S]{0,220}title="Enrich"/,
  'Disabling Transformations must hide Smart Action Enrichers');
assert.match(analysisExecution, /let run_detectors = allow_text_participants && options\.include_detectors;/,
  'Enrichment must not implicitly enable Detectors');
assert.match(commands, /include_detectors: include_detectors\.unwrap_or\(true\)[\s\S]{0,100}Feature::ContentDetection/,
  'The live Analyzer command must honor the Content Detection feature gate');
assert.match(extractorManager, /inputContract !== 'image' \|\| ocrEnabled/,
  'Disabling OCR must hide image Extractors');
assert.match(extractorManager, /inputContract !== 'file_references' \|\| transcriptionsEnabled/,
  'Disabling Transcriptions must hide file transcription Extractors');
assert.match(extractorManager, /value: 'original_text', label: 'Text', disabled: true/,
  'Extractor settings must explain that Text clips are already searchable');
assert.match(extractorManager, /value: 'image', label: 'Image'/,
  'Extractor settings must present the Image Clip Type instead of its wire contract');
assert.match(extractorManager, /value: 'file_references', label: 'Files'/,
  'Extractor settings must present the Files Clip Type instead of its wire contract');
assert.match(extractorManager, /value: 'searchable_text', label: 'Searchable text'/,
  'Extractor settings must present the searchable-text output contract readably');
assert.doesNotMatch(extractorManager, />Pass<|>extract<\/strong>/,
  'Extractor settings must not repeat the enclosing Analysis step');
assert.match(analysisSettings, /<ContentExtractorManagerDialog[\s\S]{0,220}ocrEnabled=\{ocrEnabled\}[\s\S]{0,100}transcriptionsEnabled=\{transcriptionsEnabled\}/,
  'Analysis Settings must pass both extraction feature gates to Extractor management');
assert.match(commands, /extract_text_from_file_clip[\s\S]{0,220}Feature::Transcriptions/,
  'Native file transcription must enforce the Transcriptions feature gate');
assert.match(clipPreviewContent, /transcriptionsEnabled && <div className="theme-panel space-y-3 rounded-xl border p-4 shadow-lg">/,
  'Clip Preview must hide transcription controls when Transcriptions is disabled');
for (const command of [
  'restore_default_content_extractors',
  'restore_default_content_detectors',
  'restore_default_content_types',
  'restore_default_content_type_groups',
]) {
  assert.match(
    analysisSettings,
    new RegExp(`invoke(?:<[^>]+>)?\\('${command}'\\)`),
    `The global Analysis restore must include ${command}`,
  );
}
assert.match(analysisSettings, /<ActionButton onClick=\{restoreAnalysis\}[\s\S]{0,180}'Reset…'/,
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
  detector: fixture('detector-interactive-matched'),
  detectorNoMatch: fixture('detector-interactive-no-match'),
  enricher: fixture('enricher-interactive-empty'),
};

for (const [surface, value] of Object.entries(fixtures)) {
  assert.equal(value.formatVersion, 1, `${surface} fixture must use Analysis contract v1`);
  assert.ok(['capture', 'interactive'].includes(value.policy), `${surface} fixture must name its policy`);
  assert.equal(value.through, value.policy === 'capture' ? 'classify' : 'enrich',
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
  'enricher-interactive-empty',
  'extractor-interactive-unavailable',
  'detector-interactive-no-match',
]) {
  assert.ok(cliTests.includes(`${name}.json`), `CLI integration must consume ${name}.json`);
}

for (const field of ['structure', 'mediaMetadata', 'recommendations']) {
  assert.match(clipPreview, new RegExp(`result\\.${field}`),
    `Clip Preview must consume whole-Analyzer ${field}`);
}
assert.match(clipPreview, /Smart Actions/, 'Clip Preview must present Enricher recommendations contextually');
assert.match(clipPreviewContent, /Extracted by \{ocrExtractorLabel\}/,
  'Clip Preview must identify the Extractor that produced OCR text');
for (const field of ['ocr_extractor_ref', 'ocr_extractor_name', 'ocr_engine_version']) {
  assert.match(database, new RegExp(`pub ${field}: Option<String>`),
    `Shared ClipItem must expose OCR provenance field ${field}`);
  assert.match(types, new RegExp(`${field}\\?: string \\| null`),
    `Frontend ClipItem must expose OCR provenance field ${field}`);
}
for (const title of ['Capture', 'Inspect', 'Extract', 'Detect', 'Enrich']) {
  assert.match(analysisSettings, new RegExp(`title="${title}"`),
    `Analysis settings must expose ${title} behind a compact manager row`);
}
assert.match(builtinLifecycleManager, /get_library_items/,
  'Capture, Inspector, and Enricher managers must consume the shared registry');
assert.match(builtinLifecycleManager, /participantContract/,
  'Inspector and Enricher managers must render typed participant contracts');
assert.match(builtinLifecycleManager, /typeRelations/,
  'Inspector and Enricher managers must render registered Type relations');
assert.match(builtinLifecycleManager, /get_content_inspectors/,
  'Inspector management must load shared engine availability');
assert.match(builtinLifecycleManager, /Technical details/,
  'Internal participant contracts must remain behind contextual technical details');
assert.match(builtinLifecycleManager, /pasted \{kind\} get &lt;ref&gt; --json/,
  'Stable references must explain their CLI and API purpose');
for (const [label, manager] of [['Extractor', extractorManager], ['Detector', analysisSettings]]) {
  assert.equal((manager.match(/<RegistryPanelFooter/g) ?? []).length, 2,
    `${label} management must keep item actions and form actions in their owning panels`);
  assert.match(manager, /<AppDialogFooter[\s\S]*Reset…[\s\S]*Close/,
    `${label} management must keep scoped restore and close actions in the modal footer`);
  assert.match(manager, /discardDraftThen[\s\S]*ConfirmationDialog/,
    `${label} management must protect edited drafts with the shared confirmation UI`);
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
assert.match(architecture, /## Version 1 freeze/, 'Analysis architecture must declare the v1 freeze');
assert.match(releaseChecklist, /## Content Analysis/, 'The release checklist must retain Analysis acceptance');

console.log('Analysis JSON contract audit passed.');
