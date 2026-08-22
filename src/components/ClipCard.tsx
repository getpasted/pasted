import React from 'react';

import { useClipCardPointerDrag } from '../hooks/useClipCardPointerDrag';
import { useFeatures } from '../hooks/useFeatures';
import { useLocalization } from '../localization/LocalizationProvider';
import { getClipNoteSummary } from '../types';
import { clipConcealmentPolicy } from '../utils/clipConcealment';
import { ClipCardActions } from './ClipCardActions';
import { ClipCardContent } from './ClipCardContent';
import { ClipCardHeader } from './ClipCardHeader';
import { clipCardPropsEqual, type ClipCardProps } from './clipCardModel';
import { useContentTypes } from './ContentTypeProvider';

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
  const features = useFeatures();
  const { definitions: contentTypeDefinitions } = useContentTypes();
  const { consumeSuppressedClick, onPointerDown } = useClipCardPointerDrag({
    clipId: clip.id,
    canDragClips: viewPolicy.canDragClips,
    setDraggedClipId,
    onPointerDragStart,
    onPointerDragMove,
    onPointerDragEnd,
    onPointerDragCancel,
  });
  const primaryContentType = clip.content_types?.[0] ?? clip.content_type;
  const concealment = clipConcealmentPolicy(clip, bins, contentTypeDefinitions);
  const isSensitive = features.concealment && concealment.effective;
  const isSmall = rowHeight === 'small';
  const isLarge = rowHeight === 'large';
  const paddingClass = isSmall ? 'p-2' : isLarge ? 'p-5' : 'p-3.5';
  const lineClampClass = isSmall
    ? 'line-clamp-1 text-[11px]'
    : isLarge ? 'line-clamp-5 text-[13px]' : 'line-clamp-2 text-xs';
  const imageMaxHeightClass = isSmall ? 'max-h-16' : isLarge ? 'max-h-44' : 'max-h-24';
  const imagePlaceholderHeightClass = isSmall ? 'min-h-16' : isLarge ? 'min-h-44' : 'min-h-24';
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

  return <div
    data-clip-id={clip.id}
    data-stable-reorder-id={stableReorderId}
    data-pinned-clip={features.pinning && clip.is_pinned ? 'true' : undefined}
    onClick={(event) => {
      if (!consumeSuppressedClick(event)) onSelect(clip, event);
    }}
    onContextMenu={onContextMenu}
    draggable={false}
    style={reorderOffsetY !== 0 || isDragging ? {
      transform: `translateY(${reorderOffsetY}px)`,
      zIndex: isDragging ? 20 : 10,
    } : undefined}
    onPointerDown={onPointerDown}
    className={`clip-card relative cursor-pointer select-none border transition-[background-color,border-color,box-shadow,opacity,transform] duration-75 ease-out ${paddingClass} ${
      isDeleting
        ? 'clip-card-deleting'
        : `${isSelected
          ? 'clip-card-selected'
          : `clip-card-idle ${isHovered && !isDragInProgress ? 'clip-card-hovered' : ''}`
        }`
    } ${attributeTintClass} ${isDragging ? 'clip-card-drag-source' : ''} ${isTransforming ? 'clip-card-transforming' : ''}`}
  >
    <ClipCardHeader
      bins={bins}
      clip={clip}
      features={features}
      headerSpacingClass={headerSpacingClass}
      headerTextClass={headerTextClass}
      isTransforming={isTransforming}
      isTrashMode={isTrashMode}
      noteSummary={noteSummary}
      primaryContentType={primaryContentType}
      protectedByBin={protectedByBin}
      queueIndex={queueIndex}
      searchQuery={searchQuery}
      transformError={transformError}
    />
    <ClipCardContent
      clip={clip}
      features={features}
      filePreviewMaxMb={filePreviewMaxMb}
      filePreviewMode={filePreviewMode}
      imageMaxHeightClass={imageMaxHeightClass}
      imagePlaceholderHeightClass={imagePlaceholderHeightClass}
      isSensitive={isSensitive}
      isSmall={isSmall}
      lineClampClass={lineClampClass}
      noteSummary={noteSummary}
      searchQuery={searchQuery}
    />
    <ClipCardActions
      clip={clip}
      concealmentEffective={concealment.effective}
      features={features}
      isDragInProgress={isDragInProgress}
      isQueueMode={isQueueMode}
      isTrashMode={isTrashMode}
      onCopy={onCopy}
      onDelete={onDelete}
      onName={onName}
      onPasteQueueItem={onPasteQueueItem}
      onPin={onPin}
      onPurgePermanently={onPurgePermanently}
      onRemoveFromQueue={onRemoveFromQueue}
      onRestore={onRestore}
      onToggleConcealed={onToggleConcealed}
      onToggleProtected={onToggleProtected}
      protectedByBin={protectedByBin}
      protectionToggleDisabled={protectionToggleDisabled}
      queueIndex={queueIndex}
      showActions={showActions}
      trashEnabled={trashEnabled}
      viewPolicy={viewPolicy}
    />
  </div>;
};

export const ClipCard = React.memo(ClipCardComponent, clipCardPropsEqual);
