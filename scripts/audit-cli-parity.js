import assert from 'node:assert/strict';
import fs from 'node:fs';

const read = (path) => fs.readFileSync(path, 'utf8');
const cli = read('src-tauri/src/bin/pasted_cli.rs');
const help = read('src/components/HelpView.tsx');
const database = read('src-tauri/src/db.rs');
const commands = read('src-tauri/src/commands.rs');
const actions = read('src/hooks/useClipActions.ts');

const documentedCommands = [
  'pasted copy',
  'pasted list',
  'pasted search',
  'pasted clear',
  'pasted clip get',
  'pasted clip pin|unpin',
  'pasted clip protect|unprotect',
  'pasted clip trash|restore',
  'pasted clip assign',
  'pasted bin list',
  'pasted bin clips',
  'pasted bin order',
  'pasted transform list',
  'pasted transform run',
  'pasted operation list',
  'pasted operation run',
  'pasted pipeline list',
  'pasted pipeline run',
  'pasted diagnostics',
  'pasted type list',
  'pasted type create',
  'pasted type archive|restore',
  'pasted type group-list',
  'pasted type group-create',
  'pasted type group-archive|group-restore',
  'pasted type group-delete',
  'pasted detector list',
  'pasted detector rescan',
  'pasted ocr status',
  'pasted ocr scan',
  'pasted reset',
];

for (const command of documentedCommands) {
  assert.ok(help.includes(command), `Help & Docs must document ${command}`);
}
assert.doesNotMatch(help, /pasted-cli/, 'Help & Docs must expose the stable pasted command, not an implementation alias');

for (const route of ['copy', 'list', 'search', 'clear', 'clip', 'bin', 'transform', 'operation', 'pipeline', 'type', 'detector', 'diagnostics', 'ocr', 'reset']) {
  assert.match(cli, new RegExp(`"${route}"`), `The CLI must retain its ${route} route`);
}

for (const mutation of ['batch_pin_clips', 'batch_protect_clips', 'batch_trash_clips']) {
  assert.match(database, new RegExp(`pub fn ${mutation}`), `${mutation} must live in the shared database domain layer`);
  assert.match(commands, new RegExp(`pub fn ${mutation}`), `${mutation} must be exposed to the GUI`);
  assert.match(cli, new RegExp(`db\\.${mutation}`), `${mutation} must be reused by the CLI`);
}

assert.match(commands, /bin_assignment::assign_clips_to_bin/, 'GUI Bin assignment must use the shared workflow');
assert.match(cli, /assign_clips_to_bin/, 'CLI Bin assignment must use the shared workflow, including attached Transforms');

assert.match(actions, /invoke\('batch_protect_clips'/, 'GUI batch protection must be one explicit mutation, not a loop of toggles');
assert.doesNotMatch(actions, /Promise\.all\(idsToChange\.map\(\(clipId\) => invoke\('toggle_clip_protected'/, 'GUI batch protection must not race toggle calls');
assert.match(database, /pub struct ClipMutationSummary/, 'GUI and CLI mutations must share a stable result contract');
assert.match(commands, /db\.rescan_content_detection\(\)/, 'GUI history rescans must use the shared detector domain service');
assert.match(cli, /db\.rescan_content_detection\(\)/, 'CLI history rescans must use the shared detector domain service');
for (const method of ['get_content_types', 'create_content_type', 'update_content_type', 'set_content_type_archived', 'restore_default_content_types']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(commands, new RegExp(`pub async fn ${method}|pub fn ${method}`), `${method} must be exposed to the GUI`);
  assert.match(cli, new RegExp(`db\\s*\\.${method}`), `${method} must be reused by the CLI`);
}
for (const method of ['get_content_type_groups', 'create_content_type_group', 'update_content_type_group', 'set_content_type_group_archived', 'delete_content_type_group', 'restore_default_content_type_groups']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(commands, new RegExp(`pub fn ${method}`), `${method} must be exposed to the GUI`);
  assert.match(cli, new RegExp(`db\\s*\\.${method}`), `${method} must be reused by the CLI`);
}

console.log('GUI and CLI parity audit passed.');
