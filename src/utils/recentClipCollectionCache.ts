import type { ClipItem } from '../types';

interface CachedClipCollection {
  items: ClipItem[];
  totalCount: number;
}

const MAXIMUM_ENTRIES = 12;
const entries = new Map<string, CachedClipCollection>();
export const peekRecentClipCollection = (key: string) => entries.get(key);

export function getRecentClipCollection(key: string) {
  const cached = entries.get(key);
  if (!cached) return undefined;
  entries.delete(key);
  entries.set(key, cached);
  return cached;
}

export function cacheRecentClipCollection(key: string, collection: CachedClipCollection) {
  entries.delete(key);
  entries.set(key, collection);
  while (entries.size > MAXIMUM_ENTRIES) {
    const oldestKey = entries.keys().next().value as string | undefined;
    if (oldestKey === undefined) break;
    entries.delete(oldestKey);
  }
}

export function clearRecentClipCollections() {
  entries.clear();
}
