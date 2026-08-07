import assert from 'node:assert/strict';
import fs from 'node:fs';

const read = (path) => fs.readFileSync(path, 'utf8');
const cli = read('src-tauri/src/bin/pasted_cli.rs');
const help = read('src/components/HelpView.tsx');
const database = read('src-tauri/src/db.rs');
const commands = read('src-tauri/src/commands.rs');
const actions = read('src/hooks/useClipActions.ts');

const documentedCommands = [
  'pasted-cli copy',
  'pasted-cli list',
  'pasted-cli search',
  'pasted-cli clear',
  'pasted-cli clip get',
  'pasted-cli clip pin|unpin',
  'pasted-cli clip protect|unprotect',
  'pasted-cli clip trash|restore',
  'pasted-cli clip assign',
  'pasted-cli bin list',
  'pasted-cli bin clips',
  'pasted-cli bin order',
  'pasted-cli transform list',
  'pasted-cli transform run',
  'pasted-cli operation list',
  'pasted-cli operation run',
  'pasted-cli pipeline list',
  'pasted-cli pipeline run',
  'pasted-cli diagnostics',
  'pasted-cli ocr status',
  'pasted-cli ocr scan',
  'pasted-cli reset',
];

for (const command of documentedCommands) {
  assert.ok(help.includes(command), `Help & Docs must document ${command}`);
}

for (const route of ['copy', 'list', 'search', 'clear', 'clip', 'bin', 'transform', 'operation', 'pipeline', 'diagnostics', 'ocr', 'reset']) {
  assert.match(cli, new RegExp(`"${route}"`), `The CLI must retain its ${route} route`);
}

for (const mutation of ['batch_pin_clips', 'batch_protect_clips', 'batch_trash_clips']) {
  assert.match(database, new RegExp(`pub fn ${mutation}`), `${mutation} must live in the shared database domain layer`);
  assert.match(commands, new RegExp(`pub fn ${mutation}`), `${mutation} must be exposed to the GUI`);
  assert.match(cli, new RegExp(`db\.${mutation}`), `${mutation} must be reused by the CLI`);
}

assert.match(commands, /bin_assignment::assign_clips_to_bin/, 'GUI Bin assignment must use the shared workflow');
assert.match(cli, /assign_clips_to_bin/, 'CLI Bin assignment must use the shared workflow, including attached Transforms');

assert.match(actions, /invoke\('batch_protect_clips'/, 'GUI batch protection must be one explicit mutation, not a loop of toggles');
assert.doesNotMatch(actions, /Promise\.all\(idsToChange\.map\(\(clipId\) => invoke\('toggle_clip_protected'/, 'GUI batch protection must not race toggle calls');
assert.match(database, /pub struct ClipMutationSummary/, 'GUI and CLI mutations must share a stable result contract');

console.log('GUI and CLI parity audit passed.');
