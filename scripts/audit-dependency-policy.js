import assert from 'node:assert/strict';
import fs from 'node:fs';

const readJson = (file) => JSON.parse(fs.readFileSync(file, 'utf8'));
const policy = readJson('dependency-policy.json');
const inventory = readJson('THIRD_PARTY_LICENSES.json');
const packageJson = readJson('package.json');
const cargoToml = fs.readFileSync('src-tauri/Cargo.toml', 'utf8');
const denyToml = fs.readFileSync('deny.toml', 'utf8');
const tauriConfig = readJson('src-tauri/tauri.conf.json');

assert.equal(policy.schemaVersion, 1, 'Unsupported dependency policy schema');
assert.ok(policy.approvedLicenses.length > 0, 'Dependency policy must approve licenses explicitly');
for (const component of policy.packagingComponents) {
  assert.ok(policy.approvedLicenses.includes(component.license), `${component.name} has an unapproved packaging license`);
  assert.ok(component.version && component.repository, `${component.name} packaging provenance is incomplete`);
  assert.ok(component.noticeSourceComponent, `${component.name} must identify its shipped license notice`);
}

const forbiddenLicense = new RegExp(
  policy.forbiddenLicenseTerms.map((term) => term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|'),
  'i',
);
for (const component of inventory.components) {
  assert.doesNotMatch(
    component.license,
    forbiddenLicense,
    `${component.name}@${component.version} uses a forbidden or unknown license: ${component.license}`,
  );
  if (component.ecosystem === 'npm') {
    assert.ok(
      policy.approvedNpmExpressions.includes(component.license),
      `${component.name}@${component.version} introduces an unreviewed npm license expression: ${component.license}`,
    );
  }
}

const componentNames = new Set(inventory.components.map(({ name }) => name.toLowerCase()));
for (const forbidden of policy.forbiddenDependencyNames) {
  assert.ok(
    !componentNames.has(forbidden.toLowerCase()),
    `Forbidden telemetry or tracking dependency detected: ${forbidden}`,
  );
}

const directNpm = new Set(Object.keys(packageJson.dependencies ?? {}));
const cargoSections = cargoToml.split(/^\[/m).filter((section) => section.startsWith('dependencies]') || /\.dependencies\]/.test(section.split('\n', 1)[0]));
const directCargo = new Set(cargoSections.flatMap((section) => [...section.matchAll(/^([A-Za-z0-9_-]+)\s*=/gm)].map((match) => match[1])));
const directDependencies = new Set([...directNpm, ...directCargo]);
const approvedNetwork = new Set(policy.approvedNetworkDependencies);
for (const dependency of policy.networkCapableDirectDependencies) {
  assert.ok(
    !directDependencies.has(dependency) || approvedNetwork.has(dependency),
    `Network-capable direct dependency requires explicit mission-policy approval: ${dependency}`,
  );
}

const connectSrc = tauriConfig.app?.security?.csp?.['connect-src'] ?? '';
const allowedConnectSources = new Set(["'self'", 'ipc:', 'http://ipc.localhost']);
const unreviewedConnectSources = connectSrc.split(/\s+/).filter((source) => source && !allowedConnectSources.has(source));
assert.deepEqual(unreviewedConnectSources, [], 'Production CSP must not permit unsolicited remote webview connections');

const today = new Date().toISOString().slice(0, 10);
for (const advisory of policy.ignoredRustAdvisories) {
  assert.match(advisory.id, /^RUSTSEC-\d{4}-\d{4}$/, `Invalid RustSec advisory ID: ${advisory.id}`);
  assert.ok(advisory.reason.length >= 40, `Ignored advisory ${advisory.id} needs a substantive reason`);
  assert.ok(advisory.expires >= today, `Ignored advisory ${advisory.id} expired on ${advisory.expires}`);
  assert.ok(denyToml.includes(advisory.id), `Ignored advisory ${advisory.id} is missing from deny.toml`);
}
const denyAdvisories = [...denyToml.matchAll(/RUSTSEC-\d{4}-\d{4}/g)].map((match) => match[0]);
for (const advisory of new Set(denyAdvisories)) {
  assert.ok(
    policy.ignoredRustAdvisories.some(({ id }) => id === advisory),
    `deny.toml advisory exception ${advisory} lacks a reviewed expiry in dependency-policy.json`,
  );
}

assert.match(
  fs.readFileSync('src/components/AnalyticsView.tsx', 'utf8'),
  /invoke<AnalyticsSummary>\('get_analytics_summary'\)/,
  'Analytics & Insights must remain an on-device database query',
);

console.log(
  `Dependency policy audit passed for ${inventory.componentCount} components, `
  + `${policy.approvedNpmExpressions.length} npm license expressions, and `
  + `${policy.ignoredRustAdvisories.length} expiring advisory exception.`,
);
