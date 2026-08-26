import React from 'react';
import { Edit3, Plus, Sparkles, Trash2 } from 'lucide-react';

import { translate } from '../localization/runtime';
import type { Bin } from '../types';
import { binTextColor } from '../utils/binColor';
import { getClipCollection } from '../utils/clipCollections';
import { formatEmojiIcon } from '../utils/emoji';
import { activateSidebarBin, activateSidebarBinFromKeyboard } from '../utils/sidebarBinActivation';
import { OverflowText } from './OverflowText';

interface SidebarBinsSectionProps {
  bins: Bin[];
  isOpen: boolean;
  setIsOpen: (open: boolean) => void;
  currentTab: string;
  selectedBinId: number | null;
  isClipDragging: boolean;
  draggedClipId?: number | null;
  disabledDropBinId?: number | null;
  pointerDropTargetBinId?: number | null;
  activeDragBinId: number | null;
  reorderOffsets: Record<string, number>;
  isReorderSettling: boolean;
  isHoverMuted: boolean;
  hoveredControl: string | null;
  binListRef: React.RefObject<HTMLElement | null>;
  onStartBinDrag: (id: string, event: React.PointerEvent<HTMLElement>) => void;
  consumeBinDragClick: () => boolean;
  onOpenNewBin: () => void;
  onSelectBin: (id: number) => void;
  onClipDropOnBin?: (clipId: number, binId: number) => void;
  onEditBin?: (bin: Bin) => void;
  onDeleteBin?: (bin: Bin) => void;
  onBinContextMenu?: (x: number, y: number, bin: Bin) => void;
}

export function SidebarBinsSection({
  bins,
  isOpen,
  setIsOpen,
  currentTab,
  selectedBinId,
  isClipDragging,
  draggedClipId,
  disabledDropBinId,
  pointerDropTargetBinId,
  activeDragBinId,
  reorderOffsets,
  isReorderSettling,
  isHoverMuted,
  hoveredControl,
  binListRef,
  onStartBinDrag,
  consumeBinDragClick,
  onOpenNewBin,
  onSelectBin,
  onClipDropOnBin,
  onEditBin,
  onDeleteBin,
  onBinContextMenu,
}: SidebarBinsSectionProps) {
  const [dropTargetBinId, setDropTargetBinId] = React.useState<number | null>(null);

  return (
    <div>
      <div
        data-sidebar-hover-key="section:bins"
        onClick={isClipDragging ? undefined : () => setIsOpen(!isOpen)}
        className={`px-2.5 pb-1 flex items-center justify-between select-none ${isClipDragging ? 'cursor-default' : 'cursor-pointer'}`}
        title={translate('component.sidebar.toggleBins')}
      >
        <span className={`sidebar-section-label text-[11px] font-semibold transition-colors tracking-tight ${hoveredControl === 'section:bins' ? 'is-hovered' : ''}`}>
          {translate('component.sidebar.bins')}
        </span>
        <button
          data-sidebar-hover-key="create-bin"
          onClick={(event) => {
            event.stopPropagation();
            onOpenNewBin();
          }}
          disabled={isClipDragging}
          className={`sidebar-add-btn p-0.5 rounded transition-colors ${isClipDragging ? 'cursor-default' : `cursor-pointer ${hoveredControl === 'create-bin' ? 'is-hovered' : ''}`}`}
          title={translate('component.sidebar.newBin')}
        >
          <Plus className="w-3.5 h-3.5" strokeWidth={2} />
        </button>
      </div>
      <div className={`grid transition-[grid-template-rows,opacity] duration-150 ease-in-out ${isOpen ? 'grid-rows-[1fr] opacity-100' : 'grid-rows-[0fr] opacity-0'}`}>
        <nav
          ref={binListRef}
          className={`min-h-0 space-y-0.5 ${isOpen ? 'overflow-visible' : 'overflow-hidden'} ${isReorderSettling ? 'is-settling-stable-reorder' : ''}`}
        >
          {bins.map((bin) => {
            const isDragging = activeDragBinId === bin.id;
            const binCollection = getClipCollection('bin', bin);
            const isManualBin = Boolean(binCollection?.capabilities.acceptsClipDrop);
            const isDisabledDropTarget = isClipDragging && disabledDropBinId === bin.id;
            const isIneligibleSmartBin = isClipDragging && !isManualBin;
            const isDropTarget = (dropTargetBinId === bin.id || pointerDropTargetBinId === bin.id)
              && isManualBin && !isDisabledDropTarget;
            const isBinHovered = !isHoverMuted && hoveredControl === `bin:${bin.id}`;
            const dropAccent = binTextColor(bin.color) ?? 'var(--accent-primary)';
            return (
              <div
                key={bin.id}
                data-stable-reorder-id={String(bin.id)}
                style={{
                  '--sidebar-bin-drop-color': dropAccent,
                  ...(reorderOffsets[bin.id] !== undefined ? {
                    transform: `translateY(${reorderOffsets[bin.id]}px)`,
                    zIndex: activeDragBinId === bin.id ? 20 : 10,
                  } : {}),
                } as React.CSSProperties}
                data-sidebar-hover-key={`bin:${bin.id}`}
                data-bin-drop-id={isManualBin && !isDisabledDropTarget ? bin.id : undefined}
                role="button"
                tabIndex={0}
                title={isDisabledDropTarget
                  ? translate('component.sidebar.alreadyInThisBin')
                  : isIneligibleSmartBin ? translate('component.sidebar.smartBinAutomatic') : undefined}
                onPointerDown={(event) => onStartBinDrag(String(bin.id), event)}
                onDragOver={(event) => {
                  if (!isManualBin || isDisabledDropTarget) return;
                  event.preventDefault();
                  event.stopPropagation();
                  event.dataTransfer.dropEffect = 'copy';
                  if (dropTargetBinId !== bin.id) setDropTargetBinId(bin.id);
                }}
                onDragEnter={(event) => {
                  if (!isManualBin || isDisabledDropTarget) return;
                  event.preventDefault();
                  event.stopPropagation();
                  event.dataTransfer.dropEffect = 'copy';
                  setDropTargetBinId(bin.id);
                }}
                onDragLeave={(event) => {
                  if (!isManualBin || isDisabledDropTarget) return;
                  event.preventDefault();
                  if (event.relatedTarget && event.currentTarget.contains(event.relatedTarget as Node)) return;
                  const rect = event.currentTarget.getBoundingClientRect();
                  if (event.clientX >= rect.left && event.clientX <= rect.right
                    && event.clientY >= rect.top && event.clientY <= rect.bottom) return;
                  setDropTargetBinId((current) => current === bin.id ? null : current);
                }}
                onDrop={(event) => {
                  if (!isManualBin || isDisabledDropTarget) return;
                  event.preventDefault();
                  event.stopPropagation();
                  setDropTargetBinId(null);
                  const parsedClip = parseInt(event.dataTransfer.getData('clip_id'), 10);
                  const parsedText = parseInt(event.dataTransfer.getData('text/plain'), 10);
                  const targetClipId = !Number.isNaN(parsedClip) && parsedClip > 0
                    ? parsedClip
                    : !Number.isNaN(parsedText) && parsedText > 0 ? parsedText : draggedClipId;
                  if (targetClipId) onClipDropOnBin?.(targetClipId, bin.id);
                }}
                onClick={() => activateSidebarBin(bin.id, activeDragBinId, consumeBinDragClick, onSelectBin)}
                onKeyDown={(event) => activateSidebarBinFromKeyboard(event, bin.id, activeDragBinId, onSelectBin)}
                onContextMenu={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  onBinContextMenu?.(event.clientX, event.clientY, bin);
                }}
                className={`sidebar-nav-row justify-between select-none transition-[background-color,border-color,color,box-shadow,opacity,transform] duration-100 ${
                  isDisabledDropTarget || isIneligibleSmartBin ? 'cursor-not-allowed'
                    : isClipDragging ? 'cursor-grabbing' : 'cursor-pointer active:cursor-grabbing'
                } ${isDropTarget ? 'sidebar-bin-drop-target'
                  : isDisabledDropTarget || isIneligibleSmartBin ? 'sidebar-bin-ineligible'
                  : isClipDragging && isManualBin ? 'sidebar-bin-drop-eligible font-normal'
                  : isDragging ? 'sidebar-bin-drag-source rounded-md relative pointer-events-none'
                  : currentTab === 'bin' && selectedBinId === bin.id ? 'sidebar-item-active font-medium'
                  : isBinHovered ? 'sidebar-item-hovered font-normal' : 'sidebar-item-idle font-normal'}`}
              >
                <div className="flex items-center gap-3 truncate pe-1 min-w-0">
                  <span className="sidebar-nav-icon sidebar-nav-icon-emoji sidebar-icon-primary text-sm">{formatEmojiIcon(bin.icon)}</span>
                  <OverflowText text={bin.name} className="truncate" style={{ color: binTextColor(bin.color) }} />
                </div>
                <div className="flex items-center justify-end shrink-0 ps-1">
                  <div className={`flex items-center space-x-1.5 ${isBinHovered && !isDragging ? 'hidden' : ''}`}>
                    {bin.smart_rule && (bin.clip_count ?? 0) > 0 ? (
                      <span title={translate('component.sidebar.smartBinCountMatches', { count: bin.clip_count ?? 0 })} className="sidebar-badge text-[11px] px-1.5 py-0.5 rounded-md font-mono flex items-center space-x-1">
                        <Sparkles className="theme-note-text w-3 h-3 shrink-0" /><span>{bin.clip_count}</span>
                      </span>
                    ) : !!bin.clip_count && bin.clip_count > 0 && (
                      <span className="sidebar-badge text-[11px] px-1.5 py-0.5 rounded-md font-mono">{bin.clip_count}</span>
                    )}
                  </div>
                  <div className={`${isBinHovered && !isDragging ? 'flex' : 'hidden'} items-center space-x-1`}>
                    <button type="button" onPointerDown={(event) => event.stopPropagation()} onMouseDown={(event) => event.stopPropagation()} onClick={(event) => { event.stopPropagation(); onEditBin?.(bin); }} className="sidebar-row-action is-edit p-1 rounded transition-colors cursor-pointer" title={translate('component.sidebar.editBin')}>
                      <Edit3 className="w-3.5 h-3.5" />
                    </button>
                    <button type="button" onPointerDown={(event) => event.stopPropagation()} onMouseDown={(event) => event.stopPropagation()} onClick={(event) => { event.stopPropagation(); onDeleteBin?.(bin); }} className="sidebar-row-action is-danger p-1 rounded transition-colors cursor-pointer" title={translate('component.sidebar.deleteBin')}>
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              </div>
            );
          })}
        </nav>
      </div>
    </div>
  );
}
