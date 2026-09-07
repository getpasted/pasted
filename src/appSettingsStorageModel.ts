import { settingDefault } from './settingsContract.ts';

export const DEFAULT_SNAPSHOT_INTERVAL_MINUTES = settingDefault('snapshotIntervalMinutes');
export const DEFAULT_SNAPSHOT_KEEP_COUNT = settingDefault('snapshotKeepCount');

function storedBoundedNumber(
  saved: Record<string, string>,
  key: string,
  fallback: number,
  min: number,
  max: number,
): number {
  const value = Number(saved[key]);
  return Math.max(min, Math.min(max, Number.isFinite(value) ? value : fallback));
}

export function storedSnapshotIntervalMinutes(saved: Record<string, string>): number {
  return storedBoundedNumber(saved, 'snapshotIntervalMinutes', DEFAULT_SNAPSHOT_INTERVAL_MINUTES, 1, 10080);
}

export function storedSnapshotKeepCount(saved: Record<string, string>): number {
  return storedBoundedNumber(saved, 'snapshotKeepCount', DEFAULT_SNAPSHOT_KEEP_COUNT, 0, 10000);
}
