import type { AppSettings } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';

export interface FileClipPreview {
  index: number;
  dataUrl: string | null;
  textContent: string | null;
  width: number | null;
  height: number | null;
  availability: 'available' | 'missing' | 'inaccessible' | 'unavailable';
  cached: boolean;
}

const resultCache = new Map<string, FileClipPreview[]>();
const requestCache = new Map<string, Promise<FileClipPreview[]>>();

export const getCachedFilePreviews = (cacheKey: string) => resultCache.get(cacheKey);

export function loadFilePreviews(
  cacheKey: string,
  request: {
    clipId: number;
    mode: AppSettings['filePreviewMode'];
    maxSizeMb: number;
    forceRecheck?: boolean;
  },
  bypassCache = false,
): Promise<FileClipPreview[]> {
  if (bypassCache) {
    resultCache.delete(cacheKey);
    requestCache.delete(cacheKey);
  }
  const cached = resultCache.get(cacheKey);
  if (cached) return Promise.resolve(cached);
  const pending = requestCache.get(cacheKey);
  if (pending) return pending;
  const next = invoke<FileClipPreview[]>('get_file_clip_previews', request)
    .then((items) => {
      const previews = Array.isArray(items) ? items : [];
      resultCache.set(cacheKey, previews);
      return previews;
    })
    .finally(() => requestCache.delete(cacheKey));
  requestCache.set(cacheKey, next);
  return next;
}
