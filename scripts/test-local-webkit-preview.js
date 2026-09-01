import assert from 'node:assert/strict';
import fs from 'node:fs';
import { spawnSync } from 'node:child_process';

const scriptPath = 'scripts/run-with-local-webkit.sh';
const source = fs.readFileSync(scriptPath, 'utf8');

const syntax = spawnSync('bash', ['-n', scriptPath], { encoding: 'utf8' });
assert.equal(syntax.status, 0, syntax.stderr);

const help = spawnSync('bash', [scriptPath, '--help'], { encoding: 'utf8' });
assert.equal(help.status, 0, help.stderr);
assert.match(help.stdout, /WebKitBuild\/Release/);
assert.match(help.stdout, /vmmap confirms/);

for (const variable of [
  'DYLD_FRAMEWORK_PATH',
  '__XPC_DYLD_FRAMEWORK_PATH',
  'DYLD_LIBRARY_PATH',
  '__XPC_DYLD_LIBRARY_PATH',
]) {
  assert.match(source, new RegExp(`\\b${variable}=`), `${variable} must remain in the preview process environment`);
}

assert.match(source, /cargo build --locked/, 'The preview must build from the reviewed Rust lockfile');
assert.match(source, /npm run dev -- --host 127\.0\.0\.1/, 'The preview must bind Vite to loopback');
assert.match(source, /vmmap "\$app_pid"/, 'The preview must inspect the launched Pasted process');
assert.match(source, /kill "\$app_pid"/, 'Interrupted previews must stop the launched application');
assert.match(source, /kill "\$vite_pid"/, 'Interrupted previews must stop the Vite server');

console.log('Local WebKit preview launcher checks passed.');
