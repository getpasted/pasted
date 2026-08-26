import { backupApi } from '../api/backup';
import { settingsApi } from '../api/settings';
import {
  applyBackupClientStateTo,
  collectBackupClientStateFrom,
} from './backupClientStateCodec';
import {
  discardPendingScrollPositionPersistence,
  flushPendingScrollPositionPersistence,
} from './scrollPositionState';

let persistenceTimer: ReturnType<typeof setTimeout> | undefined;
let restoredBeforeAppMount = false;

export function collectBackupClientState(): string {
  flushPendingScrollPositionPersistence();
  return collectBackupClientStateFrom(localStorage);
}

export function applyBackupClientState(json: string): boolean {
  discardPendingScrollPositionPersistence();
  return applyBackupClientStateTo(localStorage, json);
}

export function scheduleBackupClientStatePersistence(delayMs = 300) {
  if (persistenceTimer) clearTimeout(persistenceTimer);
  persistenceTimer = setTimeout(() => {
    persistenceTimer = undefined;
    void settingsApi.save('backedUpClientState', collectBackupClientState()).catch(() => {
      // UI state remains available locally if the native library is unavailable.
    });
  }, delayMs);
}

export async function consumePendingBackupClientState(): Promise<boolean> {
  const state = await backupApi.consumePendingClientState();
  return state ? applyBackupClientState(state) : false;
}

export async function restorePendingBackupClientStateBeforeMount(): Promise<boolean> {
  restoredBeforeAppMount = await consumePendingBackupClientState();
  return restoredBeforeAppMount;
}

export function wasBackupClientStateRestoredBeforeMount(): boolean {
  return restoredBeforeAppMount;
}
