import React from 'react';
import { dateTimeAttribute, formatFullDateTime, formatRelativeTime } from '../utils/date';
import { ClipItem, getClipFilePaths, getClipFileSummary, getClipNoteSummary, type Bin } from '../types';
import type { ClipViewPolicy } from '../utils/clipViewPolicy';
import { clipDeleteLabel, UI_COPY } from '../utils/uiCopy';
import { safeInvoke as invoke } from '../utils/tauri';
import { getClipSearchHighlightTerms, type ClipSearchHighlightField } from '../utils/clipSearch';
import { FloatingActionStrip } from './FloatingActionStrip';
import { OverflowText } from './OverflowText';
import { useFeatures } from '../hooks/useFeatures';
import { useMinuteTick } from '../hooks/useMinuteTick';
import { ContentTypeIcon } from './ContentTypeIcon';
import { structuralClipType } from '../utils/contentTypes';
import { useContentTypes } from './ContentTypeProvider';
import { clipConcealmentPolicy } from '../utils/clipConcealment';
import { concealedClipMask } from '../utils/concealedClipMask';
import {
  Files,
  Pin,
  Trash2,
  Copy,
  Check,
  StickyNote,
  ScanText,
  RotateCcw,
  Trash,
  Eye,
  EyeOff,
  ArrowRightCircle,
  MinusCircle,
  Shield,
  Workflow,
  LoaderCircle,
  AlertTriangle,
  ShieldOff,
  X,
  FilePenLine,
} from 'lucide-react';
import { ClipBinSummary } from './ClipBinSummary';
import { SafeRasterImage } from './SafeRasterImage';
import { translate } from '../localization/runtime';
import { useLocalization } from '../localization/LocalizationProvider';
import { localizedSourceName } from '../localization/presentation';

const clipImageCache = new Map<string, string | null>();
interface FileCardPreview {
  index: number;
  dataUrl: string | null;
  textContent: string | null;
}

const clipFilePreviewCache = new Map<string, FileCardPreview | null>();

function HighlightedClipText({
  text,
  query,
  field,
}: {
  text: string;
  query?: string;
  field: ClipSearchHighlightField;
}) {
  if (!query) return <bdi>{text}</bdi>;
  const terms = getClipSearchHighlightTerms(query, field);
  if (terms.length === 0) return <bdi>{text}</bdi>;
  const escaped = terms.map((term) => term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
  const expression = new RegExp(`(${escaped.join('|')})`, 'gi');
  return (
    <bdi>
      {text.split(expression).map((part, index) => (
        terms.some((term) => term.toLowerCase() === part.toLowerCase())
          ? <mark className="clip-search-match" key={`${part}:${index}`}>{part}</mark>
          : <React.Fragment key={`${part}:${index}`}>{part}</React.Fragment>
      ))}
    </bdi>
  );
}

function ClipImageThumbnail({
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

  return (
    <div
      ref={stageRef}
      className={`clip-thumbnail-stage clip-thumbnail-lazy relative rounded border overflow-hidden p-1 flex justify-center ${source ? 'is-loaded' : placeholderHeightClass}`}
    >
      {source && (
        <SafeRasterImage
          source={source}
          alt={translate('component.clipCard.clipboardClip')}
          loading="lazy"
          decoding="async"
          className={`${maxHeightClass} object-contain rounded`}
        />
      )}
    </div>
  );
}

function ClipFileThumbnail({
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
  const cacheKey = `${clip.id}:${clip.content_hash}:${mode}:${maxSizeMb}`;
  const [preview, setPreview] = React.useState<FileCardPreview | null | undefined>(() => (
    clipFilePreviewCache.has(cacheKey) ? clipFilePreviewCache.get(cacheKey) : undefined
  ));

  React.useEffect(() => {
    let cancelled = false;
    const stage = stageRef.current;
    if (!stage || preview !== undefined || mode === 'off' || previewIndexes.length === 0) return undefined;

    const load = () => {
      invoke<FileCardPreview[]>('get_file_clip_previews', {
        clipId: clip.id,
        mode,
        maxSizeMb,
        onlyIndex: previewIndexes[0],
      })
        .then((previews) => {
          const nextPreview = previews.find((item) => previewIndexes.includes(item.index)) ?? null;
          clipFilePreviewCache.set(cacheKey, nextPreview);
          if (!cancelled) setPreview(nextPreview);
        })
        .catch(() => {
          if (!cancelled) setPreview(null);
        });
    };

    if (typeof IntersectionObserver === 'undefined') {
      load();
      return () => { cancelled = true; };
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
  }, [cacheKey, clip.id, maxSizeMb, mode, preview, previewIndexes]);

  if (mode === 'off' || previewIndexes.length === 0 || preview === null) {
    return (
      <div className="clip-thumbnail-stage flex items-center gap-2 p-2 rounded border">
        <Files className="theme-status-info-text h-4 w-4 shrink-0" />
        <OverflowText text={getClipFileSummary(clip)} className="truncate" />
        {paths.length > 1 && (
          <span className="theme-text-muted ms-auto shrink-0 text-[10px]">{translate('format.fileCount', { count: paths.length })}</span>
        )}
      </div>
    );
  }

  const previewPath = paths[preview?.index ?? previewIndexes[0]] ?? '';
  return (
    <div
      ref={stageRef}
      className={`clip-thumbnail-stage clip-thumbnail-lazy relative rounded border overflow-hidden p-1 ${preview?.dataUrl ? 'flex justify-center' : ''} ${preview ? 'is-loaded' : placeholderHeightClass}`}
    >
      {preview && (
        <>
          {preview.dataUrl ? (
            <SafeRasterImage
              source={preview.dataUrl}
              alt={translate('common.previewOfName', { name: previewPath.split(/[\\/]/).pop() || translate('component.clipCard.file') })}
              loading="lazy"
              decoding="async"
              className={`${maxHeightClass} object-contain rounded`}
            />
          ) : (
            <pre className={`${maxHeightClass} min-h-full overflow-hidden whitespace-pre-wrap break-words p-2 pb-6 font-mono text-[10px] leading-relaxed`}>
              {preview.textContent}
            </pre>
          )}
          <OverflowText text={getClipFileSummary(clip)} className="theme-surface theme-text-muted absolute bottom-1 start-1 max-w-[calc(100%-0.5rem)] truncate rounded-md px-1.5 py-0.5 text-[9px] shadow-sm" />
        </>
      )}
    </div>
  );
}

interface ClipCardProps {
  clip: ClipItem;
  isSelected: boolean;
  isHovered?: boolean;
  showActions?: boolean;
  isDragging?: boolean;
  isDragInProgress?: boolean;
  isTransforming?: boolean;
  transformError?: string;
  reorderOffsetY?: number;
  stableReorderId?: string;
  isDeleting?: boolean;
  viewPolicy: ClipViewPolicy;
  isQueueMode?: boolean;
  queueIndex?: number;
  bins: Bin[];
  rowHeight?: 'small' | 'medium' | 'large';
  filePreviewMode: 'off' | 'safe' | 'all';
  filePreviewMaxMb: number;
  trashEnabled: boolean;
  searchQuery?: string;
  onSelect: (clip: ClipItem, e: React.MouseEvent) => void;
  onPin: () => void;
  onToggleProtected?: () => void;
  onToggleConcealed?: () => void;
  onName?: () => void;
  onDelete: (e?: React.MouseEvent) => void;
  onCopy: () => void;
  onRestore?: () => void;
  onPurgePermanently?: () => void;
  onRemoveFromQueue?: () => void;
  onPasteQueueItem?: () => void;
  onContextMenu: (e: React.MouseEvent) => void;
  setDraggedClipId?: (id: number | null) => void;
  onPointerDragStart?: (id: number) => void;
  onPointerDragMove?: (x: number, y: number) => void;
  onPointerDragEnd?: (x: number, y: number, id: number) => void;
  onPointerDragCancel?: () => void;
}

const ClipCardComponent: React.FC<ClipCardProps> = ({
  clip,
  isSelected,
  isHovered = false,
  showActions = false,
  isDragging = false,
  isDragInProgress = false,
  isTransforming = false,
  transformError,
  reorderOffsetY = 0,
  stableReorderId,
  isDeleting = false,
  viewPolicy,
  isQueueMode = false,
  queueIndex,
  bins,
  rowHeight = 'medium',
  filePreviewMode,
  filePreviewMaxMb,
  trashEnabled,
  searchQuery,
  onSelect,
  onPin,
  onToggleProtected,
  onToggleConcealed,
  onName,
  onDelete,
  onCopy,
  onRestore,
  onPurgePermanently,
  onRemoveFromQueue,
  onPasteQueueItem,
  onContextMenu,
  setDraggedClipId,
  onPointerDragStart,
  onPointerDragMove,
  onPointerDragEnd,
  onPointerDragCancel,
}) => {
  useLocalization();
  const relativeTimeNow = useMinuteTick();
  const features = useFeatures();
  const { definitions: contentTypeDefinitions } = useContentTypes();
  const [copied, setCopied] = React.useState(false);
  const pointerDragRef = React.useRef<{
    pointerId: number;
    startX: number;
    startY: number;
    active: boolean;
  } | null>(null);
  const removePointerListenersRef = React.useRef<(() => void) | null>(null);
  const suppressClickRef = React.useRef(false);
  const primaryContentType = clip.content_types?.[0] ?? clip.content_type;
  const concealment = clipConcealmentPolicy(clip, bins, contentTypeDefinitions);
  const isSensitive = features.concealment && concealment.effective;

  React.useEffect(() => () => removePointerListenersRef.current?.(), []);

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    onCopy();
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const isSmall = rowHeight === 'small';
  const isLarge = rowHeight === 'large';

  const paddingClass = isSmall ? 'p-2' : isLarge ? 'p-5' : 'p-3.5';
  const lineClampClass = isSmall ? 'line-clamp-1 text-[11px]' : isLarge ? 'line-clamp-5 text-[13px]' : 'line-clamp-2 text-xs';
  const imgMaxHeightClass = isSmall ? 'max-h-16' : isLarge ? 'max-h-44' : 'max-h-24';
  const imgPlaceholderHeightClass = isSmall ? 'min-h-16' : isLarge ? 'min-h-44' : 'min-h-24';
  const headerTextClass = isSmall ? 'text-[10px]' : isLarge ? 'text-[13px]' : 'text-xs';
  const headerSpacingClass = isSmall ? 'mb-0.5' : isLarge ? 'mb-2' : 'mb-1';
  const noteSummary = features.notes ? getClipNoteSummary(clip.note) : '';
  const isTrashMode = viewPolicy.state === 'trash';
  const protectedByBin = Boolean(clip.protecting_bin_ids?.length);
  const protectionToggleDisabled = Boolean(clip.hotkey) || protectedByBin;
  const attributeTintClass = isTrashMode
    ? 'clip-card-trashed'
    : features.protection && clip.is_protected
      ? 'clip-card-attribute clip-card-protected'
      : features.pinning && clip.is_pinned
        ? 'clip-card-attribute clip-card-pinned'
        : noteSummary
          ? 'clip-card-attribute clip-card-noted'
          : '';

  return (
    <div
      data-clip-id={clip.id}
      data-stable-reorder-id={stableReorderId}
      data-pinned-clip={features.pinning && clip.is_pinned ? 'true' : undefined}
      onClick={(e) => {
        if (suppressClickRef.current) {
          e.preventDefault();
          e.stopPropagation();
          suppressClickRef.current = false;
          return;
        }
        onSelect(clip, e);
      }}
      onContextMenu={onContextMenu}
      draggable={false}
      style={reorderOffsetY !== 0 || isDragging ? {
        transform: `translateY(${reorderOffsetY}px)`,
        zIndex: isDragging ? 20 : 10,
      } : undefined}
      onPointerDown={(e) => {
        if (!viewPolicy.canDragClips || e.button !== 0 || (e.target as HTMLElement).closest('button, input, select, textarea, a')) return;
        pointerDragRef.current = {
          pointerId: e.pointerId,
          startX: e.clientX,
          startY: e.clientY,
          active: false,
        };

        const removeListeners = () => {
          window.removeEventListener('pointermove', handlePointerMove);
          window.removeEventListener('pointerup', handlePointerEnd);
          window.removeEventListener('pointercancel', handlePointerCancel);
          window.removeEventListener('keydown', handleKeyDown);
          removePointerListenersRef.current = null;
        };

        const suppressClickUntilPointerRelease = () => {
          suppressClickRef.current = true;
          const clearSuppression = () => {
            window.removeEventListener('pointerup', clearSuppression);
            window.removeEventListener('pointercancel', clearSuppression);
            setTimeout(() => {
              suppressClickRef.current = false;
            }, 0);
          };
          window.addEventListener('pointerup', clearSuppression);
          window.addEventListener('pointercancel', clearSuppression);
        };

        const handlePointerMove = (event: PointerEvent) => {
          const drag = pointerDragRef.current;
          if (!drag || drag.pointerId !== event.pointerId) return;
          if (!drag.active && Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY) >= 6) {
            drag.active = true;
            if (setDraggedClipId) setDraggedClipId(clip.id);
            if (onPointerDragStart) onPointerDragStart(clip.id);
          }
          if (drag.active) {
            event.preventDefault();
            if (onPointerDragMove) onPointerDragMove(event.clientX, event.clientY);
          }
        };

        const handlePointerEnd = (event: PointerEvent) => {
          const drag = pointerDragRef.current;
          if (!drag || drag.pointerId !== event.pointerId) return;
          pointerDragRef.current = null;
          removeListeners();
          if (!drag.active) return;
          suppressClickRef.current = true;
          if (onPointerDragEnd) onPointerDragEnd(event.clientX, event.clientY, clip.id);
          if (setDraggedClipId) setDraggedClipId(null);
          setTimeout(() => {
            suppressClickRef.current = false;
          }, 0);
        };

        const handlePointerCancel = (event: PointerEvent) => {
          const drag = pointerDragRef.current;
          if (!drag || drag.pointerId !== event.pointerId) return;
          pointerDragRef.current = null;
          removeListeners();
          if (setDraggedClipId) setDraggedClipId(null);
          if (onPointerDragCancel) onPointerDragCancel();
        };

        const handleKeyDown = (event: KeyboardEvent) => {
          const drag = pointerDragRef.current;
          if (event.key !== 'Escape' || !drag?.active) return;
          event.preventDefault();
          event.stopPropagation();
          pointerDragRef.current = null;
          removeListeners();
          suppressClickUntilPointerRelease();
          if (setDraggedClipId) setDraggedClipId(null);
          if (onPointerDragCancel) onPointerDragCancel();
        };

        removePointerListenersRef.current?.();
        removePointerListenersRef.current = removeListeners;
        window.addEventListener('pointermove', handlePointerMove, { passive: false });
        window.addEventListener('pointerup', handlePointerEnd);
        window.addEventListener('pointercancel', handlePointerCancel);
        window.addEventListener('keydown', handleKeyDown);
      }}
      className={`clip-card relative cursor-pointer select-none border transition-[background-color,border-color,box-shadow,opacity,transform] duration-75 ease-out ${paddingClass} ${
        isDeleting
          ? 'clip-card-deleting'
          : `${isSelected
              ? 'clip-card-selected'
              : `clip-card-idle ${isHovered && !isDragInProgress ? 'clip-card-hovered' : ''}`
            }`
      } ${attributeTintClass} ${isDragging ? 'clip-card-drag-source' : ''} ${isTransforming ? 'clip-card-transforming' : ''}`}
    >
      {/* Header Info */}
      <div className={`clip-card-header flex items-center justify-between ${headerTextClass} ${headerSpacingClass}`}>
        <div className="flex items-center space-x-2">
          {(features.clipTypes || (features.types && (clip.content_types?.length ?? 0) > 0)) && (
            <div className="clip-type-icon theme-badge p-1 rounded border">
              <ContentTypeIcon type={features.types && (clip.content_types?.length ?? 0) > 0 ? primaryContentType : structuralClipType(clip.content_type)} className="w-3.5 h-3.5 theme-text-muted" />
            </div>
          )}
          {features.sources && <span className="font-medium theme-text-main truncate max-w-[120px]" title={localizedSourceName(clip.source)}>
            <HighlightedClipText text={localizedSourceName(clip.source)} query={searchQuery} field="source" />
          </span>}
        </div>
        <div className="clip-meta-row theme-text-subtle flex items-center text-[11px] font-mono">
          {features.transformations && isTransforming && (
            <span
              role="status"
              aria-label={translate('component.clipCard.applyingTransform')}
              title={translate('component.clipCard.applyingTransform2')}
              className="clip-meta-item clip-meta-icon-only clip-transform-working"
            >
              <LoaderCircle className="clip-meta-icon animate-spin" />
            </span>
          )}
          {features.transformations && !isTransforming && transformError && (
            <span
              role="status"
              aria-label={translate('component.clipCard.transformFailed')}
              title={translate('component.clipCard.transformFailedTransformerror', { transformError: transformError })}
              className="clip-meta-item clip-meta-icon-only theme-danger-text"
            >
              <AlertTriangle className="clip-meta-icon" />
            </span>
          )}
          {features.bins && <ClipBinSummary bins={bins} primaryBinId={clip.bin_id} />}
          {features.protection && clip.is_protected && (
            <span
              role="img"
              aria-label={translate('component.clipCard.protectedClip')}
              title={clip.hotkey
                ? translate('component.clipCard.protectedByHotkey')
                : protectedByBin
                  ? translate('component.clipCard.protectedByBin')
                  : translate('component.clipCard.protected')}
              className="clip-meta-item clip-meta-icon-only clip-protected-accent"
            >
              <Shield className="clip-meta-icon" />
            </span>
          )}
          {features.transformations && clip.is_transformed && (
            <span
              role="img"
              aria-label={translate('component.clipCard.transformedClip')}
              title={translate('component.clipCard.transformed')}
              className="clip-meta-item clip-meta-icon-only transform-accent manual-transforms"
            >
              <Workflow className="clip-meta-icon" />
            </span>
          )}
          {features.queue && queueIndex !== undefined && (
            queueIndex === 1 ? (
              <span className="clip-meta-item clip-queue-next rounded-full font-mono font-extrabold shadow animate-pulse">
                {translate('component.clipCard.nextUp1')}
              </span>
            ) : (
              <span className="clip-meta-item clip-queue-position rounded-full font-mono font-semibold">
                {translate('component.clipCard.queuePosition', { position: queueIndex })}</span>
            )
          )}
          {clip.content_type === 'image' && clip.text_content && (
            <span
              role="img"
              aria-label={translate('component.clipCard.ocrTextAvailable')}
              title={translate('component.clipCard.ocrText')}
              className="clip-meta-item clip-meta-icon-only clip-ocr-accent"
            >
              <ScanText className="clip-meta-icon" />
            </span>
          )}
          {noteSummary && (
            <span title={translate('component.clipCard.notesNotesummary', { noteSummary: noteSummary })} className="clip-meta-item clip-meta-icon-only">
              <StickyNote className="clip-meta-icon clip-note-accent" />
            </span>
          )}
          {features.pinning && clip.is_pinned && (
            <span title={translate('component.clipCard.pinned')} className="clip-meta-item clip-meta-icon-only">
              <Pin className="clip-meta-icon pin-icon" />
            </span>
          )}
          {isTrashMode && (
            <span role="img" aria-label={translate('component.clipCard.clipInTrash')} title={translate('component.clipCard.inTrash')} className="clip-meta-item clip-meta-icon-only theme-status-danger-text">
              <Trash2 className="clip-meta-icon" />
            </span>
          )}
          <time
            className="clip-meta-time"
            dateTime={dateTimeAttribute(clip.created_at)}
            title={formatFullDateTime(clip.created_at)}
          >
            {formatRelativeTime(clip.created_at, relativeTimeNow)}
          </time>
        </div>
      </div>

      {features.naming && clip.name && (
        <div className="theme-named-text my-2.5 flex items-center space-x-2 text-xs font-semibold font-sans">
          {(features.clipTypes || (features.types && (clip.content_types?.length ?? 0) > 0)) && (
            <span className="clip-name-icon shrink-0 rounded border p-1">
              <FilePenLine className="h-3.5 w-3.5" />
            </span>
          )}
          <span className="truncate" title={clip.name}>
            <HighlightedClipText text={clip.name} query={searchQuery} field="name" />
          </span>
        </div>
      )}

      {/* Body Content */}
      <div className={`theme-text-main ${clip.content_type === 'file' ? (isSmall ? 'text-[11px]' : 'text-xs') : lineClampClass} font-mono leading-relaxed break-all`}>
        {isSensitive ? (
          <div
            className="theme-status-warning flex items-center rounded-lg border p-1.5 text-xs font-mono select-none"
            aria-label={translate('collection.concealed')}
          >
            <span className="tracking-widest font-bold">{concealedClipMask(clip)}</span>
          </div>
        ) : clip.content_type === 'image' ? (
          <ClipImageThumbnail
            key={`${clip.id}:${clip.content_hash}`}
            clipId={clip.id}
            contentHash={clip.content_hash}
            maxHeightClass={imgMaxHeightClass}
            placeholderHeightClass={imgPlaceholderHeightClass}
          />
        ) : clip.content_type === 'file' ? (
          <ClipFileThumbnail
            key={`${clip.id}:${clip.content_hash}`}
            clip={clip}
            mode={filePreviewMode}
            maxSizeMb={filePreviewMaxMb}
            maxHeightClass={imgMaxHeightClass}
            placeholderHeightClass={imgPlaceholderHeightClass}
          />
        ) : (clip.content_types ?? [clip.content_type]).includes('color') ? (
          <div className="clip-thumbnail-stage flex items-center space-x-3 p-2 rounded border">
            <div
              className="theme-divider w-8 h-8 rounded border shadow"
              style={{ backgroundColor: clip.text_content || '#ffffff' }}
            />
            <span className="clip-note-accent font-mono text-xs">
              {clip.text_content}
            </span>
          </div>
        ) : (
          <div className="relative flex items-center justify-between">
            <span>
              <HighlightedClipText text={clip.text_content || translate('component.clipCard.emptyItem')} query={searchQuery} field="content" />
            </span>
          </div>
        )}
      </div>

      {/* Note preview if attached */}
      {features.notes && noteSummary && (
        <div className="clip-note-summary mt-2 pt-1.5 border-t flex items-center space-x-1.5 text-[11px] font-sans italic">
          <StickyNote className="w-3 h-3 shrink-0" />
          <span className="truncate" title={noteSummary}>
            <HighlightedClipText text={noteSummary} query={searchQuery} field="note" />
          </span>
        </div>
      )}

      {/* Hover Action Buttons */}
      <FloatingActionStrip
        label={translate('component.clipCard.clipActions')}
        visible={showActions && !isDragInProgress}
      >
        <button
          onClick={handleCopy}
          className="floating-action-button"
          title={copied ? UI_COPY.copied : UI_COPY.copy}
        >
          {copied ? (
            <Check className="w-3.5 h-3.5 theme-status-success-text" />
          ) : (
            <Copy className="w-3.5 h-3.5" />
          )}
        </button>
        {features.concealment && viewPolicy.canOrganize && onToggleConcealed && (
          <button
            onClick={(event) => {
              event.stopPropagation();
              onToggleConcealed();
            }}
            className="floating-action-button is-warning"
            title={concealment.effective
              ? translate('component.clipCard.revealSensitiveText')
              : translate('action.conceal')}
          >
            {concealment.effective
              ? <Eye className="h-3.5 w-3.5" />
              : <EyeOff className="h-3.5 w-3.5" />}
          </button>
        )}
        {features.naming && viewPolicy.canOrganize && onName && (
          <button
            onClick={(event) => {
              event.stopPropagation();
              onName();
            }}
            className={`floating-action-button is-named ${clip.name ? 'is-active' : ''}`}
            title={clip.name ? translate('action.editName') : translate('action.nameClip')}
          >
            <FilePenLine className="h-3.5 w-3.5" />
          </button>
        )}

        {isQueueMode || queueIndex !== undefined ? (
          <>
            {onPasteQueueItem && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onPasteQueueItem();
                }}
                className="floating-action-button is-accent"
                title={translate('component.clipCard.paste')}
              >
                <ArrowRightCircle className="h-3.5 w-3.5 rtl:-scale-x-100" />
              </button>
            )}
            {onRemoveFromQueue && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onRemoveFromQueue();
                }}
                className="floating-action-button is-danger"
                title={translate('component.clipCard.removeFromQueue')}
              >
                <MinusCircle className="w-3.5 h-3.5" />
              </button>
            )}
          </>
        ) : isTrashMode ? (
          <>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onRestore?.();
              }}
              className="floating-action-button is-accent"
              title={UI_COPY.restore}
            >
              <RotateCcw className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onPurgePermanently?.();
              }}
              className="floating-action-button is-danger"
              title={UI_COPY.deletePermanently}
            >
              <Trash className="w-3.5 h-3.5" />
            </button>
          </>
        ) : (
          <>
            {features.pinning && <button
              onClick={(e) => {
                e.stopPropagation();
                onPin();
              }}
              className={`floating-action-button ${
                clip.is_pinned ? 'is-success pin-icon' : ''
              }`}
              title={clip.is_pinned ? UI_COPY.unpin : UI_COPY.pin}
            >
              <Pin className="w-3.5 h-3.5" />
            </button>}

            {features.protection && onToggleProtected && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onToggleProtected();
                }}
                disabled={protectionToggleDisabled}
                className={`floating-action-button ${
                  clip.is_protected ? 'is-accent' : ''
                }`}
                title={clip.hotkey
                  ? translate('component.clipCard.protectedByHotkey')
                  : protectedByBin
                    ? translate('component.clipPreview.protectedByBin')
                    : clip.is_protected ? UI_COPY.unprotect : UI_COPY.protect}
              >
                {clip.is_protected && !protectionToggleDisabled ? (
                  <ShieldOff className="w-3.5 h-3.5" />
                ) : (
                  <Shield className="w-3.5 h-3.5" />
                )}
              </button>
            )}

            <button
              onClick={(e) => {
                e.stopPropagation();
                if (!clip.is_protected) {
                  onDelete(e);
                }
              }}
              disabled={clip.is_protected}
              className={`floating-action-button ${
                clip.is_protected
                  ? 'is-disabled cursor-not-allowed opacity-50'
                  : 'is-danger'
              }`}
              title={clip.is_protected
                ? translate('component.clipCard.clipIsProtectedUnprotectFirstToDelete')
                : trashEnabled
                  ? translate('component.clipCard.movetotrashOptionClickToDeletePermanently', { moveToTrash: UI_COPY.moveToTrash })
                  : clipDeleteLabel({ trashEnabled })}
            >
              {trashEnabled ? <Trash2 className="w-3.5 h-3.5" /> : <X className="w-3.5 h-3.5" />}
            </button>
          </>
        )}
      </FloatingActionStrip>

    </div>
  );
};

export const ClipCard = React.memo(ClipCardComponent, (prevProps, nextProps) => {
  const previousBinIds = prevProps.clip.bin_ids ?? [];
  const nextBinIds = nextProps.clip.bin_ids ?? [];
  const previousContentTypes = prevProps.clip.content_types ?? [];
  const nextContentTypes = nextProps.clip.content_types ?? [];
  const previousProtectingBinIds = prevProps.clip.protecting_bin_ids ?? [];
  const nextProtectingBinIds = nextProps.clip.protecting_bin_ids ?? [];
  return (
    prevProps.clip.id === nextProps.clip.id &&
    prevProps.clip.content_hash === nextProps.clip.content_hash &&
    prevProps.clip.content_type === nextProps.clip.content_type &&
    previousContentTypes.length === nextContentTypes.length &&
    previousContentTypes.every((type, index) => type === nextContentTypes[index]) &&
    prevProps.clip.text_content === nextProps.clip.text_content &&
    prevProps.clip.image_base64 === nextProps.clip.image_base64 &&
    prevProps.clip.source === nextProps.clip.source &&
    prevProps.clip.created_at === nextProps.clip.created_at &&
    prevProps.clip.is_pinned === nextProps.clip.is_pinned &&
    prevProps.clip.pin_order === nextProps.clip.pin_order &&
    prevProps.clip.is_protected === nextProps.clip.is_protected &&
    prevProps.clip.is_explicitly_protected === nextProps.clip.is_explicitly_protected &&
    prevProps.clip.is_concealed === nextProps.clip.is_concealed &&
    prevProps.clip.is_explicitly_concealed === nextProps.clip.is_explicitly_concealed &&
    prevProps.clip.is_explicitly_revealed === nextProps.clip.is_explicitly_revealed &&
    prevProps.clip.hotkey === nextProps.clip.hotkey &&
    previousProtectingBinIds.length === nextProtectingBinIds.length &&
    previousProtectingBinIds.every((id, index) => id === nextProtectingBinIds[index]) &&
    prevProps.clip.is_transformed === nextProps.clip.is_transformed &&
    prevProps.clip.note === nextProps.clip.note &&
    prevProps.clip.name === nextProps.clip.name &&
    prevProps.clip.bin_id === nextProps.clip.bin_id &&
    previousBinIds.length === nextBinIds.length &&
    previousBinIds.every((id, index) => id === nextBinIds[index]) &&
    prevProps.isSelected === nextProps.isSelected &&
    prevProps.isHovered === nextProps.isHovered &&
    prevProps.showActions === nextProps.showActions &&
    prevProps.isDragging === nextProps.isDragging &&
    prevProps.isDragInProgress === nextProps.isDragInProgress &&
    prevProps.isTransforming === nextProps.isTransforming &&
    prevProps.transformError === nextProps.transformError &&
    prevProps.reorderOffsetY === nextProps.reorderOffsetY &&
    prevProps.isDeleting === nextProps.isDeleting &&
    prevProps.viewPolicy.state === nextProps.viewPolicy.state &&
    prevProps.viewPolicy.canDragClips === nextProps.viewPolicy.canDragClips &&
    prevProps.isQueueMode === nextProps.isQueueMode &&
    prevProps.queueIndex === nextProps.queueIndex &&
    prevProps.bins.length === nextProps.bins.length &&
    prevProps.bins.every((bin, index) => {
      const nextBin = nextProps.bins[index];
      return bin.id === nextBin?.id
        && bin.name === nextBin.name
        && bin.icon === nextBin.icon
        && bin.color === nextBin.color
        && bin.smart_rule === nextBin.smart_rule
        && bin.protect_clips === nextBin.protect_clips
        && bin.conceal_clips === nextBin.conceal_clips;
    }) &&
    prevProps.rowHeight === nextProps.rowHeight &&
    prevProps.filePreviewMode === nextProps.filePreviewMode &&
    prevProps.filePreviewMaxMb === nextProps.filePreviewMaxMb &&
    prevProps.searchQuery === nextProps.searchQuery &&
    prevProps.trashEnabled === nextProps.trashEnabled
  );
});
