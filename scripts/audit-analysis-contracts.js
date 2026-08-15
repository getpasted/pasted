import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(path, 'utf8');
const fixture = (name) => JSON.parse(read(`contracts/analysis/v1/${name}.json`));
const frontendMock = read('src/utils/tauri.ts');
const cliTests = read('src-tauri/tests/cli_integration.rs');

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

console.log('Analysis JSON contract audit passed.');
