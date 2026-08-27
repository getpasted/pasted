import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

const root = fs.mkdtempSync(path.join(os.tmpdir(), 'pasted-updater-manifest-'));
const assets = path.join(root, 'assets');
fs.mkdirSync(assets);
for (const name of ['Pasted.app.tar.gz', 'Pasted.AppImage', 'Pasted-setup.exe']) {
  fs.writeFileSync(path.join(assets, name), name);
  fs.writeFileSync(path.join(assets, `${name}.sig`), `signature-${name}`);
}
const notes = path.join(root, 'notes.md');
const output = path.join(root, 'latest.json');
fs.writeFileSync(notes, 'A signed update.');
execFileSync(process.execPath, [
  'scripts/render-updater-manifest.js',
  '--version', '1.0.0-rc.6',
  '--tag', 'v1.0.0-rc.6',
  '--asset-root', assets,
  '--output', output,
  '--notes-file', notes,
  '--published-at', '2026-08-27T00:00:00Z',
]);
const manifest = JSON.parse(fs.readFileSync(output, 'utf8'));
assert.equal(manifest.version, '1.0.0-rc.6');
assert.equal(manifest.notes, 'A signed update.');
assert.deepEqual(Object.keys(manifest.platforms), [
  'darwin-aarch64',
  'darwin-x86_64',
  'linux-x86_64',
  'windows-x86_64',
]);
assert.equal(manifest.platforms['darwin-aarch64'].signature, 'signature-Pasted.app.tar.gz');
assert.match(manifest.platforms['windows-x86_64'].url, /v1\.0\.0-rc\.6\/Pasted-setup\.exe$/);
console.log('Updater manifest tests passed.');
