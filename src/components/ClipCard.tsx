import React, { useState } from 'react';
import { formatClipTime } from '../utils/date';
import { ClipItem, getClipNoteSummary, isSensitiveText, maskSensitiveText } from '../types';
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
  ShieldOff,
  GripVertical,
} from 'lucide-react';

interface ClipCardProps {
  clip: ClipItem;
  isSelected: boolean;
  isDeleting?: boolean;
  isTrashMode?: boolean;
  isQueueMode?: boolean;
  queueIndex?: number;
  rowHeight?: 'small' | 'medium' | 'large';
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
  isDeleting = false,
  isTrashMode = false,
  isQueueMode = false,
  queueIndex,
  rowHeight = 'medium',
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
        return <Code className="w-3.5 h-3.5 text-emerald-400" />;
      case 'image':
        return <ImageIcon className="w-3.5 h-3.5 text-pink-400" />;
      case 'color':
        return <Palette className="w-3.5 h-3.5 text-amber-400" />;
      case 'link':
        return <LinkIcon className="w-3.5 h-3.5 text-blue-400" />;
      default:
        return <FileText className="w-3.5 h-3.5 text-gray-400" />;
    }
  };

  const isSmall = rowHeight === 'small';
  const isLarge = rowHeight === 'large';

  const paddingClass = isSmall ? 'p-2.5' : isLarge ? 'p-4' : 'p-3';
  const lineClampClass = isSmall ? 'line-clamp-1 text-[11px]' : isLarge ? 'line-clamp-5 text-xs' : 'line-clamp-2 text-xs';
  const imgMaxHeightClass = isSmall ? 'max-h-16' : isLarge ? 'max-h-44' : 'max-h-24';
  const headerTextClass = isSmall ? 'text-[11px]' : 'text-xs';
  const noteSummary = getClipNoteSummary(clip.note);

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
      onPointerDown={(e) => {
        if (e.button !== 0 || (e.target as HTMLElement).closest('button, input, select, textarea, a')) return;
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
          removePointerListenersRef.current = null;
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

        removePointerListenersRef.current?.();
        removePointerListenersRef.current = removeListeners;
        window.addEventListener('pointermove', handlePointerMove, { passive: false });
        window.addEventListener('pointerup', handlePointerEnd);
        window.addEventListener('pointercancel', handlePointerCancel);
      }}
      className={`clip-card group relative rounded-xl cursor-pointer select-none border transition-[background-color,border-color,box-shadow] duration-75 ease-out ${paddingClass} ${
        isDeleting
          ? 'clip-card-deleting'
          : `${isSelected
              ? 'clip-card-selected bg-[#2f2f2f] border-[#444444] shadow-md ring-1 ring-white/10'
              : 'clip-card-idle bg-[#212121] hover:bg-[#262626] border-[#2f2f2f] hover:border-[#383838] hover:shadow-md'
            }`
      }`}
    >
      {/* Header Info */}
      <div className={`flex items-center justify-between ${headerTextClass} text-gray-400 mb-1`}>
        <div className="flex items-center space-x-2">
          {clip.is_pinned && (
            <div title="Drag to reorder pinned clip" className="p-0.5 text-gray-500 hover:text-orange-400 cursor-grab active:cursor-grabbing">
              <GripVertical className="w-3.5 h-3.5" />
            </div>
          )}
          <div className="p-1 rounded bg-gray-900/90 border border-gray-700/60">
            {getIcon()}
          </div>
          <span className="font-medium text-gray-300 truncate max-w-[120px]">
            {clip.source_app}
          </span>
        </div>
        <div className="flex items-center space-x-1.5 text-[11px] font-mono text-gray-500">
          {clip.is_protected && (
            <span title="Clip is Protected against deletion" className="px-1.5 py-0.5 rounded bg-cyan-950/90 border border-cyan-500/40 text-cyan-300 text-[10px] font-sans font-bold flex items-center space-x-1">
              <Shield className="w-3 h-3 text-cyan-400" />
              <span>Protected</span>
            </span>
          )}
          {queueIndex !== undefined && (
            queueIndex === 1 ? (
              <span className="px-2 py-0.5 rounded-full bg-purple-600 text-white font-mono text-[10px] font-extrabold shadow animate-pulse">
                Next Up (#1)
              </span>
            ) : (
              <span className="px-2 py-0.5 rounded-full bg-purple-950/90 text-purple-300 border border-purple-500/40 font-mono text-[10px] font-semibold">
                #{queueIndex} in Queue
              </span>
            )
          )}
          {clip.content_type === 'image' && clip.text_content && (
            <span title="OCR Text Recognized" className="px-1 py-0.5 rounded bg-cyan-950/80 border border-cyan-800/60 text-cyan-300 text-[9px] font-sans font-bold flex items-center space-x-0.5">
              <ScanText className="w-2.5 h-2.5" />
              <span>OCR</span>
            </span>
          )}
          {noteSummary && (
            <span title={`Notes: ${noteSummary}`}>
              <StickyNote className="w-3 h-3 text-amber-400" />
            </span>
          )}
          <span>{formatClipTime(clip.created_at)}</span>
        </div>
      </div>

      {/* Body Content */}
      <div className={`text-gray-200 ${lineClampClass} font-mono leading-relaxed break-all`}>
        {clip.content_type === 'image' && clip.image_base64 ? (
          <div className="relative rounded border border-gray-800 overflow-hidden bg-black/60 p-1 flex justify-center">
            <img
              src={clip.image_base64}
              alt="Clipboard Clip"
              className={`${imgMaxHeightClass} object-contain rounded`}
            />
          </div>
        ) : clip.content_type === 'color' ? (
          <div className="flex items-center space-x-3 p-2 bg-gray-950/80 rounded border border-gray-800">
            <div
              className="w-8 h-8 rounded border border-gray-700 shadow"
              style={{ backgroundColor: clip.text_content || '#ffffff' }}
            />
            <span className="font-mono text-xs text-amber-300">
              {clip.text_content}
            </span>
          </div>
        ) : isSensitive && !showRevealed ? (
          <div className="flex items-center justify-between p-1.5 bg-amber-950/40 border border-amber-500/40 rounded-lg text-amber-200 text-xs font-mono select-none">
            <span className="tracking-widest font-bold text-amber-300">{maskSensitiveText(clip.text_content)}</span>
            <button
              onClick={(e) => {
                e.stopPropagation();
                setShowRevealed(true);
              }}
              className="ml-2 p-1 hover:bg-amber-900/60 rounded text-amber-400 hover:text-amber-200 transition-colors"
              title="Click to reveal sensitive key/secret"
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
                className="ml-2 p-1 hover:bg-gray-800 rounded text-gray-400 hover:text-white transition-colors shrink-0"
                title="Hide sensitive key/secret"
              >
                <EyeOff className="w-3.5 h-3.5" />
              </button>
            )}
          </div>
        )}
      </div>

      {/* Note preview if attached */}
      {noteSummary && (
        <div className="mt-2 pt-1.5 border-t border-amber-500/20 flex items-center space-x-1.5 text-[11px] text-amber-300 font-sans italic">
          <StickyNote className="w-3 h-3 text-amber-400 shrink-0" />
          <span className="truncate">{noteSummary}</span>
        </div>
      )}

      {/* Hover Action Buttons */}
      <div
        onPointerDown={(e) => e.stopPropagation()}
        onMouseDown={(e) => e.stopPropagation()}
        className="absolute right-2 bottom-2 opacity-0 group-hover:opacity-100 transition-opacity flex items-center space-x-1 bg-gray-950/95 p-1 rounded-lg border border-gray-700/80 shadow-xl"
      >
        <button
          onClick={handleCopy}
          className="p-1 text-gray-400 hover:text-white rounded hover:bg-gray-800"
          title="Copy to Clipboard"
        >
          {copied ? (
            <Check className="w-3.5 h-3.5 text-emerald-400" />
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
                className="p-1 text-purple-400 hover:text-purple-300 rounded hover:bg-gray-800"
                title="Paste this Queued Item"
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
                className="p-1 text-rose-400 hover:text-rose-300 rounded hover:bg-gray-800"
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
              className="p-1 text-cyan-400 hover:text-cyan-300 rounded hover:bg-gray-800"
              title="Restore Clip from Trash"
            >
              <RotateCcw className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onPurgePermanently?.();
              }}
              className="p-1 text-red-400 hover:text-red-300 rounded hover:bg-gray-800"
              title="Delete Permanently"
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
              className={`p-1 rounded hover:bg-gray-800 ${
                clip.is_pinned ? 'text-orange-500 fill-orange-500/20 pin-icon' : 'text-gray-400 hover:text-white'
              }`}
              title={clip.is_pinned ? 'Unpin' : 'Pin Clip'}
            >
              <Pin className="w-3.5 h-3.5" />
            </button>

            {onToggleProtected && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  onToggleProtected();
                }}
                className={`p-1 rounded hover:bg-gray-800 ${
                  clip.is_protected ? 'text-cyan-400' : 'text-gray-400 hover:text-cyan-300'
                }`}
                title={clip.is_protected ? 'Unprotect Clip' : 'Protect Clip'}
              >
                {clip.is_protected ? (
                  <ShieldOff className="w-3.5 h-3.5 text-cyan-400" />
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
              className={`p-1 rounded transition-colors ${
                clip.is_protected
                  ? 'text-gray-600 cursor-not-allowed opacity-50'
                  : 'text-gray-400 hover:text-red-400 hover:bg-gray-800'
              }`}
              title={clip.is_protected ? 'Clip is Protected. Unprotect first to delete.' : 'Move to Trash (Option-click to permanently delete)'}
            >
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          </>
        )}
      </div>

      <div className="absolute top-2 right-2 flex items-center space-x-1 pointer-events-none">
        {clip.is_protected && (
          <span title="Protected Clip">
            <Shield className="w-3.5 h-3.5 text-cyan-400 fill-cyan-400/20 shrink-0" />
          </span>
        )}
        {clip.is_pinned && (
          <span title="Pinned Clip">
            <Pin className="w-3.5 h-3.5 text-orange-500 fill-orange-500 pin-icon shrink-0" />
          </span>
        )}
      </div>
    </div>
  );
};

export const ClipCard = React.memo(ClipCardComponent, (prevProps, nextProps) => {
  return (
    prevProps.clip.id === nextProps.clip.id &&
    prevProps.clip.is_pinned === nextProps.clip.is_pinned &&
    prevProps.clip.is_protected === nextProps.clip.is_protected &&
    prevProps.clip.note === nextProps.clip.note &&
    prevProps.isSelected === nextProps.isSelected &&
    prevProps.isDeleting === nextProps.isDeleting &&
    prevProps.rowHeight === nextProps.rowHeight
  );
});
