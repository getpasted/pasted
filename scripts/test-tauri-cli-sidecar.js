import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import {
  executableExtension,
  sidecarFilename,
  stageCliSidecar,
} from './stage-tauri-cli-sidecar.js';

assert.equal(executableExtension('linux'), '');
assert.equal(executableExtension('darwin'), '');
assert.equal(executableExtension('win32'), '.exe');
assert.equal(sidecarFilename('x86_64-unknown-linux-gnu', 'linux'), 'pasted-x86_64-unknown-linux-gnu');
assert.equal(sidecarFilename('x86_64-pc-windows-msvc', 'win32'), 'pasted-x86_64-pc-windows-msvc.exe');

const root = fs.mkdtempSync(path.join(os.tmpdir(), 'pasted-sidecar-test-'));
const targetDir = path.join(root, 'custom-target');
const source = path.join(targetDir, 'release', 'pasted');
fs.mkdirSync(path.dirname(source), { recursive: true });
fs.writeFileSync(source, 'headless-cli');
fs.chmodSync(source, 0o755);

const staged = stageCliSidecar({
  root,
  targetDir,
  targetTriple: 'x86_64-unknown-linux-gnu',
  platform: 'linux',
});
assert.equal(staged.destination, path.join(root, 'src-tauri', 'binaries', 'pasted-x86_64-unknown-linux-gnu'));
assert.equal(fs.readFileSync(staged.destination, 'utf8'), 'headless-cli');
assert.ok((fs.statSync(staged.destination).mode & 0o111) !== 0, 'Unix sidecar must remain executable');

console.log('Tauri CLI sidecar staging tests passed.');
