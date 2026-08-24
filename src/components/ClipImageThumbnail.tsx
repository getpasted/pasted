import React from 'react';

import { translate } from '../localization/runtime';
import { safeInvoke as invoke } from '../utils/tauri';
import { SafeRasterImage } from './SafeRasterImage';

const clipImageCache = new Map<string, string | null>();

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
  const cacheKey = `${clipId}:${contentHash}`;
  const [source, setSource] = React.useState<string | null | undefined>(() => (
    clipImageCache.has(cacheKey) ? clipImageCache.get(cacheKey) : undefined
  ));

  React.useEffect(() => {
    let cancelled = false;
    const stage = stageRef.current;
    if (!stage || source !== undefined) return undefined;

    const load = () => {
      invoke<string | null>('get_clip_image', { id: clipId })
        .then((image) => {
          clipImageCache.set(cacheKey, image);
          if (!cancelled) setSource(image);
        })
        .catch(() => {
          if (!cancelled) setSource(null);
        });
    };

    if (typeof IntersectionObserver === 'undefined') {
      load();
      return () => {
        cancelled = true;
      };
    }

    const observer = new IntersectionObserver((entries) => {
      if (!entries.some((entry) => entry.isIntersecting)) return;
      observer.disconnect();
      load();
    }, { rootMargin: '240px 0px' });
    observer.observe(stage);
    return () => {
      cancelled = true;
      observer.disconnect();
    };
  }, [cacheKey, clipId, source]);

  return <div
    ref={stageRef}
    className={`clip-thumbnail-stage clip-thumbnail-lazy relative rounded border overflow-hidden p-1 flex justify-center ${source ? 'is-loaded' : placeholderHeightClass}`}
  >
    {source && <SafeRasterImage
      source={source}
      alt={translate('component.clipCard.clipboardClip')}
      loading="lazy"
      decoding="async"
      className={`${maxHeightClass} object-contain rounded`}
    />}
  </div>;
}
