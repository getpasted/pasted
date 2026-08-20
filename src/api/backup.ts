import { safeInvoke as invoke } from '../utils/tauri';

export interface FullBackupReport { path: string; createdAt: string; sizeBytes: number }
export interface FullRestoreReport { recoveryPath: string; backupCreatedAt: string }

export const backupApi = {
  exportTransfer: () => invoke<string | null>('export_backup_file'),
  chooseImport: <T>() => invoke<T | null>('choose_import_file'),
  importInspected: <T>(path: string, kind: string, format: string) =>
    invoke<T>('import_inspected_file', { path, kind, format }),
  exportFull: (clientStateJson: string) =>
    invoke<FullBackupReport | null>('export_full_backup_file', { clientStateJson }),
  restoreFull: (currentClientStateJson: string, backupPath?: string) =>
    invoke<FullRestoreReport | null>('restore_full_backup_file', { currentClientStateJson, backupPath }),
  consumePendingClientState: () => invoke<string | null>('consume_pending_full_restore_client_state'),
};
