import React, { useState } from 'react';
import { formatClipTime } from '../utils/date';
import { formatEmojiIcon } from '../utils/emoji';
import { ClipItem, getClipNoteSummary, isSensitiveText, maskSensitiveText } from '../types';
import type { ClipViewPolicy } from '../utils/clipViewPolicy';
import { clipDeleteLabel, UI_COPY } from '../utils/uiCopy';
import { FloatingActionStrip } from './FloatingActionStrip';
import {
  Code,
  FileText,
  Image as ImageIcon,
  Link as LinkIcon,
  Palette,
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
} from 'lucide-react';

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
  isDeleting?: boolean;
  viewPolicy: ClipViewPolicy;
  isQueueMode?: boolean;
  queueIndex?: number;
  primaryBinName?: string;
  primaryBinIcon?: string;
  rowHeight?: 'small' | 'medium' | 'large';
  selectionVersion: string;
  trashEnabled: boolean;
  onSelect: (e: React.MouseEvent) => void;
  onPin: () => void;
  onToggleProtected?: () => void;
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
  isDeleting = false,
  viewPolicy,
  isQueueMode = false,
  queueIndex,
  primaryBinName,
  primaryBinIcon,
  rowHeight = 'medium',
  trashEnabled,
  onSelect,
  onPin,
  onToggleProtected,
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
  const [copied, setCopied] = React.useState(false);
  const [showRevealed, setShowRevealed] = useState(false);
  const pointerDragRef = React.useRef<{
    pointerId: number;
    startX: number;
    startY: number;
    active: boolean;
  } | null>(null);
  const removePointerListenersRef = React.useRef<(() => void) | null>(null);
  const suppressClickRef = React.useRef(false);
  const isSensitive = isSensitiveText(clip.text_content);

  React.useEffect(() => () => removePointerListenersRef.current?.(), []);

  const handleCopy = (e: React.MouseEvent) => {
    e.stopPropagation();
    onCopy();
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  const getIcon = () => {
    switch (clip.content_type) {
      case 'code':
        return <Code className="w-3.5 h-3.5 theme-status-success-text" />;
      case 'image':
        return <ImageIcon className="w-3.5 h-3.5 text-pink-400" />;
      case 'color':
        return <Palette className="w-3.5 h-3.5 theme-status-warning-text" />;
      case 'link':
        return <LinkIcon className="w-3.5 h-3.5 text-blue-400" />;
      default:
        return <FileText className="w-3.5 h-3.5 theme-text-muted" />;
    }
  };

  const isSmall = rowHeight === 'small';
  const isLarge = rowHeight === 'large';

  const paddingClass = isSmall ? 'p-2.5' : isLarge ? 'p-4' : 'p-3';
  const lineClampClass = isSmall ? 'line-clamp-1 text-[11px]' : isLarge ? 'line-clamp-5 text-xs' : 'line-clamp-2 text-xs';
  const imgMaxHeightClass = isSmall ? 'max-h-16' : isLarge ? 'max-h-44' : 'max-h-24';
  const headerTextClass = isSmall ? 'text-[11px]' : 'text-xs';
  const noteSummary = getClipNoteSummary(clip.note);
  const isTrashMode = viewPolicy.state === 'trash';

  return (
    <div
      data-clip-id={clip.id}
      onClick={(e) => {
        if (suppressClickRef.current) {
          e.preventDefault();
          e.stopPropagation();
          suppressClickRef.current = false;
          return;
        }
        onSelect(e);
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
      className={`clip-card relative rounded-xl cursor-pointer select-none border transition-[background-color,border-color,box-shadow,opacity,transform] duration-75 ease-out ${paddingClass} ${
        isDeleting
          ? 'clip-card-deleting'
          : `${isSelected
              ? 'clip-card-selected'
              : `clip-card-idle ${isHovered && !isDragInProgress ? 'clip-card-hovered' : ''}`
            }`
      } ${isDragging ? 'clip-card-drag-source' : ''} ${isTransforming ? 'clip-card-transforming' : ''}`}
    >
      {/* Header Info */}
      <div className={`clip-card-header flex items-center justify-between ${headerTextClass} mb-1`}>
        <div className="flex items-center space-x-2">
          <div className="clip-type-icon theme-badge p-1 rounded border">
            {getIcon()}
          </div>
          <span className="font-medium theme-text-main truncate max-w-[120px]">
            {clip.source_app}
          </span>
        </div>
        <div className="clip-meta-row theme-text-subtle flex items-center text-[11px] font-mono">
          {isTransforming && (
            <span
              role="status"
              aria-label="Applying Transform"
              title="Applying Transform…"
              className="clip-meta-item clip-meta-icon-only clip-transform-working"
            >
              <LoaderCircle className="clip-meta-icon animate-spin" />
            </span>
          )}
          {!isTransforming && transformError && (
            <span
              role="status"
              aria-label="Transform failed"
              title={`Transform failed: ${transformError}`}
              className="clip-meta-item clip-meta-icon-only theme-danger-text"
            >
              <AlertTriangle className="clip-meta-icon" />
            </span>
          )}
          {primaryBinName && (
            <span
              role="img"
              aria-label={`Bin: ${primaryBinName}`}
              title={`Bin: ${primaryBinName}`}
              className="clip-meta-item clip-meta-icon-only"
            >
              <span className="clip-bin-emoji">{formatEmojiIcon(primaryBinIcon)}</span>
            </span>
          )}
          {clip.is_protected && (
            <span
              role="img"
              aria-label="Protected clip"
              title="Protected"
              className="clip-meta-item clip-meta-icon-only clip-protected-accent"
            >
              <Shield className="clip-meta-icon" />
            </span>
          )}
          {clip.is_transformed && (
            <span
              role="img"
              aria-label="Transformed clip"
              title="Transformed"
              className="clip-meta-item clip-meta-icon-only transform-accent pipelines"
            >
              <Workflow className="clip-meta-icon" />
            </span>
          )}
          {queueIndex !== undefined && (
            queueIndex === 1 ? (
              <span className="clip-meta-item clip-queue-next rounded-full font-mono font-extrabold shadow animate-pulse">
                Next Up (#1)
              </span>
            ) : (
              <span className="clip-meta-item clip-queue-position rounded-full font-mono font-semibold">
                #{queueIndex} in Queue
              </span>
            )
          )}
          {clip.content_type === 'image' && clip.text_content && (
            <span
              role="img"
              aria-label="OCR text available"
              title="OCR Text"
              className="clip-meta-item clip-meta-icon-only clip-ocr-accent"
            >
              <ScanText className="clip-meta-icon" />
            </span>
          )}
          {noteSummary && (
            <span title={`Notes: ${noteSummary}`} className="clip-meta-item clip-meta-icon-only">
              <StickyNote className="clip-meta-icon clip-note-accent" />
            </span>
          )}
          {clip.is_pinned && (
            <span title="Pinned" className="clip-meta-item clip-meta-icon-only">
              <Pin className="clip-meta-icon pin-icon" />
            </span>
          )}
          {isTrashMode && (
            <span role="img" aria-label="Clip in Trash" title="In Trash" className="clip-meta-item clip-meta-icon-only clip-trash-badge">
              <Trash2 className="clip-meta-icon" />
            </span>
          )}
          <span className="clip-meta-time">{formatClipTime(clip.created_at)}</span>
        </div>
      </div>

      {/* Body Content */}
      <div className={`theme-text-main ${lineClampClass} font-mono leading-relaxed break-all`}>
        {clip.content_type === 'image' && clip.image_base64 ? (
          <div className="clip-thumbnail-stage relative rounded border overflow-hidden p-1 flex justify-center">
            <img
              src={clip.image_base64}
              alt="Clipboard Clip"
              className={`${imgMaxHeightClass} object-contain rounded`}
            />
          </div>
        ) : clip.content_type === 'color' ? (
          <div className="clip-thumbnail-stage flex items-center space-x-3 p-2 rounded border">
            <div
              className="theme-divider w-8 h-8 rounded border shadow"
              style={{ backgroundColor: clip.text_content || '#ffffff' }}
            />
            <span className="clip-note-accent font-mono text-xs">
              {clip.text_content}
            </span>
          </div>
        ) : isSensitive && !showRevealed ? (
          <div className="theme-status-warning flex items-center justify-between p-1.5 border rounded-lg text-xs font-mono select-none">
            <span className="tracking-widest font-bold">{maskSensitiveText(clip.text_content)}</span>
            <button
              onClick={(e) => {
                e.stopPropagation();
                setShowRevealed(true);
              }}
              className="clip-sensitive-action ml-2 p-1 rounded transition-colors"
              title="Reveal Sensitive Text"
            >
              <Eye className="w-3.5 h-3.5" />
            </button>
          </div>
        ) : (
          <div className="relative group/sensitive flex items-center justify-between">
            <span>{clip.text_content || 'Empty item'}</span>
            {isSensitive && showRevealed && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  setShowRevealed(false);
                }}
                className="clip-sensitive-action ml-2 p-1 rounded transition-colors shrink-0"
                title="Hide Sensitive Text"
              >
                <EyeOff className="w-3.5 h-3.5" />
              </button>
            )}
          </div>
        )}
      </div>

      {/* Note preview if attached */}
      {noteSummary && (
        <div className="clip-note-summary mt-2 pt-1.5 border-t flex items-center space-x-1.5 text-[11px] font-sans italic">
          <StickyNote className="w-3 h-3 shrink-0" />
          <span className="truncate">{noteSummary}</span>
        </div>
      )}

      {/* Hover Action Buttons */}
      <FloatingActionStrip
        label="Clip actions"
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

        {isQueueMode || queueIndex !== undefined ? (
          <>
            {onPasteQueueItem && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onPasteQueueItem();
                }}
                className="floating-action-button is-accent"
                title="Paste"
              >
                <ArrowRightCircle className="w-3.5 h-3.5" />
              </button>
            )}
            {onRemoveFromQueue && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onRemoveFromQueue();
                }}
                className="floating-action-button is-danger"
                title="Remove from Queue"
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
            <button
              onClick={(e) => {
                e.stopPropagation();
                onPin();
              }}
              className={`floating-action-button ${
                clip.is_pinned ? 'is-warning pin-icon' : ''
              }`}
              title={clip.is_pinned ? UI_COPY.unpin : UI_COPY.pin}
            >
              <Pin className="w-3.5 h-3.5" />
            </button>

            {onToggleProtected && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onToggleProtected();
                }}
                className={`floating-action-button ${
                  clip.is_protected ? 'is-accent' : ''
                }`}
                title={clip.is_protected ? UI_COPY.unprotect : UI_COPY.protect}
              >
                {clip.is_protected ? (
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
                ? 'Clip is Protected. Unprotect first to delete.'
                : trashEnabled
                  ? `${UI_COPY.moveToTrash} (Option-click to delete permanently)`
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
  return (
    prevProps.clip.id === nextProps.clip.id &&
    prevProps.clip.content_type === nextProps.clip.content_type &&
    prevProps.clip.text_content === nextProps.clip.text_content &&
    prevProps.clip.image_base64 === nextProps.clip.image_base64 &&
    prevProps.clip.source_app === nextProps.clip.source_app &&
    prevProps.clip.created_at === nextProps.clip.created_at &&
    prevProps.clip.is_pinned === nextProps.clip.is_pinned &&
    prevProps.clip.pin_order === nextProps.clip.pin_order &&
    prevProps.clip.is_protected === nextProps.clip.is_protected &&
    prevProps.clip.is_transformed === nextProps.clip.is_transformed &&
    prevProps.clip.note === nextProps.clip.note &&
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
    prevProps.primaryBinName === nextProps.primaryBinName &&
    prevProps.primaryBinIcon === nextProps.primaryBinIcon &&
    prevProps.rowHeight === nextProps.rowHeight &&
    prevProps.trashEnabled === nextProps.trashEnabled &&
    prevProps.selectionVersion === nextProps.selectionVersion
  );
});
