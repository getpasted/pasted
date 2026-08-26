export const BACKED_UP_LOCAL_STORAGE_KEYS = [
  'pasted_app_ui_state',
  'pasted_sidebar_width',
  'pasted_list_width',
  'pasted_bin_order',
  'pasted_scroll_positions',
] as const;

export interface BackupStateStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

export function collectBackupClientStateFrom(storage: BackupStateStorage): string {
  const values = Object.fromEntries(BACKED_UP_LOCAL_STORAGE_KEYS.flatMap((key) => {
    const value = storage.getItem(key);
    return value === null ? [] : [[key, value]];
  }));
  return JSON.stringify({ version: 1, localStorage: values });
}

export function applyBackupClientStateTo(storage: BackupStateStorage, json: string): boolean {
  const parsed: unknown = JSON.parse(json);
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return false;
  const record = parsed as { version?: unknown; localStorage?: unknown };
  if (record.version !== 1 || !record.localStorage || typeof record.localStorage !== 'object' || Array.isArray(record.localStorage)) return false;
  const values = record.localStorage as Record<string, unknown>;
  for (const key of BACKED_UP_LOCAL_STORAGE_KEYS) {
    const value = values[key];
    if (typeof value === 'string') storage.setItem(key, value);
    else storage.removeItem(key);
  }
  return true;
}
