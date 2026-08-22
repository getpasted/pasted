import { useCallback, useEffect, useMemo, useState } from 'react';

import type { FileClipPreview } from '../components/fileClipPreviewModel';
import { getCachedFilePreviews, loadFilePreviews } from '../components/fileClipPreviewModel';
import type { ClipItem } from '../types';

interface UseFileClipPreviewsInput {
  clip: ClipItem | null;
  mode: 'off' | 'safe' | 'all';
  maxSizeMb: number;
}

export function useFileClipPreviews({ clip, mode, maxSizeMb }: UseFileClipPreviewsInput) {
  const [items, setItems] = useState<FileClipPreview[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const cacheKey = useMemo(
    () => clip ? `${clip.id}:${clip.content_hash}:${mode}:${maxSizeMb}` : '',
    [clip?.content_hash, clip?.id, maxSizeMb, mode],
  );

  useEffect(() => {
    let cancelled = false;
    if (!clip || clip.content_type !== 'file') {
      setItems([]);
      setIsLoading(false);
      return () => { cancelled = true; };
    }
    const cached = getCachedFilePreviews(cacheKey);
    if (cached) {
      setItems(cached);
      setIsLoading(false);
      return () => { cancelled = true; };
    }
    setItems([]);
    setIsLoading(true);
    loadFilePreviews(cacheKey, { clipId: clip.id, mode, maxSizeMb })
      .then((previews) => { if (!cancelled) setItems(previews); })
      .catch((error) => { if (!cancelled) console.error('Failed to load file previews:', error); })
      .finally(() => { if (!cancelled) setIsLoading(false); });
    return () => { cancelled = true; };
  }, [cacheKey, clip?.content_type, clip?.id, maxSizeMb, mode]);

  const recheck = useCallback(async () => {
    if (!clip || clip.content_type !== 'file') return;
    setIsLoading(true);
    try {
      const previews = await loadFilePreviews(
        cacheKey,
        { clipId: clip.id, mode, maxSizeMb, forceRecheck: true },
        true,
      );
      setItems(previews);
    } catch (error) {
      console.error('Failed to recheck file references:', error);
    } finally {
      setIsLoading(false);
    }
  }, [cacheKey, clip, maxSizeMb, mode]);

  return { filePreviews: items, isFilePreviewLoading: isLoading, recheckFileReferences: recheck };
}
