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
assert.doesNotMatch(frontendSource, /dangerouslySetInnerHTML/, 'Render untrusted clip content as text, never raw HTML');
assert.doesNotMatch(frontendSource, /\b(?:eval|Function)\s*\(/, 'Frontend dynamic code execution is forbidden');

console.log('Security configuration and frontend trust-boundary audit passed.');
