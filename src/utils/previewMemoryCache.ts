const MAX_PREVIEW_CACHE_WEIGHT = 48 * 1024 * 1024;

interface CachedPreview {
  value: unknown;
  weight: number;
}

const previews = new Map<string, CachedPreview>();
let totalWeight = 0;

function approximateWeight(value: unknown): number {
  if (typeof value === 'string') return value.length * 2;
  if (Array.isArray(value)) return value.reduce((total, item) => total + approximateWeight(item), 16);
  if (value && typeof value === 'object') {
    return Object.values(value).reduce((total, item) => total + approximateWeight(item), 32);
  }
  return 8;
}

export function getCachedPreview<T>(key: string): T | undefined {
  const cached = previews.get(key);
  if (!cached) return undefined;
  previews.delete(key);
  previews.set(key, cached);
  return cached.value as T;
}

export function cachePreview(key: string, value: unknown) {
  const previous = previews.get(key);
  if (previous) totalWeight -= previous.weight;
  previews.delete(key);
  const weight = approximateWeight(value);
  if (weight > MAX_PREVIEW_CACHE_WEIGHT) return;
  previews.set(key, { value, weight });
  totalWeight += weight;
  while (totalWeight > MAX_PREVIEW_CACHE_WEIGHT) {
    const oldestKey = previews.keys().next().value as string | undefined;
    if (oldestKey === undefined) break;
    totalWeight -= previews.get(oldestKey)?.weight ?? 0;
    previews.delete(oldestKey);
  }
}

export const previewCacheWeightForTests = () => totalWeight;
