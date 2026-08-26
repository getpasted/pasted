import { translate } from '../localization/runtime';
import type { useAppController } from '../hooks/useAppController';
import { AppDialogLayer } from './AppDialogLayer';
import { AppDestinationView } from './AppDestinationView';
import { ClipContextMenuLayer } from './ClipContextMenuLayer';
import { ClipDragPreview } from './ClipDragPreview';
import { ClipListContent } from './ClipListContent';
import { ClipListHeader } from './ClipListHeader';
import { ClipPreview } from './ClipPreview';
import { ClipSelectionBatchActions } from './ClipSelectionBatchActions';
import { MacRtlWindowControls } from './MacRtlWindowControls';
import { SequentialQueueBar } from './SequentialQueueBar';
import { Sidebar } from './Sidebar';

type AppController = ReturnType<typeof useAppController>;

export function AppShellView({ controller }: { controller: AppController }) {
  const { shell, data, navigation, overlays, layout, clipView, selection, actions, drag, handlers } = controller;
  const { direction, enabledFeatures, appSettings, settingsHydrated, initialDataLoaded } = shell;
  const { handleUpdateSettings } = controller.settings;
  const {
    allClips, trashedClips, bins, manualTransforms, seqStatus, totalClipCount,
    clipCollectionSummary, isClipboardPaused, ignoredAppStatus, fetchClips,
    fetchTrashedClips, fetchClipCollectionSummary, fetchBins,
    fetchSequentialStatus, handleToggleClipboardPause, handlePurgeClipPermanently, handleEmptyTrash,
  } = data;
  const {
    currentTab, selectedBinId, setSelectedBinId,
    searchQuery, setSearchQuery, isSidebarCollapsed, setIsSidebarCollapsed, sidebarSections,
    handleSidebarSectionStateChange, navigateToTab, enterSearchView, exitEmptySearch,
  } = navigation;
  const {
    contextMenu, setContextMenu, binContextMenu, setBinContextMenu, isBinModalOpen,
    editingBin, binToDelete, setBinToDelete, notePromptClip, setNotePromptClip,
    notePromptText, setNotePromptText, namePromptClip, setNamePromptClip, namePromptText,
    setNamePromptText, clearHistoryMode, setClearHistoryMode, handleOpenNewBinModal,
    handleEditBin, closeBinModal, handleBinContextMenu, handlePromptAddNote, handlePromptNameClip,
  } = overlays;
  const {
    sidebarWidth, clipsListWidth, isResizingSidebar, isResizingList,
    handleSidebarPointerDown, handleListPointerDown,
  } = layout;
  const {
    queuedIndexMap, searchTotalCount, currentCollection, binClipReorder, isBinCollection,
    isQueueCollection, queueReorder, selectedClipViewPolicy, hasRestrictedSelection, clipHistoryFocus,
  } = clipView;
  const {
    selectedClip, selectedClipIds, clearClipSelection, handleSetSelectedPinned,
  } = selection;
  const {
    handleTogglePin, handleToggleProtected, handleToggleConcealed, handleSetProtected, handleSetConcealed, handleBatchTrash,
    handleDeleteClip, handleCopyClip, assignClipToBin, removeClipFromBin,
    handleRunTransformForClip, handleToggleSequentialStack, handleUpdateClipNoteLocally,
    handleUpdateClipNameLocally, handleDeleteNoteFromClip, transformingClipIds,
    transformErrorsByClipId, handleClearClipName,
  } = actions;
  const {
    draggedClipId, pointerDropTargetBinId, pointerDropTargetAction, clipDragPreview,
    disabledDropBinId, disabledDropActions, isPinnedReorderSettling,
    handleSidebarClipDropOnBin, draggedPreviewClip,
  } = drag;
  const {
    handleToggleCopyQueue, handleSidebarNavigate, handleSidebarBinSelect, handleRequestDeleteBin,
    handleRequestClearHistory, handleAssignBin, handlePreviewClipUpdate, handleClearHistory,
  } = handlers;

  return (
    <div className={`app-shell flex h-screen w-screen overflow-hidden font-sans ${clipDragPreview ? 'cursor-grabbing' : ''} ${
      draggedClipId !== null ? 'is-dragging-clip' : ''
    } ${
      isPinnedReorderSettling || queueReorder.isSettling || binClipReorder.isSettling ? 'is-settling-pinned-reorder' : ''
    } ${
      isResizingSidebar || isResizingList ? 'is-resizing-columns' : ''
    }`}>
      {direction === 'rtl' && <MacRtlWindowControls />}
      {clipDragPreview && draggedPreviewClip && (
        <ClipDragPreview
          clip={draggedPreviewClip}
          x={clipDragPreview.x}
          y={clipDragPreview.y}
          batchCount={selectedClipIds.has(draggedPreviewClip.id) ? selectedClipIds.size : 1}
          showSource={enabledFeatures.sources}
          concealed={enabledFeatures.concealment && Boolean(draggedPreviewClip.is_concealed)}
        />
      )}
      {/* Inline-start application sidebar; platform CSS reserves mirrored macOS traffic lights. */}
      <Sidebar
        currentTab={currentTab}
        setCurrentTab={handleSidebarNavigate}
        selectedBinId={selectedBinId}
        setSelectedBinId={handleSidebarBinSelect}
        bins={bins}
        clipCollectionSummary={clipCollectionSummary}
        features={enabledFeatures}
        onRefreshBins={() => {
          void Promise.all([fetchBins(), fetchClips(), fetchClipCollectionSummary()]);
        }}
        onOpenNewBinModal={handleOpenNewBinModal}
        onEditBin={handleEditBin}
        onDeleteBin={handleRequestDeleteBin}
        onBinContextMenu={handleBinContextMenu}
        onClipDropOnBin={handleSidebarClipDropOnBin}
        draggedClipId={draggedClipId}
        pointerDropTargetBinId={pointerDropTargetBinId}
        pointerDropTargetAction={pointerDropTargetAction}
        disabledDropBinId={disabledDropBinId}
        disabledDropActions={disabledDropActions}
        searchQuery={searchQuery}
        setSearchQuery={setSearchQuery}
        onSearchFocus={enterSearchView}
        onEmptySearchEscape={exitEmptySearch}
        seqStatus={seqStatus}
        onClearHistory={handleRequestClearHistory}
        totalClipCount={totalClipCount}
        isCollapsed={isSidebarCollapsed}
        setIsCollapsed={setIsSidebarCollapsed}
        sidebarWidth={sidebarWidth}
        sectionState={sidebarSections}
        onSectionStateChange={handleSidebarSectionStateChange}
      />

      {/* Sidebar Resizer Handle (Only active when sidebar is expanded) */}
      {!isSidebarCollapsed && (
        <div
          onPointerDown={handleSidebarPointerDown}
          className="column-resizer relative w-[1px] h-screen cursor-col-resize z-30 shrink-0 select-none touch-none"
          title={translate('app.resizeSidebar')}
        >
          <div className={`column-resizer-line w-[1px] h-full transition-colors ${isResizingSidebar ? 'is-active' : ''}`} />
          <div className="absolute -inset-x-1 inset-y-0 z-40 cursor-col-resize" />
        </div>
      )}

      <AppDestinationView controller={controller} renderClipWorkspace={() => (
        <div className="flex-1 h-screen flex overflow-hidden">
          {/* Middle Clips List Panel */}
          <div
            style={{ width: `${clipsListWidth}px` }}
            className="shrink-0 col-list h-screen flex flex-col overflow-hidden"
          >
            <ClipListHeader
              collection={currentCollection}
              currentTab={currentTab}
              searchTotalCount={searchTotalCount}
              ignoredAppStatus={ignoredAppStatus}
              trashIsEmpty={trashedClips.length === 0}
              onEmptyTrash={handleEmptyTrash}
              clipboardPaused={isClipboardPaused}
              onToggleClipboardPause={handleToggleClipboardPause}
              queueEnabled={enabledFeatures.queue}
              queueStatus={seqStatus}
              onToggleQueue={handleToggleCopyQueue}
            />

            {/* Sequential Paste Top Header Banner if active */}
            {isQueueCollection && (
              <div className={`queue-controls-region p-3 border-b ${seqStatus?.is_active ? 'is-active' : ''}`}>
                <SequentialQueueBar
                  status={seqStatus}
                  onRefresh={fetchSequentialStatus}
                />
              </div>
            )}

            <ClipListContent controller={controller} />

            <ClipSelectionBatchActions
              selectedClipIds={selectedClipIds} collection={currentCollection} viewPolicy={selectedClipViewPolicy}
              hasRestrictedSelection={hasRestrictedSelection} pinningEnabled={enabledFeatures.pinning}
              trashEnabled={appSettings.enableTrash} onSetPinned={handleSetSelectedPinned} onTrash={handleBatchTrash}
              onUnprotect={(ids) => handleSetProtected(ids[0], false)}
              onReveal={(ids) => handleSetConcealed(ids[0], false)}
              onRestore={(ids) => void clipHistoryFocus.restoreClipsFromTrash(ids)}
              onDeletePermanently={(ids) => void Promise.all(ids.map(handlePurgeClipPermanently))}
              onClearSelection={clearClipSelection}
            />
          </div>

          {/* List resizer with a 1px visual line and an inline-end grab target. */}
          <div
            onPointerDown={handleListPointerDown}
            className="column-resizer relative w-[1px] h-screen cursor-col-resize z-20 shrink-0 select-none touch-none"
            title={translate('app.resizeClipList')}
          >
            <div className={`column-resizer-line w-[1px] h-full transition-colors ${isResizingList ? 'is-active' : ''}`} />
            <div className="absolute inset-y-0 start-0 -end-2 z-20 cursor-col-resize" />
          </div>

          {/* Inline-end detail preview panel. */}
          <ClipPreview
            clip={selectedClip}
            viewPolicy={selectedClipViewPolicy}
            bins={bins}
            viewedBinId={isBinCollection ? selectedBinId : null}
            manualTransforms={manualTransforms}
            onUpdateClip={handlePreviewClipUpdate}
            onAssignBin={handleAssignBin}
            onRemoveBin={removeClipFromBin}
            onTogglePin={handleTogglePin}
            onToggleProtected={handleToggleProtected}
            onToggleConcealed={handleToggleConcealed}
            onName={handlePromptNameClip}
            onDeleteClip={selectedClipViewPolicy.state === 'trash' ? handlePurgeClipPermanently : handleDeleteClip}
            onRestoreClip={clipHistoryFocus.restoreClipToHistory}
            onUpdateClipNote={handleUpdateClipNoteLocally}
            isTransforming={selectedClip ? transformingClipIds.has(selectedClip.id) : false}
            transformError={selectedClip ? transformErrorsByClipId.get(selectedClip.id) : undefined}
            onOpenTransformations={() => navigateToTab('transformations')}
            onOpenIntelligence={() => navigateToTab('settings:intelligence')}
            trashEnabled={appSettings.enableTrash}
            filePreviewMode={appSettings.filePreviewMode}
            filePreviewMaxMb={appSettings.filePreviewMaxMb}
          />
        </div>
      )} />

      <ClipContextMenuLayer
        menu={contextMenu}
        setMenu={setContextMenu}
        clips={allClips}
        trashedClips={trashedClips}
        currentTab={currentTab}
        selectedClipIds={selectedClipIds}
        bins={bins}
        queuedIndexMap={queuedIndexMap}
        trashEnabled={appSettings.enableTrash}
        onCopy={handleCopyClip}
        onAssignBin={(clipId, binId) => assignClipToBin(clipId, binId, { includeSelection: true })}
        onRemoveBin={removeClipFromBin}
        onRunTransform={handleRunTransformForClip}
        onOpenTransformations={() => navigateToTab('transformations')}
        onName={handlePromptNameClip}
        onClearName={handleClearClipName}
        onAddNote={handlePromptAddNote}
        onDeleteNote={handleDeleteNoteFromClip}
        onToggleQueue={(clip) => void handleToggleSequentialStack(clip)}
        onTogglePin={handleTogglePin}
        onToggleProtected={handleToggleProtected}
        onToggleConcealed={handleToggleConcealed}
        onDelete={(clipId, permanently) => handleDeleteClip(clipId, permanently)}
        onRestore={clipHistoryFocus.restoreClipToHistory}
        onPurge={handlePurgeClipPermanently}
      />

      <AppDialogLayer
        features={enabledFeatures}
        settings={appSettings}
        settingsHydrated={settingsHydrated}
        initialDataLoaded={initialDataLoaded}
        updateSettings={handleUpdateSettings}
        bins={bins}
        clipCollectionSummary={clipCollectionSummary}
        selectedBinId={selectedBinId}
        setSelectedBinId={setSelectedBinId}
        navigateToTab={navigateToTab}
        binContextMenu={binContextMenu}
        setBinContextMenu={setBinContextMenu}
        isBinModalOpen={isBinModalOpen}
        editingBin={editingBin}
        editBin={handleEditBin}
        closeBinModal={closeBinModal}
        binToDelete={binToDelete}
        setBinToDelete={setBinToDelete}
        notePromptClip={notePromptClip}
        setNotePromptClip={setNotePromptClip}
        notePromptText={notePromptText}
        setNotePromptText={setNotePromptText}
        updateClipNoteLocally={handleUpdateClipNoteLocally}
        namePromptClip={namePromptClip}
        setNamePromptClip={setNamePromptClip}
        namePromptText={namePromptText}
        setNamePromptText={setNamePromptText}
        updateClipNameLocally={handleUpdateClipNameLocally}
        onNameCleared={(clipId) => clipHistoryFocus.handlePropertyRemoved('name', [clipId])}
        clearHistoryMode={clearHistoryMode}
        setClearHistoryMode={setClearHistoryMode}
        confirmClearHistory={handleClearHistory}
        fetchBins={fetchBins}
        fetchClips={fetchClips}
        fetchTrashedClips={fetchTrashedClips}
        fetchClipCollectionSummary={fetchClipCollectionSummary}
      />
    </div>
  );
}
