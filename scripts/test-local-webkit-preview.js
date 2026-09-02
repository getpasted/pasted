import assert from 'node:assert/strict';
import fs from 'node:fs';
import { spawnSync } from 'node:child_process';

const scriptPath = 'scripts/run-with-local-webkit.sh';
const source = fs.readFileSync(scriptPath, 'utf8');
const previewSource = fs.readFileSync('src-tauri/src/local_webkit_preview.rs', 'utf8');

const syntax = spawnSync('bash', ['-n', scriptPath], { encoding: 'utf8' });
assert.equal(syntax.status, 0, syntax.stderr);

const help = spawnSync('bash', [scriptPath, '--help'], { encoding: 'utf8' });
assert.equal(help.status, 0, help.stderr);
assert.match(help.stdout, /WebKitBuild\/Release/);
assert.match(help.stdout, /vmmap confirms/);
assert.match(help.stdout, /temporary, seeded/);

for (const variable of [
  'DYLD_FRAMEWORK_PATH',
  '__XPC_DYLD_FRAMEWORK_PATH',
  'DYLD_LIBRARY_PATH',
  '__XPC_DYLD_LIBRARY_PATH',
]) {
  assert.match(source, new RegExp(`\\b${variable}=`), `${variable} must remain in the preview process environment`);
}

assert.match(source, /cargo build --locked/, 'The preview must build from the reviewed Rust lockfile');
assert.match(source, /--no-default-features --features cli --bin pasted/, 'The preview must build the CLI used to seed its isolated database');
assert.match(source, /npm run dev -- --host 127\.0\.0\.1/, 'The preview must bind Vite to loopback');
assert.match(source, /vmmap "\$app_pid"/, 'The preview must inspect the launched Pasted process');
assert.match(source, /mktemp -d/, 'The preview must isolate its demonstration database in a temporary directory');
assert.match(source, /PASTED_PREVIEW_DATABASE_PATH="\$preview_database"/, 'The GUI must use the isolated preview database');
assert.match(source, /PASTED_DATABASE_PATH="\$preview_database"/, 'The CLI must seed the same isolated preview database');
assert.match(source, /rm -rf -- "\$preview_root"/, 'The temporary preview database must be removed during cleanup');
assert.match(source, /kill "\$app_pid"/, 'Interrupted previews must stop the launched application');
assert.match(source, /kill "\$vite_pid"/, 'Interrupted previews must stop the Vite server');
assert.match(previewSource, /#\[cfg\(debug_assertions\)\][\s\S]*PASTED_PREVIEW_DATABASE_PATH/, 'Only debug builds may honor the preview database override');
assert.match(previewSource, /#\[cfg\(not\(debug_assertions\)\)\][\s\S]*None/, 'Release builds must ignore the preview database override');
assert.match(previewSource, /is_absolute\(\)[\s\S]*pasted\.db/, 'The debug override must reject relative or unexpected database paths');
assert.match(previewSource, /canonical_parent\.parent\(\)[\s\S]*pasted-local-webkit\./, 'The debug override must stay inside a script-managed temporary directory');
assert.match(previewSource, /file_type\(\)\.is_symlink\(\)/, 'The debug override must reject database symlinks');

console.log('Local WebKit preview launcher checks passed.');
