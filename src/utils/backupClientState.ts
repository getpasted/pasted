import { backupApi } from '../api/backup';
import { settingsApi } from '../api/settings';
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
    void settingsApi.save('backedUpClientState', collectBackupClientState()).catch(() => {
      // UI state remains available locally if the native library is unavailable.
    });
  }, 300);
}

export async function consumePendingBackupClientState(): Promise<boolean> {
  const state = await backupApi.consumePendingClientState();
  return state ? applyBackupClientState(state) : false;
}
