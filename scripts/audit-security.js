import assert from 'node:assert/strict';
import fs from 'node:fs';

const readJson = (path) => JSON.parse(fs.readFileSync(path, 'utf8'));
const readFilesRecursively = (directory, extensions) => fs.readdirSync(directory, { withFileTypes: true })
  .flatMap((entry) => {
    const path = `${directory}/${entry.name}`;
    if (entry.isDirectory()) return readFilesRecursively(path, extensions);
    return extensions.some((extension) => entry.name.endsWith(extension))
      ? [fs.readFileSync(path, 'utf8')]
      : [];
  });

const tauriConfig = readJson('src-tauri/tauri.conf.json');
const capability = readJson('src-tauri/capabilities/default.json');
const packageJson = readJson('package.json');
const frontendSource = readFilesRecursively('src', ['.ts', '.tsx']).join('\n');
const rustSource = readFilesRecursively('src-tauri/src', ['.rs']).join('\n');
const cargoToml = fs.readFileSync('src-tauri/Cargo.toml', 'utf8');
const security = tauriConfig.app?.security;

assert.ok(security?.csp, 'Production Tauri CSP must remain enabled');
assert.equal(security.freezePrototype, true, 'Tauri must freeze Object.prototype in packaged webviews');
assert.match(security.csp['default-src'], /'self'/, 'CSP default-src must be self-restricted');
assert.match(security.csp['connect-src'], /ipc:/, 'CSP must permit Tauri IPC');
assert.equal(security.csp['object-src'], "'none'", 'CSP must block embedded objects');
assert.equal(security.csp['base-uri'], "'none'", 'CSP must block base URL rewriting');
assert.equal(security.csp['frame-src'], "'none'", 'CSP must block framed content');

assert.ok(!capability.permissions.includes('opener:default'), 'Unused opener permission must not return');
assert.ok(!packageJson.dependencies?.['@tauri-apps/plugin-opener'], 'Unused opener dependency must not return');
assert.ok(
  !capability.permissions.some((permission) => permission.startsWith('shell:')),
  'The webview must not receive Tauri shell permissions',
);
assert.ok(!packageJson.dependencies?.['@tauri-apps/plugin-shell'], 'The frontend must not gain shell access');
assert.doesNotMatch(cargoToml, /tauri-plugin-shell/, 'The backend must not enable the Tauri shell plugin');
assert.match(
  rustSource,
  /SQLITE_DBCONFIG_DEFENSIVE/,
  'SQLite connections must retain defensive mode as a second layer behind bound parameters',
);
assert.doesNotMatch(frontendSource, /dangerouslySetInnerHTML/, 'Render untrusted clip content as text, never raw HTML');
assert.doesNotMatch(frontendSource, /\b(?:eval|Function)\s*\(/, 'Frontend dynamic code execution is forbidden');
assert.doesNotMatch(
  rustSource,
  /Command::new\(\s*"(?:\/[^"\s]+\/)?(?:ba|z|fi)?sh"\s*\)/,
  'Never restore a general-purpose shell interpreter to a transformation path',
);
assert.doesNotMatch(
  rustSource,
  /vaultPasscodeHash|set_vault_passcode|verify_vault_passcode/,
  'Do not expose the removed fast-hash passcode API without a reviewed credential design',
);

console.log('Security configuration, process, and frontend trust-boundary audit passed.');
