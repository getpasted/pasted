import { safeInvoke as invoke } from './tauri';
import {
  applyBackupClientStateTo,
  collectBackupClientStateFrom,
} from './backupClientStateCodec';

let persistenceTimer: ReturnType<typeof setTimeout> | undefined;

export function collectBackupClientState(): string {
  return collectBackupClientStateFrom(localStorage);
}

export function applyBackupClientState(json: string): boolean {
  return applyBackupClientStateTo(localStorage, json);
}

export function scheduleBackupClientStatePersistence() {
  if (persistenceTimer) clearTimeout(persistenceTimer);
  persistenceTimer = setTimeout(() => {
    persistenceTimer = undefined;
    void invoke('save_app_setting', {
      key: 'backedUpClientState',
      value: collectBackupClientState(),
    }).catch(() => {
      // UI state remains available locally if the native library is unavailable.
    });
  }, 300);
}

export async function consumePendingBackupClientState(): Promise<boolean> {
  const state = await invoke<string | null>('consume_pending_full_restore_client_state');
  return state ? applyBackupClientState(state) : false;
}
