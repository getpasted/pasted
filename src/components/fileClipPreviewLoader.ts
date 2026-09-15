import type { AppSettings } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { cachePreview, getCachedPreview } from '../utils/previewMemoryCache';
import type { FileClipPreview } from './fileClipPreviewModel';

interface FilePreviewRequest extends Record<string, unknown> {
  clipId: number;
  mode: AppSettings['filePreviewMode'];
  maxSizeMb: number;
  forceRecheck?: boolean;
  onlyIndex?: number;
}

const requestCache = new Map<string, Promise<FileClipPreview[]>>();

export const getCachedFilePreviews = (cacheKey: string) => getCachedPreview<FileClipPreview[]>(`file-detail:${cacheKey}`);

export function loadFilePreviews(
  cacheKey: string,
  request: FilePreviewRequest,
): Promise<FileClipPreview[]> {
  const cached = getCachedFilePreviews(cacheKey);
  if (cached) return Promise.resolve(cached);
  const pending = requestCache.get(cacheKey);
  if (pending) return pending;
  const next = invoke<FileClipPreview[]>('get_file_clip_previews', request)
    .then((items) => {
      const previews = Array.isArray(items) ? items : [];
      cachePreview(`file-detail:${cacheKey}`, previews);
      return previews;
    })
    .finally(() => requestCache.delete(cacheKey));
  requestCache.set(cacheKey, next);
  return next;
}

export async function recheckFilePreview(
  cacheKey: string,
  request: FilePreviewRequest,
  index: number,
) {
  const [preview] = await invoke<FileClipPreview[]>('get_file_clip_previews', {
    ...request,
    onlyIndex: index,
    forceRecheck: true,
  });
  if (!preview) return getCachedFilePreviews(cacheKey) ?? [];
  const merged = [...(getCachedFilePreviews(cacheKey) ?? [])].filter((item) => item.index !== index);
  merged.push(preview);
  merged.sort((left, right) => left.index - right.index);
  cachePreview(`file-detail:${cacheKey}`, merged);
  return merged;
}
