import { useCallback, useMemo, useState } from 'react';
import type { Bin, ClipItem } from '../types';
import { useLocalization } from '../localization/LocalizationProvider';
import { useAppData } from './useAppData';
import { useAppSettings } from './useAppSettings';
import { useClipActions } from './useClipActions';
import { useClipViews } from './useClipViews';
import { useColumnResize } from './useColumnResize';
import { useAppLibraryActions } from './useAppLibraryActions';
import { findDraggedPreviewClip, selectionHasRestrictedClip } from './appControllerModel';
import {
  useAppMenuActions,
  useAppNavigation,
  useAppOverlays,
  useAppShell,
  useClipDragController,
  useClipHistoryFocus,
  useClipListViewport,
  useClipReordering,
  useClipSelectionController,
  useCopyQueueController,
  useSettledSearchQuery,
  useSoundSettings,
} from './appControllers';
import { enabledFeatureRecord } from '../utils/features';
import { getClipCollection } from '../utils/clipCollections';
import { getClipViewPolicy } from '../utils/clipViewPolicy';
import { readAppUiState } from '../utils/appUiState';

export function useAppController() {
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
    resetBlacklistApps: handleResetBlacklistApps, prepareForFactoryReset,
  } = useAppSettings();
  const enabledFeatures = useMemo(() => enabledFeatureRecord(appSettings), [appSettings]);

  useSoundSettings(appSettings.enableSounds);

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
    restoreClip: restoreClipInData,
    purgeClipPermanently: handlePurgeClipPermanently,
    emptyTrash: handleEmptyTrash,
  } = useAppData();

  useAppShell({
    catalogReady,
    direction,
    settingsHydrated,
    initialDataLoaded,
  });

  const [selectedClip, setSelectedClip] = useState<ClipItem | null>(null);
  const [selectedClipIds, setSelectedClipIds] = useState<Set<number>>(new Set());
  const {
    currentTab,
    setCurrentTab,
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
    selectedClipId: selectedClip?.id ?? null,
  });
  const settledSearchQuery = useSettledSearchQuery(searchQuery, currentTab === 'search');
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
    namePromptClip,
    setNamePromptClip,
    namePromptText,
    setNamePromptText,
    clearHistoryMode,
    setClearHistoryMode,
    openNewBinModal: handleOpenNewBinModal,
    editBin: handleEditBin,
    closeBinModal,
    openBinContextMenu: handleBinContextMenu,
    promptAddNote: handlePromptAddNote,
    promptNameClip: handlePromptNameClip,
  } = useAppOverlays({
    binsEnabled: enabledFeatures.bins,
    notesEnabled: enabledFeatures.notes,
    namingEnabled: enabledFeatures.naming,
  });

  const handleToggleCopyQueue = useCopyQueueController({
    enabled: enabledFeatures.queue,
    active: Boolean(seqStatus?.is_active),
    navigateToTab,
    setSelectedBinId,
    refreshStatus: fetchSequentialStatus,
  });

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
    searchDisplayQuery,
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
    searchQuery: settledSearchQuery,
    sequentialStatus: seqStatus,
    features: enabledFeatures,
  });
  const currentCollection = useMemo(
    () => getClipCollection(currentTab, selectedBinId === null ? undefined : bins.find((bin) => bin.id === selectedBinId)),
    [bins, currentTab, locale, selectedBinId],
  );
  const clipHistoryFocus = useClipHistoryFocus({
    currentTab,
    currentAssociation: currentCollection?.association,
    selectedClip,
    setCurrentTab,
    setSelectedBinId,
    restoreClip: restoreClipInData,
  });
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
    focusRequest: clipHistoryFocus.focusRequest,
  });
  const {
    binClipReorder,
    isBinCollection,
    isQueueCollection,
    queueReorder,
    reorderIdsForClip,
    displayedClipsForRender,
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
  const selectedClipViewPolicy = getClipViewPolicy(currentTab, selectedClip);
  const hasRestrictedSelection = selectionHasRestrictedClip(
    selectedClipIds, displayedClips, (clip) => !getClipViewPolicy(currentTab, clip).canOrganize,
  );

  const {
    togglePin: handleTogglePin,
    toggleProtected: handleToggleProtected,
    toggleConcealed: handleToggleConcealed,
    setPinned: handleSetPinned,
    setProtected: handleSetProtected,
    setConcealed: handleSetConcealed,
    deleteSelectedClips: handleBatchTrash,
    deleteClip: handleDeleteClip,
    copyClip: handleCopyClip,
    assignClipToBin,
    removeClipFromBin,
    runTransformForClip: handleRunTransformForClip,
    addToSequentialStack: handleAddToSequentialStack,
    toggleSequentialStack: handleToggleSequentialStack,
    updateClipNoteLocally: handleUpdateClipNoteLocally,
    updateClipNameLocally: handleUpdateClipNameLocally,
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
    onClipPropertyRemoved: clipHistoryFocus.handlePropertyRemoved,
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
    focusRequest: clipHistoryFocus.focusRequest,
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
    concealmentEnabled: enabledFeatures.concealment,
    assignClipToBin,
    addToQueue: handleAddToSequentialStack,
    setPinned: handleSetPinned,
    setProtected: handleSetProtected,
    setConcealed: handleSetConcealed,
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

  const {
    handleClearClipName,
    handleClearHistory,
    handleRestoreAllTrashedClips,
  } = useAppLibraryActions({
    clearHistoryMode,
    setClearHistoryMode,
    updateClipNameLocally: handleUpdateClipNameLocally,
    handlePropertyRemoved: clipHistoryFocus.handlePropertyRemoved,
    fetchClips,
    fetchTrashedClips,
    fetchBins,
    fetchClipCollectionSummary,
  });

  useAppMenuActions({
    enabled: true,
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
    promptNameClip: handlePromptNameClip,
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

  const draggedPreviewClip = findDraggedPreviewClip(clipDragPreview, displayedClips, allClips);
  return {
    shell: { direction, enabledFeatures, appSettings, settingsHydrated, initialDataLoaded },
    settings: { blacklistApps, handleUpdateSettings, handleAddBlacklistApp, handleRemoveBlacklistApp, handleToggleBlacklistRule, handleResetBlacklistApps, prepareForFactoryReset },
    data: {
      allClips, trashedClips, bins, manualTransforms, seqStatus, totalClipCount, totalTrashCount,
      clipCollectionSummary, isClipboardPaused, ignoredAppStatus, fetchClips, fetchTrashedClips,
      fetchClipCollectionSummary, fetchBins, fetchManualTransforms, fetchSequentialStatus,
      handleToggleClipboardPause, handlePurgeClipPermanently, handleEmptyTrash,
    },
    navigation: {
      currentTab, setCurrentTab, activeSettingsTab, setActiveSettingsTab, activeHelpTopic, setActiveHelpTopic,
      activeTransformWorkspace, setActiveTransformWorkspace, selectedBinId, setSelectedBinId,
      searchQuery, setSearchQuery, isSidebarCollapsed, setIsSidebarCollapsed, sidebarSections,
      handleSidebarSectionStateChange, navigateToTab, enterSearchView, exitEmptySearch,
    },
    overlays: {
      contextMenu, setContextMenu, binContextMenu, setBinContextMenu, isBinModalOpen,
      editingBin, setEditingBin, binToDelete, setBinToDelete, notePromptClip, setNotePromptClip,
      notePromptText, setNotePromptText, namePromptClip, setNamePromptClip, namePromptText,
      setNamePromptText, clearHistoryMode, setClearHistoryMode, handleOpenNewBinModal,
      handleEditBin, closeBinModal, handleBinContextMenu, handlePromptAddNote, handlePromptNameClip,
    },
    layout: {
      sidebarWidth, clipsListWidth, isResizingSidebar, isResizingList,
      handleSidebarPointerDown, handleListPointerDown, resetColumnWidths,
    },
    clipView: {
      displayedClips, queuedIndexMap, searchTotalCount, searchDisplayQuery, searchFailed,
      retrySearch, currentCollection, clipListRef, handleClipListScroll, isLoadingCurrentCollection,
      pinnedShelfClips, stackedPinnedClipIds, binClipReorder, isBinCollection, isQueueCollection,
      queueReorder, reorderIdsForClip, displayedClipsForRender, binsById, selectedClipViewPolicy, hasRestrictedSelection,
      clipHistoryFocus,
    },
    selection: {
      selectedClip, selectedClipIds, clearClipSelection, handleClipSelect,
      selectClipForContextMenu, selectPinnedShelfClip, handleSetSelectedPinned,
    },
    actions: {
      handleTogglePin, handleToggleProtected, handleToggleConcealed, handleSetProtected, handleSetConcealed, handleBatchTrash,
      handleDeleteClip, handleCopyClip, assignClipToBin, removeClipFromBin,
      handleRunTransformForClip, handleToggleSequentialStack, handleUpdateClipNoteLocally,
      handleUpdateClipNameLocally, handleDeleteNoteFromClip, transformingClipIds,
      transformErrorsByClipId, handleClearClipName,
    },
    drag: {
      draggedClipId, setDraggedClipId, pointerDropTargetBinId, setPointerDropTargetBinId,
      pointerDropTargetAction, setPointerDropTargetAction, clipDragPreview, setClipDragPreview,
      disabledDropBinId, disabledDropActions, pinnedReorderOffsets, isPinnedReorderSettling,
      updatePointerDropTarget, beginPinnedReorderPreview, updatePinnedReorderPreview,
      cancelPinnedReorderPreview, handleClipPointerDragEnd, hoveredClipId, setHoveredClipId,
      handleSidebarClipDropOnBin, draggedPreviewClip,
    },
    handlers: {
      handleToggleCopyQueue, handleSidebarNavigate, handleSidebarBinSelect, handleRequestDeleteBin,
      handleRequestClearHistory, handleAssignBin, handlePreviewClipUpdate, handleClearHistory,
      handleRestoreAllTrashedClips,
    },
  };
}
