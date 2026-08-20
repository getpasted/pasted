import { lazy, Suspense, useEffect, useState, useCallback, useMemo } from 'react';
import { safeInvoke as invoke } from './utils/tauri';
import { ClipItem, Bin } from './types';
import { Sidebar } from './components/Sidebar';
import { ClipCard } from './components/ClipCard';
import { EmptyClipList } from './components/EmptyClipList';
import { PinnedClipShelf } from './components/PinnedClipShelf';
import { ClipPreview } from './components/ClipPreview';
import { SequentialQueueBar } from './components/SequentialQueueBar';
import { ContextMenu } from './components/ContextMenu';
import { QuickHudWindow } from './components/QuickHudWindow';
import { OverflowText } from './components/OverflowText';
import { handleWindowDragDoubleClick, startWindowDrag } from './utils/windowDrag';
import { useColumnResize } from './hooks/useColumnResize';
import { useAppSettings } from './hooks/useAppSettings';
import { useClipViews, useLiveClipSnapshot } from './hooks/useClipViews';
import { getClipViewPolicy } from './utils/clipViewPolicy';
import { getClipCollection } from './utils/clipCollections';
import { useAppData } from './hooks/useAppData';
import { useClipActions } from './hooks/useClipActions';
import { Clipboard, Trash2, Pause, Disc, Square, Search } from 'lucide-react';
import { enabledFeatureRecord } from './utils/features';
import { FeatureProvider } from './hooks/useFeatures';
import { soundManager } from './utils/sound';
import { readAppUiState } from './utils/appUiState';
import './App.css';
import { useLocalization } from './localization/LocalizationProvider';
import { translate } from './localization/runtime';
import { MacRtlWindowControls } from './components/MacRtlWindowControls';
import { SearchErrorNotice } from './components/SearchErrorNotice';
import { clipsApi } from './api/clips';
import { useAppNavigation } from './hooks/useAppNavigation';
import { useAppShell } from './hooks/useAppShell';
import { useAppMenuActions } from './hooks/useAppMenuActions';
import { useClipSelectionController } from './hooks/useClipSelectionController';
import { useClipListViewport } from './hooks/useClipListViewport';
import { ClipBatchActionBar } from './components/ClipBatchActionBar';
import { useAppOverlays } from './hooks/useAppOverlays';
import { useClipReordering } from './hooks/useClipReordering';
import { ClipDragPreview } from './components/ClipDragPreview';
import { useClipDragController } from './hooks/useClipDragController';
import { AppDialogLayer } from './components/AppDialogLayer';
const TransformationsView = lazy(() => import('./components/TransformationsView').then(({ TransformationsView: component }) => ({ default: component })));
const SettingsModal = lazy(() => import('./components/SettingsModal').then(({ SettingsModal: component }) => ({ default: component })));
const ActivityLogView = lazy(() => import('./components/ActivityLogView').then(({ ActivityLogView: component }) => ({ default: component })));
const AnalyticsView = lazy(() => import('./components/AnalyticsView').then(({ AnalyticsView: component }) => ({ default: component })));
const HelpView = lazy(() => import('./components/HelpView').then(({ HelpView: component }) => ({ default: component })));

export default function App() {
  const { catalogReady, direction, locale } = useLocalization();
  const [restoredUiState] = useState(readAppUiState);

  const {
    appSettings,
    blacklistApps,
    settingsHydrated,
    updateSettings: handleUpdateSettings,
    addBlacklistApp: handleAddBlacklistApp,
    removeBlacklistApp: handleRemoveBlacklistApp,
    toggleBlacklistRule: handleToggleBlacklistRule,
  } = useAppSettings();
  const enabledFeatures = useMemo(() => enabledFeatureRecord(appSettings), [appSettings]);

  useEffect(() => {
    soundManager.setEnabled(appSettings.enableSounds);
  }, [appSettings.enableSounds]);

  const {
    allClips,
    setAllClips,
    trashedClips,
    setTrashedClips,
    bins,
    setBins,
    manualTransforms,
    sequentialStatus: seqStatus,
    totalClipCount,
    totalTrashCount,
    clipCollectionSummary,
    setTotalClipCount,
    isClipboardPaused,
    ignoredAppStatus,
    initialDataLoaded,
    fetchClips,
    fetchTrashedClips,
    fetchClipCollectionSummary,
    loadMoreClips,
    loadMoreTrashedClips,
    isLoadingMoreClips,
    isLoadingMoreTrash,
    fetchBins,
    fetchManualTransforms,
    fetchSequentialStatus,
    toggleClipboardPause: handleToggleClipboardPause,
    restoreClip: handleRestoreClip,
    purgeClipPermanently: handlePurgeClipPermanently,
    emptyTrash: handleEmptyTrash,
  } = useAppData();

  const { isHudView } = useAppShell({
    catalogReady,
    direction,
    settingsHydrated,
    initialDataLoaded,
  });

  const [selectedClip, setSelectedClip] = useState<ClipItem | null>(null);
  const [selectedClipIds, setSelectedClipIds] = useState<Set<number>>(new Set());
  const {
    currentTab,
    activeSettingsTab,
    setActiveSettingsTab,
    activeHelpTopic,
    setActiveHelpTopic,
    activeTransformWorkspace,
    setActiveTransformWorkspace,
    selectedBinId,
    setSelectedBinId,
    searchQuery,
    setSearchQuery,
    isSidebarCollapsed,
    setIsSidebarCollapsed,
    sidebarSections,
    handleSidebarSectionStateChange,
    navigateToTab,
    enterSearchView,
    exitEmptySearch,
  } = useAppNavigation({
    restoredUiState,
    enabledFeatures,
    bins,
    startupView: appSettings.startupView,
    settingsHydrated,
    initialDataLoaded,
    isHudView,
    selectedClipId: selectedClip?.id ?? null,
  });
  const {
    contextMenu,
    setContextMenu,
    binContextMenu,
    setBinContextMenu,
    isBinModalOpen,
    setIsBinModalOpen,
    editingBin,
    setEditingBin,
    binToDelete,
    setBinToDelete,
    notePromptClip,
    setNotePromptClip,
    notePromptText,
    setNotePromptText,
    clearHistoryMode,
    setClearHistoryMode,
    openNewBinModal: handleOpenNewBinModal,
    editBin: handleEditBin,
    closeBinModal,
    openBinContextMenu: handleBinContextMenu,
    promptAddNote: handlePromptAddNote,
  } = useAppOverlays({
    binsEnabled: enabledFeatures.bins,
    notesEnabled: enabledFeatures.notes,
  });

  const handleToggleCopyQueue = async () => {
    if (!enabledFeatures.queue) return;
    try {
      if (seqStatus?.is_active) {
        await invoke('stop_sequential_paste');
      } else {
        await invoke('start_sequential_paste');
        navigateToTab('sequential');
        setSelectedBinId(null);
      }
      fetchSequentialStatus();
    } catch (e) {
      console.error('Failed to toggle copy queue:', e);
    }
  };

  useEffect(() => {
    if (!enabledFeatures.queue && seqStatus?.is_active) {
      void invoke('stop_sequential_paste').then(fetchSequentialStatus).catch(console.error);
    }
  }, [enabledFeatures.queue, fetchSequentialStatus, seqStatus?.is_active]);

  const handleSidebarNavigate = useCallback((route: string) => {
    setBinContextMenu(null);
    navigateToTab(route);
  }, [navigateToTab]);

  const handleSidebarBinSelect = useCallback((binId: number | null) => {
    setBinContextMenu(null);
    setSelectedBinId(binId);
  }, []);

  const {
    sidebarWidth,
    clipsListWidth,
    isResizingSidebar,
    isResizingList,
    handleSidebarPointerDown,
    handleListPointerDown,
    resetColumnWidths,
  } = useColumnResize();

  const {
    displayedClips,
    queuedIndexMap,
    searchTotalCount,
    isSearching,
    searchFailed,
    retrySearch,
    loadMoreSearchResults,
  } = useClipViews({
    allClips,
    trashedClips,
    bins,
    currentTab,
    selectedBinId,
    searchQuery,
    sequentialStatus: seqStatus,
    features: enabledFeatures,
  });
  const currentCollection = useMemo(
    () => getClipCollection(currentTab, selectedBinId === null ? undefined : bins.find((bin) => bin.id === selectedBinId)),
    [bins, currentTab, locale, selectedBinId],
  );
  const {
    clipListRef,
    handleClipListScroll,
    isLoadingCurrentCollection,
    pinnedShelfClips,
    requestRepositionedClipReveal,
    stackedPinnedClipIds,
  } = useClipListViewport({
    membership: currentCollection?.membership,
    currentTab,
    selectedBinId,
    displayedClips,
    allClips,
    trashedClips,
    selectedClip,
    pinningEnabled: enabledFeatures.pinning,
    totalClipCount,
    totalTrashCount,
    searchTotalCount,
    isLoadingMoreClips,
    isLoadingMoreTrash,
    isSearching,
    loadMoreClips,
    loadMoreTrashedClips,
    loadMoreSearchResults,
  });
  const {
    binClipReorder,
    isBinCollection,
    isQueueCollection,
    queueReorder,
    reorderIdsForClip,
  } = useClipReordering({
    collection: currentCollection,
    selectedBinId,
    displayedClips,
    sequentialStatus: seqStatus,
    loadedClipCount: allClips.length,
    totalClipCount,
    clipListRef,
    fetchBins,
    fetchSequentialStatus,
  });
  const binsById = useMemo(() => new Map(bins.map((bin) => [bin.id, bin])), [bins]);
  const currentContextMenuClip = useLiveClipSnapshot(contextMenu?.clip ?? null, allClips, trashedClips);
  const selectedClipViewPolicy = getClipViewPolicy(currentTab, selectedClip);
  const hasRestrictedSelection = Array.from(selectedClipIds).some((id) => {
    const selected = displayedClips.find((clip) => clip.id === id);
    return selected ? !getClipViewPolicy(currentTab, selected).canOrganize : false;
  });

  const {
    togglePin: handleTogglePin,
    toggleProtected: handleToggleProtected,
    setPinned: handleSetPinned,
    setProtected: handleSetProtected,
    deleteSelectedClips: handleBatchTrash,
    deleteClip: handleDeleteClip,
    copyClip: handleCopyClip,
    assignClipToBin,
    removeClipFromBin,
    runTransformForClip: handleRunTransformForClip,
    addToSequentialStack: handleAddToSequentialStack,
    toggleSequentialStack: handleToggleSequentialStack,
    updateClipNoteLocally: handleUpdateClipNoteLocally,
    deleteNoteFromClip: handleDeleteNoteFromClip,
    transformingClipIds,
    transformErrorsByClipId,
  } = useClipActions({
    allClips,
    setAllClips,
    setTrashedClips,
    bins,
    setBins,
    setSelectedClip,
    selectedClipIds,
    setSelectedClipIds,
    setTotalClipCount,
    settings: appSettings,
    fetchBins,
    fetchClips,
    fetchTrashedClips,
    fetchSequentialStatus,
    queuedIndexMap,
    onCollectionChanged: fetchClipCollectionSummary,
    keepTrashedClipsVisible: currentTab === 'search',
    onClipsRepositioned: requestRepositionedClipReveal,
  });

  const {
    clearClipSelection,
    handleClipSelect,
    selectClipForContextMenu,
    selectPinnedShelfClip,
  } = useClipSelectionController({
    displayedClips,
    initialDataLoaded,
    currentTab,
    selectedBinId,
    restoredUiState,
    selectedClip,
    setSelectedClip,
    selectedClipIds,
    setSelectedClipIds,
    setIsSidebarCollapsed,
    copyClip: handleCopyClip,
    deleteClip: handleDeleteClip,
    purgeClipPermanently: handlePurgeClipPermanently,
  });

  const handleSetSelectedPinned = useCallback((pinned: boolean) => {
    const anchorId = selectedClipIds.values().next().value;
    if (typeof anchorId === 'number') handleSetPinned(anchorId, pinned);
  }, [handleSetPinned, selectedClipIds]);

  const handleRequestDeleteBin = useCallback((bin: Bin) => setBinToDelete(bin), []);
  const handleRequestClearHistory = useCallback(() => setClearHistoryMode('purge'), []);

  const {
    draggedClipId,
    setDraggedClipId,
    pointerDropTargetBinId,
    setPointerDropTargetBinId,
    pointerDropTargetAction,
    setPointerDropTargetAction,
    clipDragPreview,
    setClipDragPreview,
    disabledDropBinId,
    disabledDropActions,
    pinnedReorderOffsets,
    isPinnedReorderSettling,
    updatePointerDropTarget,
    beginPinnedReorderPreview,
    updatePinnedReorderPreview,
    cancelPinnedReorderPreview,
    finishClipPointerDrag: handleClipPointerDragEnd,
    hoveredClipId,
    setHoveredClipId,
    assignSidebarDropToBin: handleSidebarClipDropOnBin,
  } = useClipDragController({
    isQueueCollection,
    allClips,
    setAllClips,
    bins,
    selectedClipIds,
    fetchClips,
    binsEnabled: enabledFeatures.bins,
    queueEnabled: enabledFeatures.queue,
    pinningEnabled: enabledFeatures.pinning,
    protectionEnabled: enabledFeatures.protection,
    assignClipToBin,
    addToQueue: handleAddToSequentialStack,
    setPinned: handleSetPinned,
    setProtected: handleSetProtected,
    deleteClip: handleDeleteClip,
  });

  const handleAssignBin = useCallback(
    (clipId: number, binId: number | null) => {
      if (!enabledFeatures.bins) return;
      assignClipToBin(clipId, binId);
    },
    [assignClipToBin, enabledFeatures.bins],
  );

  const handlePreviewClipUpdate = useCallback((updatedClip?: ClipItem) => {
    if (updatedClip) {
      setAllClips((previous) => previous.map((clip) => (
        clip.id === updatedClip.id ? updatedClip : clip
      )));
      setSelectedClip((previous) => previous?.id === updatedClip.id ? updatedClip : previous);
    }
    void Promise.all([fetchClips(), fetchBins()]);
  }, [fetchBins, fetchClips]);

  const handleClearHistory = async () => {
    if (!clearHistoryMode) return;
    try {
      if (clearHistoryMode === 'purge') await invoke('purge_unpinned_clips');
      else await invoke('trash_unpinned_clips');
      setClearHistoryMode(null);
      await Promise.all([fetchClips(), fetchTrashedClips(), fetchBins(), fetchClipCollectionSummary()]);
    } catch (e) {
      console.error(e);
    }
  };

  const handleRestoreAllTrashedClips = async () => {
    const summary = await clipsApi.restoreAll();
    await Promise.all([fetchClips(), fetchTrashedClips(), fetchBins(), fetchClipCollectionSummary()]);
    return summary.changedCount;
  };

  useAppMenuActions({
    enabled: !isHudView,
    enabledFeatures,
    selectedClip,
    selectedClipIds,
    selectedClipViewPolicy,
    textSize: appSettings.textSize,
    setIsBinModalOpen,
    setEditingBin,
    setIsSidebarCollapsed,
    updateSettings: handleUpdateSettings,
    toggleClipboardPause: handleToggleClipboardPause,
    toggleCopyQueue: handleToggleCopyQueue,
    copyClip: handleCopyClip,
    promptAddNote: handlePromptAddNote,
    togglePin: handleTogglePin,
    toggleProtected: handleToggleProtected,
    batchTrash: handleBatchTrash,
    purgeClipPermanently: handlePurgeClipPermanently,
    deleteClip: handleDeleteClip,
    resetColumnWidths,
    refreshData: () => Promise.all([
      fetchClips(),
      fetchTrashedClips(),
      fetchBins(),
      fetchManualTransforms(),
      fetchSequentialStatus(),
      fetchClipCollectionSummary(),
    ]),
  });

  const draggedPreviewClip = clipDragPreview
    ? displayedClips.find((clip) => clip.id === clipDragPreview.clipId)
      ?? allClips.find((clip) => clip.id === clipDragPreview.clipId)
    : undefined;

  if (isHudView) {
    return <FeatureProvider features={enabledFeatures}><QuickHudWindow /></FeatureProvider>;
  }

  return (
    <FeatureProvider features={enabledFeatures}>
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
        pinnedCount={clipCollectionSummary.pinnedCount}
        protectedCount={clipCollectionSummary.protectedCount}
        notesCount={clipCollectionSummary.notedCount}
        trashedCount={totalTrashCount}
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

      {/* Main Content Area */}
      <Suspense fallback={null}>{currentTab === 'transformations' ? (
        <TransformationsView
          manualTransforms={manualTransforms}
          onRefreshManualTransforms={fetchManualTransforms}
          activeWorkspace={activeTransformWorkspace}
          onActiveWorkspaceChange={setActiveTransformWorkspace}
        />
      ) : currentTab === 'activity' ? (
        <ActivityLogView />
      ) : currentTab === 'analytics' ? (
        <AnalyticsView />
      ) : currentTab === 'help' ? (
        <HelpView
          activeTopic={activeHelpTopic}
          onActiveTopicChange={setActiveHelpTopic}
        />
      ) : currentTab === 'settings' ? (
        <SettingsModal
          settings={appSettings}
          onUpdateSettings={handleUpdateSettings}
          blacklistApps={blacklistApps}
          onAddBlacklistApp={handleAddBlacklistApp}
          onRemoveBlacklistApp={handleRemoveBlacklistApp}
          onToggleBlacklistRule={handleToggleBlacklistRule}
          onRefreshManualTransforms={fetchManualTransforms}
          bins={bins}
          onRefreshBins={fetchBins}
          onRefreshClips={fetchClips}
          onRefreshTrashedClips={fetchTrashedClips}
          onClearHistory={(permanent) => setClearHistoryMode(permanent ? 'purge' : 'trash')}
          onRestoreAllTrashedClips={handleRestoreAllTrashedClips}
          trashedClipCount={trashedClips.length}
          onResetColumnWidths={resetColumnWidths}
          activeTab={activeSettingsTab}
          onActiveTabChange={setActiveSettingsTab}
          onOpenAnalytics={() => handleSidebarNavigate('analytics')}
        />
      ) : (
        <div className="flex-1 h-screen flex overflow-hidden">
          {/* Middle Clips List Panel */}
          <div
            style={{ width: `${clipsListWidth}px` }}
            className="shrink-0 col-list h-screen flex flex-col overflow-hidden"
          >
            {/* Finder Header Title Bar */}
            <div
              onMouseDown={startWindowDrag}
              onDoubleClick={handleWindowDragDoubleClick}
              className="h-[60px] border-b px-3 flex items-center justify-between col-list-header cursor-default titlebar-drag-handle shrink-0"
            >
              <div className="flex items-center space-x-2 titlebar-drag-handle min-w-0 flex-1 me-2">
                {currentCollection?.icon === 'search' ? (
                  <Search className="theme-text-main w-4 h-4 titlebar-drag-handle shrink-0" />
                ) : (
                  <Clipboard className="theme-text-main w-4 h-4 titlebar-drag-handle shrink-0" />
                )}
                <OverflowText as="h2" text={currentCollection?.title ?? translate('collection.history')} className="theme-title text-xs font-bold uppercase tracking-wider titlebar-drag-handle truncate" />
                {currentTab === 'search' && (
                  <span
                    className="theme-badge min-w-5 rounded-md border px-1.5 py-0.5 text-center font-mono text-[10px] font-semibold"
                    aria-label={translate('app.searchResultCount', { count: searchTotalCount })}
                    title={translate('app.resultCount', { count: searchTotalCount })}
                  >
                    {searchTotalCount}
                  </span>
                )}
              </div>

              {/* Global Controls & Status Badges */}
              <div className="flex items-center space-x-1.5 shrink-0">
                {ignoredAppStatus && (
                  <span className="theme-status-danger text-[10px] px-2 py-0.5 rounded border font-mono flex items-center animate-in fade-in">
                    {translate('app.ignoredApp', { name: ignoredAppStatus.app_name })}
                  </span>
                )}

                {currentCollection?.membership === 'trash' && (
                  <button
                    onClick={handleEmptyTrash}
                    disabled={trashedClips.length === 0}
                    className="theme-status-danger px-2 py-1 rounded-lg border text-xs font-semibold disabled:opacity-40 transition-colors cursor-pointer flex items-center space-x-1"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                    <span>{translate('app.emptyTrash')}</span>
                  </button>
                )}

                {/* Pause History Toggle Button */}
                <button
                  onClick={handleToggleClipboardPause}
                  className={`list-toolbar-button w-7 h-7 flex items-center justify-center rounded-lg border transition-[background-color,border-color,color] cursor-pointer ${
                    isClipboardPaused
                      ? 'is-warning shadow-sm'
                      : ''
                  }`}
                  title={isClipboardPaused ? translate('app.resumeHistory') : translate('app.pauseHistory')}
                >
                  <Pause
                    className={`w-4 h-4 ${isClipboardPaused ? 'fill-current animate-pulse' : ''}`}
                    strokeWidth={2.5}
                  />
                </button>

                {/* Copy Queue Record/Stop Toggle Button */}
                {enabledFeatures.queue && <button
                  onClick={handleToggleCopyQueue}
                  className={`list-toolbar-button w-7 h-7 flex items-center justify-center rounded-lg border transition-[background-color,border-color,color] cursor-pointer ${
                    seqStatus?.is_active
                      ? 'is-queue-active shadow-sm'
                      : ''
                  }`}
                  title={seqStatus?.is_active
                    ? translate('app.stopQueueCount', { count: seqStatus.queue.length })
                    : translate('app.startQueue')}
                >
                  {seqStatus?.is_active ? (
                    <Square className="w-3.5 h-3.5 fill-current animate-pulse" strokeWidth={2.5} />
                  ) : (
                    <Disc className="w-4 h-4 transition-colors" strokeWidth={2.5} />
                  )}
                </button>}
              </div>
            </div>

            {/* Sequential Paste Top Header Banner if active */}
            {isQueueCollection && (
              <div className={`queue-controls-region p-3 border-b ${seqStatus?.is_active ? 'is-active' : ''}`}>
                <SequentialQueueBar
                  status={seqStatus}
                  onRefresh={fetchSequentialStatus}
                />
              </div>
            )}

            {/* Clips List Content */}
            <div className="relative flex-1 min-h-0">
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
                className="h-full overflow-y-auto ps-3 pe-3 py-3 space-y-2.5 custom-scrollbar"
                onScroll={(event) => handleClipListScroll(event.currentTarget)}
                onClick={(event) => {
                  if (event.target === event.currentTarget) clearClipSelection();
                }}
              >
                {displayedClips.length === 0 && !isLoadingCurrentCollection ? (
                  searchFailed && currentTab === 'search' ? (
                    <div className="flex h-full items-center justify-center p-6">
                      <SearchErrorNotice onRetry={retrySearch} />
                    </div>
                  ) : (
                  <EmptyClipList
                    currentTab={currentTab}
                    searchQuery={searchQuery}
                    selectedBin={selectedBinId === null ? undefined : binsById.get(selectedBinId)}
                  />
                  )
                ) : (
                  <>
                  {searchFailed && currentTab === 'search' && <SearchErrorNotice onRetry={retrySearch} />}
                  {displayedClips.map((clip, index) => {
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

                  return (
                    <ClipCard
                      key={clip.id}
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
                      searchQuery={currentTab === 'search' ? searchQuery : undefined}
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
                      onDelete={(e) => handleDeleteClip(clip.id, e?.altKey)}
                      onRestore={() => handleRestoreClip(clip.id)}
                      onPurgePermanently={() => handlePurgeClipPermanently(clip.id)}
                      onRemoveFromQueue={() => {
                        const idx = queueIndex !== undefined ? queueIndex - 1 : -1;
                        if (idx !== -1) {
                          invoke('remove_sequential_item_by_index', { index: idx }).then(fetchSequentialStatus);
                        }
                      }}
                      onPasteQueueItem={() => {
                        const idx = queueIndex !== undefined ? queueIndex - 1 : -1;
                        if (idx !== -1) invoke('paste_sequential_item_by_index', { index: idx }).then(fetchSequentialStatus);
                      }}
                      onCopy={() => handleCopyClip(clip)}
                      onContextMenu={(e) => {
                        e.preventDefault();
                        selectClipForContextMenu(clip);
                        setContextMenu({
                          x: e.clientX,
                          y: e.clientY,
                          clip,
                        });
                      }}
                    />
                  );
                  })}
                  {isLoadingCurrentCollection && (
                    <div className="theme-text-muted py-3 text-center text-xs" role="status">
                    {translate('app.loadingOlderClips')}
                    </div>
                  )}
                  </>
                )}
              </div>
            </div>

            {/* Floating Glass Batch Action Bar */}
            {selectedClipIds.size > 1 && selectedClipViewPolicy.showOrganizeBatchActions && !hasRestrictedSelection && (
              <ClipBatchActionBar
                selectedCount={selectedClipIds.size}
                pinningEnabled={enabledFeatures.pinning}
                trashEnabled={appSettings.enableTrash}
                onSetPinned={handleSetSelectedPinned}
                onTrash={handleBatchTrash}
                onClearSelection={clearClipSelection}
              />
            )}
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
            onDeleteClip={selectedClipViewPolicy.state === 'trash' ? handlePurgeClipPermanently : handleDeleteClip}
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
      )}</Suspense>

      {/* Right Click Context Menu */}
      {contextMenu && currentContextMenuClip && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          clip={currentContextMenuClip}
          viewPolicy={getClipViewPolicy(currentTab, currentContextMenuClip)}
          selectedCount={selectedClipIds.has(currentContextMenuClip.id) ? selectedClipIds.size : 1}
          bins={bins}
          onClose={() => setContextMenu(null)}
          onCopy={() => handleCopyClip(currentContextMenuClip)}
          onAssignBin={(binId) => assignClipToBin(
            currentContextMenuClip.id,
            binId,
            { includeSelection: true },
          )}
          onRemoveBin={(binId) => removeClipFromBin(currentContextMenuClip.id, binId)}
          onRunTransform={(transform) => handleRunTransformForClip(currentContextMenuClip, transform)}
          onOpenTransformations={() => navigateToTab('transformations')}
          onAddNote={() => handlePromptAddNote(currentContextMenuClip)}
          onDeleteNote={() => handleDeleteNoteFromClip(currentContextMenuClip.id)}
          isQueued={Boolean(currentContextMenuClip.text_content && queuedIndexMap.has(currentContextMenuClip.text_content))}
          onToggleQueue={() => void handleToggleSequentialStack(currentContextMenuClip)}
          onTogglePin={() => handleTogglePin(currentContextMenuClip.id)}
          onToggleProtected={() => handleToggleProtected(currentContextMenuClip.id)}
          onDelete={(e) => handleDeleteClip(currentContextMenuClip.id, e?.altKey)}
          onRestore={() => handleRestoreClip(currentContextMenuClip.id)}
          onPurge={() => handlePurgeClipPermanently(currentContextMenuClip.id)}
          trashEnabled={appSettings.enableTrash}
        />
      )}

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
        clearHistoryMode={clearHistoryMode}
        setClearHistoryMode={setClearHistoryMode}
        confirmClearHistory={handleClearHistory}
        fetchBins={fetchBins}
        fetchClips={fetchClips}
        fetchTrashedClips={fetchTrashedClips}
        fetchClipCollectionSummary={fetchClipCollectionSummary}
      />
    </div>
    </FeatureProvider>
  );
}
