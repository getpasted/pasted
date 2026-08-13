import assert from 'node:assert/strict';
import fs from 'node:fs';

const read = (path) => fs.readFileSync(path, 'utf8');
const cli = read('src-tauri/src/bin/pasted_cli.rs');
const help = read('src/components/HelpView.tsx');
const database = read('src-tauri/src/db.rs');
const commands = read('src-tauri/src/commands.rs');
const actions = read('src/hooks/useClipActions.ts');
const storageSettings = read('src/components/SettingsSyncPanel.tsx');

const documentedCommands = [
  'pasted copy',
  'pasted list',
  'pasted search',
  'pasted import',
  'pasted retention',
  'pasted activity list',
  'pasted activity export',
  'pasted activity import',
  'pasted activity clear',
  'pasted transfer export',
  'pasted transfer inspect',
  'pasted transfer import',
  'pasted database location',
  'pasted database move',
  'pasted database default',
  'pasted backup create',
  'pasted backup restore',
  'pasted clear',
  'pasted clip get',
  'pasted clip export',
  'pasted clip import',
  'pasted clip pin|unpin',
  'pasted clip protect|unprotect',
  'pasted clip trash|restore',
  'pasted clip restore-all',
  'pasted clip assign',
  'pasted bin list',
  'pasted bin clips',
  'pasted bin order',
  'pasted transform list',
  'pasted transform run',
  'pasted operation list',
  'pasted operation run',
  'pasted diagnostics',
  'pasted licenses',
  'pasted type list',
  'pasted type create',
  'pasted type archive|restore',
  'pasted type group-list',
  'pasted type group-create',
  'pasted type group-archive|group-restore',
  'pasted type group-delete',
  'pasted registry',
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

for (const route of ['copy', 'list', 'search', 'import', 'retention', 'activity', 'transfer', 'archive', 'backup', 'clear', 'clip', 'bin', 'transform', 'operation', 'database', 'library', 'registry', 'type', 'detector', 'diagnostics', 'licenses', 'ocr', 'reset']) {
  assert.match(cli, new RegExp(`"${route}"`), `The CLI must retain its ${route} route`);
}
for (const method of ['export_backup_json', 'inspect_library_archive_json', 'import_backup_json']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(cli, new RegExp(`(?:db\.|DbState::)${method}`), `${method} must be reused by the CLI`);
}
assert.match(commands, /pub fn inspect_library_archive_json/, 'The GUI must expose portable-transfer preflight');
for (const method of ['create_full_backup', 'restore_full_backup']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(commands, new RegExp(`\\w+\\s*\\.\\s*${method}`), `${method} must be reused by the GUI`);
  assert.match(cli, new RegExp(`db\.${method}`), `${method} must be reused by the CLI`);
}
for (const method of ['export_activity_json', 'export_activity_csv', 'import_activity_json', 'import_activity_csv']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(commands, new RegExp(`pub fn ${method}`), `${method} must be exposed to the GUI`);
  assert.match(cli, new RegExp(`db\\.${method}`), `${method} must be reused by the CLI`);
  if (method.startsWith('export_')) {
    assert.match(storageSettings, new RegExp(`['"]${method}['"]`), `${method} must be reachable from Storage`);
  }
}
for (const method of ['export_clips_json', 'export_clips_csv', 'import_clips_json', 'import_clips_csv']) {
  assert.match(database, new RegExp(`pub fn ${method}`), `${method} must live in the shared database domain layer`);
  assert.match(commands, new RegExp(`pub fn ${method}`), `${method} must be exposed to the GUI`);
  assert.match(cli, new RegExp(`db\\.${method}`), `${method} must be reused by the CLI`);
}
assert.match(storageSettings, /['"]import_inspected_file['"]/, 'Validated clip and Activity imports must be reachable from Storage');
for (const method of ['import_activity_json', 'import_activity_csv', 'import_clips_json', 'import_clips_csv']) {
  assert.match(commands, new RegExp(`db\.${method}`), `${method} must be reused by the inspected-import GUI command`);
}

assert.match(cli, /if matches!\(command, "licenses" \| "license"\)/, 'Legal notices must be available before database initialization');
assert.match(commands, /pub fn get_third_party_licenses/, 'The GUI must expose the shared generated license document');

for (const mutation of ['batch_pin_clips', 'batch_protect_clips', 'batch_trash_clips', 'restore_all_trashed_clips']) {
  assert.match(database, new RegExp(`pub fn ${mutation}`), `${mutation} must live in the shared database domain layer`);
  assert.match(commands, new RegExp(`pub fn ${mutation}`), `${mutation} must be exposed to the GUI`);
  assert.match(cli, new RegExp(`db\\.${mutation}`), `${mutation} must be reused by the CLI`);
}

assert.match(commands, /bin_assignment::assign_clips_to_bin/, 'GUI Bin assignment must use the shared workflow');
assert.match(cli, /assign_clips_to_bin/, 'CLI Bin assignment must use the shared workflow, including attached Transforms');

assert.match(actions, /invoke\('batch_protect_clips'/, 'GUI batch protection must be one explicit mutation, not a loop of toggles');
assert.match(read('src/App.tsx'), /invoke<ClipMutationSummary>\('restore_all_trashed_clips'\)/,
  'GUI bulk recovery must use the shared restore-all mutation');
assert.doesNotMatch(actions, /Promise\.all\(idsToChange\.map\(\(clipId\) => invoke\('toggle_clip_protected'/, 'GUI batch protection must not race toggle calls');
assert.match(database, /pub struct ClipMutationSummary/, 'GUI and CLI mutations must share a stable result contract');
assert.match(commands, /pub async fn import_external_history/, 'GUI migration must use the shared external import service');
assert.match(cli, /external_import::import_history/, 'CLI migration must use the shared external import service');
assert.match(database, /pub fn configure_clip_retention/, 'Retention policy must live in the shared database domain layer');
assert.match(commands, /db\.enforce_clip_retention/, 'GUI retention must use the shared domain policy');
assert.match(cli, /db\.configure_clip_retention/, 'CLI retention must use the shared domain policy');
for (const scope of ['trash', 'activity']) {
  assert.match(database, new RegExp(`pub fn configure_${scope}_retention`), `${scope} retention must live in the shared database domain layer`);
  assert.match(commands, new RegExp(`db\\.enforce_${scope}_retention`), `GUI ${scope} retention must use the shared domain policy`);
  assert.match(cli, new RegExp(`db\\.configure_${scope}_retention`), `CLI ${scope} retention must use the shared domain policy`);
}
assert.match(commands, /db\.rescan_content_detection\(\)/, 'GUI history rescans must use the shared detector domain service');
assert.match(cli, /db\.rescan_content_detection\(\)/, 'CLI history rescans must use the shared detector domain service');
assert.match(commands, /db\.get_library_items/, 'GUI library metadata must use the shared domain service');
assert.match(cli, /db\.get_library_items/, 'CLI library metadata must use the shared domain service');
assert.match(commands, /db\.set_library_item_enabled/, 'GUI lifecycle toggles must use the shared domain service');
assert.match(cli, /db\.set_library_item_enabled/, 'CLI lifecycle toggles must use the shared domain service');
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
