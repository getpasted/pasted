import { handled, unhandled, type BrowserMockResult } from './result';

export function handleBackupBrowserMock(command: string): BrowserMockResult {
  const date = new Date().toISOString().slice(0, 10);
  switch (command) {
    case 'export_backup_file':
      return handled(`/mock/Pasted_Library_Archive_${date}.json`);
    case 'choose_import_file':
      return handled({
        path: '/mock/Pasted_History_and_Organization.json', name: 'Pasted_History_and_Organization.json',
        kind: 'organization', format: 'json', sizeBytes: 184_320,
        library: { schemaVersion: 1, clipCount: 248, binCount: 7, operationCount: 5, transformCount: 12, classifierCount: 4, contentTypeCount: 9 },
      });
    case 'import_inspected_file':
      return handled({ importedCount: 248, duplicateCount: 0 });
    case 'export_full_backup_file':
      return handled({ path: `/mock/Pasted_Full_Backup_${date}.pastedbackup`, createdAt: new Date().toISOString(), sizeBytes: 2_457_600 });
    case 'restore_full_backup_file':
      return handled({ recoveryPath: '/mock/Pasted_Pre_Restore.pastedbackup', backupCreatedAt: new Date().toISOString() });
    case 'consume_pending_full_restore_client_state':
      return handled(null);
    default:
      return unhandled;
  }
}
