import type { MouseEvent } from 'react';

import type { Bin, ClipItem } from '../types.ts';
import type { ClipViewPolicy } from '../utils/clipViewPolicy.ts';

export interface ClipCardProps {
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
  onSelect: (clip: ClipItem, event: MouseEvent) => void;
  onPin: () => void;
  onToggleProtected?: () => void;
  onToggleConcealed?: () => void;
  onName?: () => void;
  onDelete: (event?: MouseEvent) => void;
  onCopy: () => void;
  onRestore?: () => void;
  onPurgePermanently?: () => void;
  onRemoveFromQueue?: () => void;
  onPasteQueueItem?: () => void;
  onContextMenu: (event: MouseEvent) => void;
  setDraggedClipId?: (id: number | null) => void;
  onPointerDragStart?: (id: number) => void;
  onPointerDragMove?: (x: number, y: number) => void;
  onPointerDragEnd?: (x: number, y: number, id: number) => void;
  onPointerDragCancel?: () => void;
}

function equalOrderedValues<T>(previous: T[] | undefined, next: T[] | undefined) {
  const previousValues = previous ?? [];
  const nextValues = next ?? [];
  return previousValues.length === nextValues.length
    && previousValues.every((value, index) => value === nextValues[index]);
}

function equalBins(previous: Bin[], next: Bin[]) {
  return previous.length === next.length && previous.every((bin, index) => {
    const nextBin = next[index];
    return bin.id === nextBin?.id
      && bin.name === nextBin.name
      && bin.icon === nextBin.icon
      && bin.color === nextBin.color
      && bin.smart_rule === nextBin.smart_rule
      && bin.protect_clips === nextBin.protect_clips
      && bin.conceal_clips === nextBin.conceal_clips;
  });
}

export function clipCardPropsEqual(previous: ClipCardProps, next: ClipCardProps) {
  return previous.clip.id === next.clip.id
    && previous.clip.content_hash === next.clip.content_hash
    && previous.clip.content_type === next.clip.content_type
    && equalOrderedValues(previous.clip.content_types, next.clip.content_types)
    && previous.clip.text_content === next.clip.text_content
    && previous.clip.image_base64 === next.clip.image_base64
    && previous.clip.source === next.clip.source
    && previous.clip.created_at === next.clip.created_at
    && previous.clip.is_pinned === next.clip.is_pinned
    && previous.clip.pin_order === next.clip.pin_order
    && previous.clip.is_protected === next.clip.is_protected
    && previous.clip.is_explicitly_protected === next.clip.is_explicitly_protected
    && previous.clip.is_concealed === next.clip.is_concealed
    && previous.clip.is_explicitly_concealed === next.clip.is_explicitly_concealed
    && previous.clip.is_explicitly_revealed === next.clip.is_explicitly_revealed
    && previous.clip.hotkey === next.clip.hotkey
    && equalOrderedValues(previous.clip.protecting_bin_ids, next.clip.protecting_bin_ids)
    && previous.clip.is_transformed === next.clip.is_transformed
    && previous.clip.note === next.clip.note
    && previous.clip.name === next.clip.name
    && previous.clip.bin_id === next.clip.bin_id
    && equalOrderedValues(previous.clip.bin_ids, next.clip.bin_ids)
    && previous.isSelected === next.isSelected
    && previous.isHovered === next.isHovered
    && previous.showActions === next.showActions
    && previous.isDragging === next.isDragging
    && previous.isDragInProgress === next.isDragInProgress
    && previous.isTransforming === next.isTransforming
    && previous.transformError === next.transformError
    && previous.reorderOffsetY === next.reorderOffsetY
    && previous.isDeleting === next.isDeleting
    && previous.viewPolicy.state === next.viewPolicy.state
    && previous.viewPolicy.canDragClips === next.viewPolicy.canDragClips
    && previous.viewPolicy.canOrganize === next.viewPolicy.canOrganize
    && previous.isQueueMode === next.isQueueMode
    && previous.queueIndex === next.queueIndex
    && equalBins(previous.bins, next.bins)
    && previous.rowHeight === next.rowHeight
    && previous.filePreviewMode === next.filePreviewMode
    && previous.filePreviewMaxMb === next.filePreviewMaxMb
    && previous.searchQuery === next.searchQuery
    && previous.trashEnabled === next.trashEnabled;
}
