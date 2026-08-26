import { safeInvoke as invoke } from '../utils/tauri';
import type { StorageProtectionInfo } from './settingsSyncModel';

let cachedStorageProtection: StorageProtectionInfo | null = null;
let storageProtectionRequest: Promise<StorageProtectionInfo> | null = null;

export function getCachedStorageProtection() {
  return cachedStorageProtection;
}

export function cacheStorageProtection(protection: StorageProtectionInfo) {
  cachedStorageProtection = protection;
}

export function loadStorageProtection(force = false): Promise<StorageProtectionInfo> {
  if (force) {
    cachedStorageProtection = null;
    storageProtectionRequest = null;
  }
  if (cachedStorageProtection) return Promise.resolve(cachedStorageProtection);
  if (storageProtectionRequest) return storageProtectionRequest;
  storageProtectionRequest = invoke<StorageProtectionInfo>('get_storage_protection')
    .then((protection) => {
      cachedStorageProtection = protection;
      return protection;
    })
    .finally(() => {
      storageProtectionRequest = null;
    });
  return storageProtectionRequest;
}
