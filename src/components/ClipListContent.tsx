import { translate } from '../localization/runtime';
import type { ClipItem } from '../types';
import { getClipViewPolicy } from '../utils/clipViewPolicy';
import { safeInvoke as invoke } from '../utils/tauri';
import type { useAppController } from '../hooks/useAppController';
import { ClipCard } from './ClipCard';
import { EmptyClipList } from './EmptyClipList';
import { PinnedClipShelf } from './PinnedClipShelf';
import { SearchErrorNotice } from './SearchErrorNotice';
import { VirtualClipList } from './VirtualClipList';

type AppController = ReturnType<typeof useAppController>;

export function ClipListContent({ controller }: { controller: AppController }) {
  const { shell, data, navigation, clipView, selection, actions, drag } = controller;
  const { enabledFeatures, appSettings } = shell;
  const { fetchSequentialStatus } = data;
  const { currentTab, selectedBinId, searchQuery } = navigation;
  const {
    displayedClips, queuedIndexMap, searchDisplayQuery, searchFailed, retrySearch,
    currentCollection, clipListRef, handleClipListScroll, isLoadingCurrentCollection,
    pinnedShelfClips, stackedPinnedClipIds, binClipReorder, isQueueCollection,
    queueReorder, reorderIdsForClip, displayedClipsForRender, binsById, hasRestrictedSelection,
  } = clipView;
  const {
    selectedClip, selectedClipIds, clearClipSelection, handleClipSelect,
    selectClipForContextMenu, selectPinnedShelfClip,
  } = selection;
  const {
    handleTogglePin, handleToggleProtected, handleToggleConcealed, handleDeleteClip,
    handleCopyClip, transformingClipIds, transformErrorsByClipId,
  } = actions;
  const {
    draggedClipId, setDraggedClipId, setPointerDropTargetBinId, setPointerDropTargetAction,
    setClipDragPreview, pinnedReorderOffsets, updatePointerDropTarget,
    beginPinnedReorderPreview, updatePinnedReorderPreview, cancelPinnedReorderPreview,
    handleClipPointerDragEnd, hoveredClipId, setHoveredClipId,
  } = drag;

  const renderClip = (clip: ClipItem, index: number) => {
    const queueIndex = isQueueCollection
      ? index + 1
      : clip.text_content
        ? queuedIndexMap.get(clip.text_content)
        : undefined;
    const clipBins = (clip.bin_ids ?? []).flatMap((binId) => {
      const bin = binsById.get(binId);
      return bin ? [bin] : [];
    });
    const baseViewPolicy = getClipViewPolicy(currentTab, clip);
    const {
      queueId: queueReorderId,
      binId: binReorderId,
      stableId: stableReorderId,
    } = reorderIdsForClip(clip, index);
    const viewPolicy = isQueueCollection
      ? { ...baseViewPolicy, canDragClips: displayedClips.length > 1, canOrganize: false, canAssignBins: false }
      : hasRestrictedSelection && selectedClipIds.has(clip.id)
        ? { ...baseViewPolicy, canDragClips: false }
        : baseViewPolicy;

    return <ClipCard
      clip={clip}
      isSelected={selectedClipIds.size > 0 ? selectedClipIds.has(clip.id) : selectedClip?.id === clip.id}
      isHovered={hoveredClipId === clip.id}
      showActions={selectedClip?.id === clip.id}
      isDragging={draggedClipId === clip.id}
      isDragInProgress={draggedClipId !== null}
      isTransforming={transformingClipIds.has(clip.id)}
      transformError={transformErrorsByClipId.get(clip.id)}
      reorderOffsetY={stableReorderId
        ? (queueReorderId ? queueReorder.offsets[stableReorderId] : binClipReorder.offsets[stableReorderId]) ?? 0
        : pinnedReorderOffsets[clip.id] ?? 0}
      stableReorderId={stableReorderId}
      viewPolicy={viewPolicy}
      isQueueMode={isQueueCollection}
      queueIndex={queueIndex}
      bins={clipBins}
      rowHeight={appSettings.rowHeight}
      filePreviewMode={appSettings.filePreviewMode}
      filePreviewMaxMb={appSettings.filePreviewMaxMb}
      trashEnabled={appSettings.enableTrash}
      searchQuery={currentTab === 'search' ? searchDisplayQuery : undefined}
      setDraggedClipId={setDraggedClipId}
      onPointerDragStart={(id) => {
        setHoveredClipId(null);
        if (queueReorderId) queueReorder.beginReorder(queueReorderId);
        else if (binReorderId) binClipReorder.beginReorder(binReorderId);
        else beginPinnedReorderPreview(id);
        setDraggedClipId(id);
      }}
      onPointerDragMove={(x, y) => {
        if (queueReorderId) {
          queueReorder.updateReorder(y);
        } else if (binReorderId) {
          binClipReorder.updateReorder(y);
          updatePointerDropTarget(x, y);
        } else {
          updatePointerDropTarget(x, y);
          updatePinnedReorderPreview(x, y, clip.id);
        }
        setClipDragPreview({ clipId: clip.id, x, y });
      }}
      onPointerDragEnd={queueReorderId
        ? () => { void queueReorder.finishReorder(); setClipDragPreview(null); }
        : binReorderId
          ? (x, y, id) => {
              const externalTarget = document
                .elementFromPoint(x, y)
                ?.closest('[data-bin-drop-id], [data-clip-drop-action]');
              if (externalTarget) {
                binClipReorder.cancel();
                void handleClipPointerDragEnd(x, y, id);
              } else {
                void binClipReorder.finishReorder();
                setPointerDropTargetBinId(null);
                setPointerDropTargetAction(null);
                setClipDragPreview(null);
              }
            }
          : handleClipPointerDragEnd}
      onPointerDragCancel={() => {
        setPointerDropTargetBinId(null);
        setPointerDropTargetAction(null);
        if (queueReorderId) queueReorder.cancel();
        else if (binReorderId) binClipReorder.cancel();
        else cancelPinnedReorderPreview();
        setClipDragPreview(null);
      }}
      onSelect={handleClipSelect}
      onPin={() => handleTogglePin(clip.id)}
      onToggleProtected={() => handleToggleProtected(clip.id)}
      onToggleConcealed={() => handleToggleConcealed(clip.id)}
      onName={() => controller.overlays.handlePromptNameClip(clip)}
      onDelete={(event) => handleDeleteClip(clip.id, event?.altKey)}
      onRestore={() => clipView.clipHistoryFocus.restoreClipToHistory(clip.id)}
      onPurgePermanently={() => data.handlePurgeClipPermanently(clip.id)}
      onRemoveFromQueue={() => {
        const queuePosition = queueIndex !== undefined ? queueIndex - 1 : -1;
        if (queuePosition !== -1) {
          invoke('remove_sequential_item_by_index', { index: queuePosition }).then(fetchSequentialStatus);
        }
      }}
      onPasteQueueItem={() => {
        const queuePosition = queueIndex !== undefined ? queueIndex - 1 : -1;
        if (queuePosition !== -1) {
          invoke('paste_sequential_item_by_index', { index: queuePosition }).then(fetchSequentialStatus);
        }
      }}
      onCopy={() => handleCopyClip(clip)}
      onContextMenu={(event) => {
        event.preventDefault();
        selectClipForContextMenu(clip);
        controller.overlays.setContextMenu({ x: event.clientX, y: event.clientY, clip });
      }}
    />;
  };

  const showEmpty = displayedClips.length === 0 && (
    !isLoadingCurrentCollection
    || (currentCollection?.membership === 'search' && Boolean(searchDisplayQuery))
  );
  const forcedClipIds = [
    ...(selectedClip ? [selectedClip.id] : []),
    ...(currentCollection?.membership === 'all' ? pinnedShelfClips.map((clip) => clip.id) : []),
  ];

  return <div className="relative flex-1 min-h-0">
    {enabledFeatures.pinning && <PinnedClipShelf
      clips={pinnedShelfClips}
      stackedClipIds={stackedPinnedClipIds}
      selectedClipId={selectedClip?.id}
      onSelect={selectPinnedShelfClip}
      onRevealAll={() => clipListRef.current?.scrollTo({ top: 0, behavior: 'smooth' })}
      onWheel={(event) => {
        if (clipListRef.current) clipListRef.current.scrollTop += event.deltaY;
      }}
    />}
    <div
      ref={clipListRef}
      data-clip-list
      className="h-full overflow-y-auto ps-3 pe-3 py-3 custom-scrollbar"
      onScroll={(event) => handleClipListScroll(event.currentTarget)}
      onClick={(event) => {
        if (event.target === event.currentTarget) clearClipSelection();
      }}
    >
      {showEmpty ? (
        searchFailed && currentTab === 'search' ? (
          <div className="flex h-full items-center justify-center p-6">
            <SearchErrorNotice onRetry={retrySearch} />
          </div>
        ) : <EmptyClipList
          currentTab={currentTab}
          searchQuery={currentTab === 'search' ? searchDisplayQuery : searchQuery}
          selectedBin={selectedBinId === null ? undefined : binsById.get(selectedBinId)}
        />
      ) : <>
        {searchFailed && currentTab === 'search' && <SearchErrorNotice onRetry={retrySearch} />}
        <VirtualClipList
          clips={displayedClipsForRender}
          disabled={Boolean(currentCollection?.capabilities.canReorder)}
          forcedClipIds={forcedClipIds}
          rowHeight={appSettings.rowHeight}
          scrollRef={clipListRef}
          renderClip={renderClip}
        />
        {isLoadingCurrentCollection && currentCollection?.membership !== 'search' && (
          <div className="theme-text-muted py-3 text-center text-xs" role="status">
            {translate('app.loadingOlderClips')}
          </div>
        )}
      </>}
    </div>
  </div>;
}
