import { Files } from 'lucide-react';
import React from 'react';

import { translate } from '../localization/runtime';
import { getClipFilePaths, getClipFileSummary, type ClipItem } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { cachePreview, getCachedPreview } from '../utils/previewMemoryCache';
import { loadVisibleThumbnail, queueFileThumbnailLoad } from '../utils/thumbnailVisibility';
import { OverflowText } from './OverflowText';
import { SafeRasterImage } from './SafeRasterImage';

interface FileCardPreview {
  index: number;
  dataUrl: string | null;
  textContent: string | null;
}

const FILE_CARD_PREVIEW_DEADLINE_MS = 500;

export function ClipFileThumbnail({
  clip,
  mode,
  maxSizeMb,
  maxHeightClass,
  placeholderHeightClass,
}: {
  clip: ClipItem;
  mode: 'off' | 'safe' | 'all';
  maxSizeMb: number;
  maxHeightClass: string;
  placeholderHeightClass: string;
}) {
  const stageRef = React.useRef<HTMLDivElement | null>(null);
  const paths = React.useMemo(() => getClipFilePaths(clip), [clip.text_content]);
  const previewIndexes = React.useMemo(() => paths
    .map((path, index) => (/\.(?:jpe?g|pdf|png|txt|webp)$/i.test(path) ? index : -1))
    .filter((index) => index >= 0), [paths]);
  const cacheKey = `file-card:${clip.id}:${clip.content_hash}:${mode}:${maxSizeMb}`;
  const [preview, setPreview] = React.useState<FileCardPreview | null | undefined>(() => (
    getCachedPreview<FileCardPreview | null>(cacheKey)
  ));

  React.useEffect(() => {
    let cancelled = false;
    let timedOut = false;
    let deadline: number | undefined;
    const stage = stageRef.current;
    if (!stage || preview !== undefined || mode === 'off' || previewIndexes.length === 0) return undefined;

    const load = () => {
      let stopQueuedLoad = () => {};
      deadline = window.setTimeout(() => {
        timedOut = true;
        stopQueuedLoad();
        if (!cancelled) setPreview(null);
      }, FILE_CARD_PREVIEW_DEADLINE_MS);
      stopQueuedLoad = queueFileThumbnailLoad(() => {
        return invoke<FileCardPreview[]>('get_file_clip_previews', {
          clipId: clip.id,
          mode,
          maxSizeMb,
          onlyIndex: previewIndexes[0],
        }).then((previews) => {
          const nextPreview = previews.find((item) => previewIndexes.includes(item.index)) ?? null;
          cachePreview(cacheKey, nextPreview);
          if (!cancelled && !timedOut) setPreview(nextPreview);
        }).catch(() => {
          if (!cancelled && !timedOut) setPreview(null);
        }).finally(() => window.clearTimeout(deadline));
      });
      return () => {
        stopQueuedLoad();
        window.clearTimeout(deadline);
      };
    };

    const stopLoading = loadVisibleThumbnail(stage, load);
    return () => {
      cancelled = true;
      stopLoading();
    };
  }, [cacheKey, clip.id, maxSizeMb, mode, preview, previewIndexes]);

  if (mode === 'off' || previewIndexes.length === 0 || preview === null) {
    return <div className="clip-thumbnail-stage flex items-center gap-2 p-2 rounded border">
      <Files className="theme-status-info-text h-4 w-4 shrink-0" />
      <OverflowText text={getClipFileSummary(clip)} className="truncate" />
      {paths.length > 1 && <span className="theme-text-muted ms-auto shrink-0 text-[10px]">
        {translate('format.fileCount', { count: paths.length })}
      </span>}
    </div>;
  }

  const previewPath = paths[preview?.index ?? previewIndexes[0]] ?? '';
  return <div
    ref={stageRef}
    className={`clip-thumbnail-stage clip-thumbnail-lazy relative rounded border overflow-hidden p-1 ${placeholderHeightClass} ${preview?.dataUrl ? 'flex justify-center' : ''} ${preview ? 'is-loaded' : ''}`}
  >
    {preview && <>
      {preview.dataUrl ? <SafeRasterImage
        source={preview.dataUrl}
        alt={translate('common.previewOfName', { name: previewPath.split(/[\\/]/).pop() || translate('component.clipCard.file') })}
        decoding="async"
        className={`${maxHeightClass} object-contain rounded`}
      /> : <pre className={`${maxHeightClass} min-h-full overflow-hidden whitespace-pre-wrap break-words p-2 pb-6 font-mono text-[10px] leading-relaxed`}>
        {preview.textContent}
      </pre>}
      <OverflowText text={getClipFileSummary(clip)} className="theme-surface theme-text-muted elevation-control absolute bottom-1 start-1 max-w-[calc(100%-0.5rem)] truncate rounded-md px-1.5 py-0.5 text-[9px]" />
    </>}
  </div>;
}
