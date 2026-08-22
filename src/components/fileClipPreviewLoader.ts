import type { AppSettings } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import type { FileClipPreview } from './fileClipPreviewModel';

interface FilePreviewRequest extends Record<string, unknown> {
  clipId: number;
  mode: AppSettings['filePreviewMode'];
  maxSizeMb: number;
  forceRecheck?: boolean;
  onlyIndex?: number;
}

const resultCache = new Map<string, FileClipPreview[]>();
const requestCache = new Map<string, Promise<FileClipPreview[]>>();

export const getCachedFilePreviews = (cacheKey: string) => resultCache.get(cacheKey);

export function loadFilePreviews(
  cacheKey: string,
  request: FilePreviewRequest,
): Promise<FileClipPreview[]> {
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
  if (!preview) return resultCache.get(cacheKey) ?? [];
  const merged = [...(resultCache.get(cacheKey) ?? [])].filter((item) => item.index !== index);
  merged.push(preview);
  merged.sort((left, right) => left.index - right.index);
  resultCache.set(cacheKey, merged);
  return merged;
}
