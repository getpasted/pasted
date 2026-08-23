import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(path, 'utf8');
const reference = JSON.parse(read('docs/baselines/analysis-macos-arm64.json'));
const harness = read('src-tauri/examples/analysis_baseline.rs');
const packageJson = JSON.parse(read('package.json'));
const extractorRuntime = read('src-tauri/src/db/extractors/runtime.rs');
const extractorRecipeRuntime = read('src-tauri/src/extractor_recipe/runtime_status.rs');
const extractorRuntimeCommands = read('src-tauri/src/commands/extractors/runtime.rs');
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

assert.match(extractorRuntime, /runtime_status_summary\(&recipe\)/,
  'Ordinary Extractor reads must use non-blocking runtime summaries');
assert.doesNotMatch(extractorRuntime, /probe_version|runtime_status\(&recipe\)/,
  'Ordinary Extractor reads must never launch external version probes');
assert.match(extractorRecipeRuntime, /build_runtime_status\(recipe, false\)/,
  'Extractor runtime summaries must explicitly disable version probes');
assert.match(extractorRuntimeCommands,
  /pub async fn get_content_extractor_runtime[\s\S]*spawn_blocking[\s\S]*inspect_extractor_runtime/,
  'Detailed Extractor runtime inspection must remain an explicit background command');
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
