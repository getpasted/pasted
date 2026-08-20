import assert from 'node:assert/strict';
import fs from 'node:fs';

const read = (path) => fs.readFileSync(path, 'utf8');
const database = read('src-tauri/src/db.rs');
const commands = read('src-tauri/src/commands.rs');
const cli = read('src-tauri/src/bin/pasted_cli.rs');
const settings = read('src/components/SettingsSyncPanel.tsx');
const englishCatalog = JSON.parse(read('src/locales/en.json'));
const settingsCatalogCopy = [...settings.matchAll(/translate\('([^']+)'/g)]
  .flatMap((match) => {
    const value = englishCatalog[match[1]];
    return typeof value === 'string' ? [value] : Object.values(value ?? {});
  })
  .join('\n');
const reset = read('src/components/SettingsResetPanel.tsx');
const backupApi = read('src/api/backup.ts');
const clientState = read('src/utils/backupClientStateCodec.ts');

assert.match(database, /rusqlite::backup::Backup::new/, 'Full Backup must use SQLite online backup semantics');
assert.match(database, /CREATE TABLE pasted_backup_manifest/, 'Full Backup must carry a versioned manifest');
assert.match(database, /PRAGMA integrity_check/, 'Full Backup and Full Restore must validate SQLite integrity');
assert.match(database, /DbState::new\(temporary\.clone\(\)\)/, 'Full Restore must apply forward migrations before activation');
assert.match(database, /Pasted_Pre_Restore_/, 'Full Restore must create a complete recovery backup first');
assert.match(
  database,
  /validate_backup_json\(manifest\.client_state_json\.as_deref\(\), "Backup UI state"\)[\s\S]*validate_backup_json\(manifest\.window_state_json\.as_deref\(\), "Backup window state"\)/,
  'Full Restore must validate embedded interface and window state before replacement',
);
assert.match(database, /full_backup_round_trip_covers_every_durable_table_and_interface_state/, 'Full Backup must have a table-coverage round-trip test');
assert.match(database, /full_restore_rejects_invalid_embedded_state_before_replacing_library/, 'Full Restore must test pre-activation rejection');
assert.doesNotMatch(database, /let _ = conn\.execute\("ALTER TABLE/,
  'Schema migrations must not swallow ALTER TABLE failures');
assert.match(database, /fn add_column_if_missing/,
  'Additive schema migrations must explicitly distinguish existing columns from failures');

for (const command of ['export_full_backup_file', 'restore_full_backup_file']) {
  assert.match(commands, new RegExp(`pub async fn ${command}`), `GUI must expose ${command}`);
}
assert.match(cli, /"backup" =>/, 'CLI must expose the shared full-backup workflow');
assert.match(cli, /"restore" =>[\s\S]*?--yes/, 'CLI Full Restore must require explicit confirmation');
assert.match(reset, /backupApi\.exportFull/, 'Factory Reset must offer a truthful Full Backup safeguard through the Backup client');
assert.match(backupApi, /export_full_backup_file/, 'The Backup client must expose Full Backup creation');
assert.match(clientState, /BACKED_UP_LOCAL_STORAGE_KEYS/, 'Full Backup must carry meaningful interface state');
assert.match(database, /preflight_library_archive\(&payload\)/, 'Portable transfer must complete preflight before opening a write transaction');
assert.match(database, /inspect_library_archive_json/, 'Portable-transfer preflight must be independently testable');
assert.match(database, /library_archive_reimport_updates_stable_identities_without_duplicates/, 'Portable transfer must retain an idempotence regression test');
assert.match(commands, /pub async fn choose_import_file[\s\S]*?inspect_library_archive_json/, 'The GUI file chooser must preflight portable transfers asynchronously');
assert.match(commands, /choose_import_file[\s\S]*?spawn_blocking/, 'File validation must not block the app UI thread');
assert.match(settings, /backupApi\.chooseImport/, 'The GUI import must inspect a file before presenting an action');
assert.match(backupApi, /choose_import_file/, 'The Backup client must expose inspected file selection');
assert.match(settings, /translate\('component\.settingsSyncPanel\.updatesRecognizableMatchesAddsNewDataAndKeepsUnrelatedData'\)/,
  'The GUI must explain merge semantics');

for (const phrase of ['Backup', 'Recovery', 'History', 'Trash', 'Activity', 'Settings', 'Credentials', 'Original files']) {
  assert.ok(settingsCatalogCopy.includes(phrase), `Full Backup settings copy must disclose ${phrase}`);
}
assert.ok(settingsCatalogCopy.includes('History and Organization'),
  'The portable merge workflow must be named History and Organization');
assert.doesNotMatch(commands, /set_title\("Export Pasted Backup"\)/, 'Portable transfer must not masquerade as Full Backup');

console.log('Full Backup and Full Restore contract audit passed.');
