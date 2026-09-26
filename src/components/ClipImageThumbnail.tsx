import React from 'react';

import { translate } from '../localization/runtime';
import { safeInvoke as invoke } from '../utils/tauri';
import { cachePreview, getCachedPreview } from '../utils/previewMemoryCache';
import { loadVisibleThumbnail } from '../utils/thumbnailVisibility';
import { SafeRasterImage } from './SafeRasterImage';

export function ClipImageThumbnail({
  clipId,
  contentHash,
  maxHeightClass,
  placeholderHeightClass,
}: {
  clipId: number;
  contentHash: string;
  maxHeightClass: string;
  placeholderHeightClass: string;
}) {
  const stageRef = React.useRef<HTMLDivElement | null>(null);
  const cacheKey = `clip-image:${clipId}:${contentHash}`;
  const [source, setSource] = React.useState<string | null | undefined>(() => (
    getCachedPreview<string | null>(cacheKey)
  ));

  React.useEffect(() => {
    let cancelled = false;
    const stage = stageRef.current;
    if (!stage || source !== undefined) return undefined;

    const load = () => {
      invoke<string | null>('get_clip_image', { id: clipId })
        .then((image) => {
          cachePreview(cacheKey, image);
          if (!cancelled) setSource(image);
        })
        .catch(() => {
          if (!cancelled) setSource(null);
        });
    };

    const stopLoading = loadVisibleThumbnail(stage, load);
    return () => {
      cancelled = true;
      stopLoading();
    };
  }, [cacheKey, clipId, source]);

  return <div
    ref={stageRef}
    className={`clip-thumbnail-stage clip-thumbnail-lazy relative rounded border overflow-hidden p-1 flex justify-center ${placeholderHeightClass} ${source ? 'is-loaded' : ''}`}
  >
    {source && <SafeRasterImage
      source={source}
      alt={translate('component.clipCard.clipboardClip')}
      decoding="async"
      className={`${maxHeightClass} object-contain rounded`}
    />}
  </div>;
}
