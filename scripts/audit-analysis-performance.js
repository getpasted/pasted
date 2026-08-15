import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(path, 'utf8');
const reference = JSON.parse(read('docs/baselines/analysis-macos-arm64.json'));
const harness = read('src-tauri/examples/analysis_baseline.rs');
const packageJson = JSON.parse(read('package.json'));

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

console.log('Analysis performance baseline audit passed.');
