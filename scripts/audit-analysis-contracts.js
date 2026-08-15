import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(path, 'utf8');
const fixture = (name) => JSON.parse(read(`contracts/analysis/v1/${name}.json`));
const frontendMock = read('src/utils/tauri.ts');
const cliTests = read('src-tauri/tests/cli_integration.rs');
const clipPreview = read('src/components/ClipPreview.tsx');
const clipPreviewContent = read('src/components/ClipPreviewContent.tsx');
const database = read('src-tauri/src/db.rs');
const types = read('src/types.ts');
const analysisSettings = read('src/components/SettingsDetectionPanel.tsx');
const extractorManager = read('src/components/ContentExtractorManagerDialog.tsx');
const registryPanelHeader = read('src/components/RegistryPanelHeader.tsx');
const registryPanelFooter = read('src/components/RegistryPanelFooter.tsx');
const architecture = read('docs/ANALYSIS_ARCHITECTURE.md');
const releaseChecklist = read('docs/RELEASE_CHECKLIST_1.0.0.md');

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

for (const field of ['structure', 'recommendations']) {
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
assert.match(analysisSettings, /title="Extractors"/, 'Analysis settings must expose authorable Extractors');
assert.match(analysisSettings, /title="Detectors"/, 'Analysis settings must expose authorable Detectors');
assert.doesNotMatch(analysisSettings, /Manage (?:Inspectors|Enrichers)/,
  'Immutable built-in participants must not gain redundant management surfaces');
for (const [label, manager] of [['Extractor', extractorManager], ['Detector', analysisSettings]]) {
  assert.equal((manager.match(/<RegistryPanelFooter/g) ?? []).length, 2,
    `${label} management must keep item actions and form actions in their owning panels`);
  assert.match(manager, /<AppDialogFooter[\s\S]*Restore Shipped Defaults…[\s\S]*Close/,
    `${label} management must keep global restore and dialog close actions in the modal footer`);
  assert.match(manager, /discardDraftThen[\s\S]*ConfirmationDialog/,
    `${label} management must protect edited drafts with the shared confirmation UI`);
  assert.doesNotMatch(manager, /window\.confirm/,
    `${label} management must not fall back to a browser confirmation prompt`);
}
assert.match(registryPanelHeader, /min-h-\[49px\] shrink-0/,
  'Registry panel headers must match the rendered action-header height');
assert.match(registryPanelFooter, /min-h-12 shrink-0/,
  'Registry panel footers must retain aligned minimum heights with and without actions');
assert.match(architecture, /## Version 1 freeze/, 'Analysis architecture must declare the v1 freeze');
assert.match(releaseChecklist, /## Content Analysis/, 'The release checklist must retain Analysis acceptance');

console.log('Analysis JSON contract audit passed.');
