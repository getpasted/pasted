import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(path, 'utf8');
const reference = JSON.parse(read('docs/baselines/analysis-macos-arm64.json'));
const harness = read('src-tauri/examples/analysis_baseline.rs');
const packageJson = JSON.parse(read('package.json'));
const extractorRuntime = read('src-tauri/src/db/extractors/runtime.rs');
const extractionCommands = read('src-tauri/src/commands/extraction/ocr_backfill.rs');
const ocrSettings = read('src/components/SettingsOcrPanel.tsx');
const externalTools = read('src-tauri/src/external_tools.rs');

assert.equal(reference.formatVersion, 1, 'Analysis performance reference must be versioned');
assert.equal(reference.environment.profile, 'release', 'Analysis baselines must use release builds');
assert.ok(reference.measurements.length >= 4, 'Analysis baseline must cover every core workload');

for (const measurement of reference.measurements) {
  assert.match(measurement.name, /^[a-z0-9_]+$/, 'Benchmark names must be stable and scriptable');
  assert.ok(measurement.iterationsPerSample > 0, `${measurement.name} must record iterations`);
  assert.ok(measurement.samples >= 5, `${measurement.name} must record multiple samples`);
  assert.ok(measurement.medianNsPerIteration > 0, `${measurement.name} must record a median`);
  assert.ok(measurement.p95NsPerIteration >= measurement.medianNsPerIteration,
    `${measurement.name} p95 must not be lower than its median`);
  assert.ok(harness.includes(`"${measurement.name}"`),
    `${measurement.name} must remain implemented by the benchmark harness`);
}

assert.match(packageJson.scripts['bench:analysis'], /cargo run --release[\s\S]*--example analysis_baseline/,
  'Analysis benchmark command must use the release-mode portable harness');
assert.doesNotMatch(packageJson.scripts['test:all'], /bench:analysis/,
  'Wall-clock Analysis benchmarks must remain outside ordinary correctness CI');

const storedExtractorLoad = extractorRuntime.indexOf('let stored = {');
const runtimeDecoration = extractorRuntime.indexOf('runtime_status(&recipe)');
assert.ok(storedExtractorLoad >= 0 && runtimeDecoration > storedExtractorLoad,
  'Extractor persistence must load records before decorating runtime status');
assert.match(
  extractorRuntime.slice(storedExtractorLoad, runtimeDecoration),
  /let stored = rows\.collect::<Result<Vec<_>>>\(\)\?;[\s\S]*drop\(conn\);[\s\S]*};[\s\S]*stored/,
  'The SQLite connection scope must end before external Extractor runtime probes begin',
);
assert.match(extractionCommands,
  /pub async fn get_ocr_backfill_status[\s\S]*spawn_blocking/,
  'OCR status database reads must not block the Tauri event loop');
assert.match(ocrSettings, /if \(polling\) return;/,
  'OCR status polling must reject overlapping requests');
assert.equal((ocrSettings.match(/listExtractors<ContentExtractor>/g) ?? []).length, 2,
  'Extractor runtime discovery must load on mount and explicit refresh, not every status tick');
assert.match(externalTools,
  /entry\.value\.is_some\(\) \|\| entry\.checked_at\.elapsed\(\) < FAILED_VERSION_PROBE_TTL/,
  'Failed external-tool version probes must be cached with a bounded retry interval');

console.log('Analysis performance baseline audit passed.');
