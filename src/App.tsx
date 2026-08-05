import { useEffect, useLayoutEffect, useState, useCallback, useMemo, useRef } from 'react';
import { safeInvoke as invoke } from './utils/tauri';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ClipItem, Bin, getClipFileSummary } from './types';
import { Sidebar } from './components/Sidebar';
import { ClipCard } from './components/ClipCard';
import { EmptyClipList } from './components/EmptyClipList';
import { PinnedClipShelf } from './components/PinnedClipShelf';
import { ClipPreview } from './components/ClipPreview';
import { SequentialQueueBar } from './components/SequentialQueueBar';
import { TransformationsView } from './components/TransformationsView';
import { SettingsModal } from './components/SettingsModal';
import { BinModal } from './components/BinModal';
import { ContextMenu } from './components/ContextMenu';
import { QuickHudWindow } from './components/QuickHudWindow';
import { ActivityLogView } from './components/ActivityLogView';
import { AnalyticsView } from './components/AnalyticsView';
import { HelpView } from './components/HelpView';
import { BinContextMenu } from './components/BinContextMenu';
import { DeleteBinDialog } from './components/DeleteBinDialog';
import { ClipNoteDialog } from './components/ClipNoteDialog';
import { ClearHistoryDialog, type ClearHistoryMode } from './components/ClearHistoryDialog';
import { startWindowDrag } from './utils/windowDrag';
import { useColumnResize } from './hooks/useColumnResize';
import { useAppSettings } from './hooks/useAppSettings';
import { useClipViews } from './hooks/useClipViews';
import { useClipBinDrag, type ClipDropAction } from './hooks/useClipBinDrag';
import { getClipViewPolicy } from './utils/clipViewPolicy';
import { sortClipsForTimeline } from './utils/clipOrder';
import { useAppData } from './hooks/useAppData';
import { useClipActions } from './hooks/useClipActions';
import { Clipboard, Trash2, Pause, Disc, Square, Pin, Search, X } from 'lucide-react';
import './App.css';

export default function App() {
  const [isHudView, setIsHudView] = useState<boolean>(false);

  useEffect(() => {
    const hideTimers = new Map<HTMLElement, number>();
    const handleToolsScroll = (event: Event) => {
      const target = event.target;
      if (!(target instanceof HTMLElement)
        || (!target.classList.contains('tools-scroll-region') && !target.classList.contains('overlay-scroll-region'))) return;

      target.classList.add('is-scrolling');
      const previousTimer = hideTimers.get(target);
      if (previousTimer) window.clearTimeout(previousTimer);
      hideTimers.set(target, window.setTimeout(() => {
        target.classList.remove('is-scrolling');
        hideTimers.delete(target);
      }, 700));
    };

    document.addEventListener('scroll', handleToolsScroll, true);
    return () => {
      document.removeEventListener('scroll', handleToolsScroll, true);
      hideTimers.forEach((timer) => window.clearTimeout(timer));
    };
  }, []);

  useEffect(() => {
    const enableHudMode = () => {
      setIsHudView(true);
      document.documentElement.classList.add('hud-mode');
      document.body.classList.add('hud-mode');
      const root = document.getElementById('root');
      if (root) root.classList.add('hud-mode');
    };

    try {
      const win = getCurrentWindow();
      if (win.label === 'hud' || window.location.search.includes('view=hud')) {
        enableHudMode();
      }
    } catch {
      if (window.location.search.includes('view=hud')) {
        enableHudMode();
      }
    }
  }, []);

  const {
    appSettings,
    blacklistApps,
    settingsHydrated,
    updateSettings: handleUpdateSettings,
    addBlacklistApp: handleAddBlacklistApp,
    removeBlacklistApp: handleRemoveBlacklistApp,
    toggleBlacklistRule: handleToggleBlacklistRule,
  } = useAppSettings();

  const {
    allClips,
    setAllClips,
    trashedClips,
    setTrashedClips,
    bins,
    setBins,
    pipelines,
    sequentialStatus: seqStatus,
    totalClipCount,
    setTotalClipCount,
    isClipboardPaused,
    ignoredAppStatus,
    initialDataLoaded,
    fetchClips,
    fetchTrashedClips,
    fetchBins,
    fetchPipelines,
    fetchSequentialStatus,
    toggleClipboardPause: handleToggleClipboardPause,
    restoreClip: handleRestoreClip,
    purgeClipPermanently: handlePurgeClipPermanently,
    emptyTrash: handleEmptyTrash,
  } = useAppData(appSettings.enableSounds);

  useEffect(() => {
    const splash = document.getElementById('startup-splash');
    if (!splash) return;
    if (isHudView) {
      splash.remove();
      return;
    }
    if (!settingsHydrated || !initialDataLoaded) return;

    let removeTimer: ReturnType<typeof setTimeout> | undefined;
    let secondFrame = 0;
    const firstFrame = requestAnimationFrame(() => {
      secondFrame = requestAnimationFrame(() => {
        splash.classList.add('is-ready');
        removeTimer = setTimeout(() => splash.remove(), 160);
      });
    });
    return () => {
      cancelAnimationFrame(firstFrame);
      if (secondFrame) cancelAnimationFrame(secondFrame);
      if (removeTimer) clearTimeout(removeTimer);
    };
  }, [initialDataLoaded, isHudView, settingsHydrated]);

  const [selectedClip, setSelectedClip] = useState<ClipItem | null>(null);
  const [selectedClipIds, setSelectedClipIds] = useState<Set<number>>(new Set());
  const [hoveredClipId, setHoveredClipId] = useState<number | null>(null);
  const [, setSelectedIndex] = useState<number>(-1);
  const [currentTab, setCurrentTab] = useState<string>('all');
  const [selectedBinId, setSelectedBinId] = useState<number | null>(null);
  const lastClipViewRef = useRef<{ tab: string; binId: number | null }>({ tab: 'all', binId: null });
  const selectionViewKey = currentTab === 'bin' ? `bin:${selectedBinId ?? 'none'}` : `section:${currentTab}`;
  const selectedClipByViewRef = useRef<Map<string, number | null>>(new Map());
  const activeSelectionViewRef = useRef<string | null>(null);
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [isBinModalOpen, setIsBinModalOpen] = useState<boolean>(false);
  const [editingBin, setEditingBin] = useState<Bin | null>(null);
  const [clearHistoryMode, setClearHistoryMode] = useState<ClearHistoryMode | null>(null);
  const isClearConfirmOpen = clearHistoryMode !== null;
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState<boolean>(false);

  const clearClipSelection = useCallback(() => {
    setSelectedClip(null);
    setSelectedClipIds(new Set());
    setSelectedIndex(-1);
  }, []);

  const navigateToTab = useCallback((tab: string) => {
    setCurrentTab(tab);
  }, []);

  const enterSearchView = useCallback(() => {
    if (currentTab !== 'search') setCurrentTab('search');
  }, [currentTab]);

  useEffect(() => {
    if (['all', 'sequential', 'pinned', 'protected', 'notes', 'trash', 'bin'].includes(currentTab)) {
      lastClipViewRef.current = { tab: currentTab, binId: currentTab === 'bin' ? selectedBinId : null };
    }
  }, [currentTab, selectedBinId]);

  const exitEmptySearch = useCallback(() => {
    const previous = lastClipViewRef.current;
    if (previous.tab === 'bin' && previous.binId !== null && bins.some((bin) => bin.id === previous.binId)) {
      setSelectedBinId(previous.binId);
      setCurrentTab('bin');
      return;
    }
    setSelectedBinId(null);
    setCurrentTab(previous.tab === 'bin' ? 'all' : previous.tab);
  }, [bins]);

  const handleToggleCopyQueue = async () => {
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

  // Context Menu State
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    clip: ClipItem;
  } | null>(null);

  // Bin Context Menu State
  const [binContextMenu, setBinContextMenu] = useState<{
    x: number;
    y: number;
    bin: Bin;
  } | null>(null);

  // Custom Bin Deletion Confirmation Modal State
  const [binToDelete, setBinToDelete] = useState<Bin | null>(null);

  // Custom Note Editing Modal State
  const [notePromptClip, setNotePromptClip] = useState<ClipItem | null>(null);
  const [notePromptText, setNotePromptText] = useState<string>('');

  const {
    sidebarWidth,
    clipsListWidth,
    isResizingSidebar,
    isResizingList,
    handleSidebarPointerDown,
    handleListPointerDown,
    resetColumnWidths,
  } = useColumnResize();

  useEffect(() => {
    if (!binContextMenu) return;
    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && target.closest('.bin-context-menu')) return;
      setBinContextMenu(null);
    };
    window.addEventListener('mousedown', handleClickOutside);
    return () => window.removeEventListener('mousedown', handleClickOutside);
  }, [binContextMenu]);

  // Global Escape key listener to cancel any active modal or context menu
  useEffect(() => {
    const handleGlobalKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (notePromptClip) {
          e.preventDefault();
          e.stopPropagation();
          setNotePromptClip(null);
        } else if (binToDelete) {
          e.preventDefault();
          e.stopPropagation();
          setBinToDelete(null);
        } else if (isClearConfirmOpen) {
          e.preventDefault();
          e.stopPropagation();
          setClearHistoryMode(null);
        } else if (binContextMenu) {
          e.preventDefault();
          e.stopPropagation();
          setBinContextMenu(null);
        } else if (contextMenu) {
          e.preventDefault();
          e.stopPropagation();
          setContextMenu(null);
        } else if (isBinModalOpen) {
          e.preventDefault();
          e.stopPropagation();
          setIsBinModalOpen(false);
          setEditingBin(null);
        }
      }
    };
    window.addEventListener('keydown', handleGlobalKeyDown, true);
    return () => window.removeEventListener('keydown', handleGlobalKeyDown, true);
  }, [notePromptClip, binToDelete, isClearConfirmOpen, binContextMenu, contextMenu, isBinModalOpen]);

  // Disable WebKit default right-click context menu (Reload/Inspect) app-wide
  useEffect(() => {
    const handleGlobalContextMenu = (e: MouseEvent) => {
      e.preventDefault();
    };
    window.addEventListener('contextmenu', handleGlobalContextMenu);
    return () => window.removeEventListener('contextmenu', handleGlobalContextMenu);
  }, []);

  const {
    displayedClips,
    pinnedCount,
    protectedCount,
    notesCount,
    queuedIndexMap,
  } = useClipViews({
    allClips,
    trashedClips,
    bins,
    currentTab,
    selectedBinId,
    searchQuery,
    sequentialStatus: seqStatus,
  });
  const clipListRef = useRef<HTMLDivElement | null>(null);
  const [stackedPinnedClipIds, setStackedPinnedClipIds] = useState<number[]>([]);
  const pinnedShelfClips = useMemo(
    () => (currentTab === 'all' || currentTab === 'pinned')
      ? displayedClips.filter((clip) => clip.is_pinned)
      : [],
    [currentTab, displayedClips],
  );
  const pinnedShelfSignature = pinnedShelfClips
    .map((clip) => `${clip.id}:${clip.pin_order ?? 0}`)
    .join('|');

  useEffect(() => {
    setStackedPinnedClipIds([]);
  }, [selectionViewKey]);

  const handleClipListScroll = useCallback((element: HTMLDivElement) => {
    if (pinnedShelfClips.length === 0 || (currentTab !== 'all' && currentTab !== 'pinned')) {
      setStackedPinnedClipIds([]);
      return;
    }
    const pinnedCards = element.querySelectorAll<HTMLElement>('[data-pinned-clip="true"]');
    if (pinnedCards.length === 0) {
      setStackedPinnedClipIds([]);
      return;
    }
    const listTop = element.getBoundingClientRect().top;
    setStackedPinnedClipIds((previous) => {
      const previousIds = new Set(previous);
      const next = Array.from(pinnedCards).flatMap((card) => {
        const id = Number(card.dataset.clipId);
        if (!Number.isFinite(id)) return [];
        const rect = card.getBoundingClientRect();
        const shouldStack = previousIds.has(id)
          ? rect.bottom <= listTop + 12
          : rect.bottom <= listTop;
        return shouldStack ? [id] : [];
      });
      return next.length === previous.length && next.every((id, index) => id === previous[index])
        ? previous
        : next;
    });
  }, [currentTab, pinnedShelfClips.length]);

  useLayoutEffect(() => {
    const element = clipListRef.current;
    if (!element) return undefined;
    let secondFrame = 0;
    const firstFrame = requestAnimationFrame(() => {
      secondFrame = requestAnimationFrame(() => handleClipListScroll(element));
    });
    return () => {
      cancelAnimationFrame(firstFrame);
      if (secondFrame) cancelAnimationFrame(secondFrame);
    };
  }, [handleClipListScroll, pinnedShelfSignature]);

  const selectPinnedShelfClip = useCallback((clip: ClipItem) => {
    const index = displayedClips.findIndex((item) => item.id === clip.id);
    setSelectedIndex(index);
    setSelectedClip(clip);
    setSelectedClipIds(new Set([clip.id]));
    selectedClipByViewRef.current.set(selectionViewKey, clip.id);
  }, [displayedClips, selectionViewKey]);

  // Each section and Bin remembers its own inspector selection. Moving into a
  // view restores that clip (or its first eligible clip), while an explicit
  // dismissal remains dismissed until the user navigates away.
  useLayoutEffect(() => {
    const displayedIds = new Set(displayedClips.map((clip) => clip.id));
    const viewChanged = activeSelectionViewRef.current !== selectionViewKey;
    const rememberedId = selectedClipByViewRef.current.get(selectionViewKey);
    activeSelectionViewRef.current = selectionViewKey;

    const selectFallback = () => {
      const fallback = displayedClips[0] ?? null;
      selectedClipByViewRef.current.set(selectionViewKey, fallback?.id ?? null);
      setSelectedClip(fallback);
      setSelectedClipIds(fallback ? new Set([fallback.id]) : new Set());
      setSelectedIndex(fallback ? 0 : -1);
    };

    if (displayedClips.length === 0) {
      selectedClipByViewRef.current.set(selectionViewKey, null);
      setSelectedClip(null);
      setSelectedClipIds(new Set());
      setSelectedIndex(-1);
      return;
    }

    if (viewChanged) {
      const rememberedClip = typeof rememberedId === 'number'
        ? displayedClips.find((clip) => clip.id === rememberedId)
        : null;
      const nextClip = rememberedClip ?? displayedClips[0];
      const nextIndex = displayedClips.findIndex((clip) => clip.id === nextClip.id);
      selectedClipByViewRef.current.set(selectionViewKey, nextClip.id);
      setSelectedClip(nextClip);
      setSelectedClipIds(new Set([nextClip.id]));
      setSelectedIndex(nextIndex);
      return;
    }

    if (selectedClip) {
      const currentIndex = displayedClips.findIndex((clip) => clip.id === selectedClip.id);
      if (currentIndex === -1) {
        selectFallback();
        return;
      }

      const currentClip = displayedClips[currentIndex];
      selectedClipByViewRef.current.set(selectionViewKey, currentClip.id);
      setSelectedClip(currentClip);
      setSelectedIndex(currentIndex);
    } else if (typeof rememberedId === 'number' && !displayedIds.has(rememberedId)) {
      // A selected clip was removed before its deletion/update completed.
      selectFallback();
      return;
    } else {
      selectedClipByViewRef.current.set(selectionViewKey, null);
      setSelectedClipIds(new Set());
      setSelectedIndex(-1);
      return;
    }

    setSelectedClipIds((previous) => {
      const next = new Set(Array.from(previous).filter((id) => displayedIds.has(id)));
      if (next.size === previous.size && Array.from(next).every((id) => previous.has(id))) {
        return previous;
      }
      return next;
    });
  }, [displayedClips, selectedClip?.id, selectionViewKey]);

  // Keyboard navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === '\\') {
        e.preventDefault();
        setIsSidebarCollapsed((prev) => !prev);
        return;
      }

      if (['INPUT', 'TEXTAREA', 'SELECT'].includes((e.target as HTMLElement).tagName)) {
        return;
      }

      if (displayedClips.length === 0) return;

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex((prev) => {
          const next = Math.min(prev + 1, displayedClips.length - 1);
          setSelectedClip(displayedClips[next]);
          setSelectedClipIds(new Set([displayedClips[next].id]));
          return next;
        });
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex((prev) => {
          const next = Math.max(prev - 1, 0);
          setSelectedClip(displayedClips[next]);
          setSelectedClipIds(new Set([displayedClips[next].id]));
          return next;
        });
      } else if (e.key === 'Enter' && selectedClip) {
        e.preventDefault();
        handleCopyClip(selectedClip);
      } else if (e.key === 'Delete' || e.key === 'Backspace') {
        if (selectedClip) {
          e.preventDefault();
          if (getClipViewPolicy(currentTab, selectedClip).state === 'trash') {
            handlePurgeClipPermanently(selectedClip.id);
          } else {
            handleDeleteClip(selectedClip.id);
          }
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [currentTab, displayedClips, selectedClip]);

  const clipSelectionVersion = useMemo(
    () => `${selectedClip?.id ?? ''}:${Array.from(selectedClipIds).sort((a, b) => a - b).join(',')}`,
    [selectedClip?.id, selectedClipIds]
  );
  const binsById = useMemo(() => new Map(bins.map((bin) => [bin.id, bin])), [bins]);
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
    runTransformForClip: handleRunTransformForClip,
    addToSequentialStack: handleAddToSequentialStack,
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
    keepTrashedClipsVisible: currentTab === 'search',
  });

  const handleAssignClipToBin = useCallback(
    (clipId: number, binId: number) => assignClipToBin(
      clipId,
      binId,
      { includeSelection: true, playSound: true },
    ),
    [assignClipToBin],
  );

  const handleClipDropAction = useCallback((clipId: number, action: ClipDropAction) => {
    if (action === 'pin') {
      handleSetPinned(clipId, true);
    } else if (action === 'protect') {
      handleSetProtected(clipId, true);
    } else {
      handleDeleteClip(clipId);
    }
  }, [handleDeleteClip, handleSetPinned, handleSetProtected]);

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
  } = useClipBinDrag({
    allClips,
    setAllClips,
    bins,
    selectedClipIds,
    fetchClips,
    assignClipToBin: handleAssignClipToBin,
    applyClipDropAction: handleClipDropAction,
  });

  useEffect(() => {
    if (draggedClipId !== null) setHoveredClipId(null);

    const updateHoveredClip = (event: PointerEvent) => {
      if (draggedClipId !== null) {
        setHoveredClipId((current) => current === null ? current : null);
        return;
      }
      const card = document
        .elementFromPoint(event.clientX, event.clientY)
        ?.closest<HTMLElement>('[data-clip-id]');
      const candidateId = Number(card?.dataset.clipId);
      const nextId = Number.isInteger(candidateId) && candidateId > 0 ? candidateId : null;
      setHoveredClipId((current) => current === nextId ? current : nextId);
    };

    const clearHoveredClipOutsideWindow = (event: PointerEvent) => {
      if (!event.relatedTarget) setHoveredClipId(null);
    };

    window.addEventListener('pointermove', updateHoveredClip, { passive: true });
    window.addEventListener('pointerout', clearHoveredClipOutsideWindow);
    return () => {
      window.removeEventListener('pointermove', updateHoveredClip);
      window.removeEventListener('pointerout', clearHoveredClipOutsideWindow);
    };
  }, [draggedClipId]);

  const handleAssignBin = useCallback(
    (clipId: number, binId: number | null) => assignClipToBin(clipId, binId),
    [assignClipToBin],
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

  const handlePromptAddNote = (clip: ClipItem) => {
    setNotePromptClip(clip);
    setNotePromptText(clip.note || '');
  };

  const handleClearHistory = async () => {
    if (!clearHistoryMode) return;
    try {
      if (clearHistoryMode === 'purge') await invoke('purge_unpinned_clips');
      else await invoke('trash_unpinned_clips');
      setClearHistoryMode(null);
      await Promise.all([fetchClips(), fetchTrashedClips(), fetchBins()]);
    } catch (e) {
      console.error(e);
    }
  };

  if (isHudView) {
    return <QuickHudWindow />;
  }

  return (
    <div className={`app-shell flex h-screen w-screen overflow-hidden font-sans ${clipDragPreview ? 'cursor-grabbing' : ''} ${
      draggedClipId !== null ? 'is-dragging-clip' : ''
    } ${
      isPinnedReorderSettling ? 'is-settling-pinned-reorder' : ''
    } ${
      isResizingSidebar || isResizingList ? 'is-resizing-columns' : ''
    }`}>
      {clipDragPreview && (() => {
        const previewClip = allClips.find((clip) => clip.id === clipDragPreview.clipId);
        if (!previewClip) return null;
        const batchCount = selectedClipIds.has(previewClip.id) ? selectedClipIds.size : 1;
        return (
          <div
            data-testid="clip-drag-preview"
            className="clip-drag-preview fixed w-64 pointer-events-none rounded-xl border px-3 py-2.5 shadow-2xl"
            style={{
              left: clipDragPreview.x + 14,
              top: clipDragPreview.y + 14,
              transform: 'rotate(1.5deg)',
            }}
          >
            <div className="theme-text-muted flex items-center justify-between gap-3 text-[10px]">
              <span className="theme-text-main truncate font-semibold">{previewClip.source_app}</span>
              {batchCount > 1 && (
                <span className="clip-drag-preview-count shrink-0 rounded-full px-2 py-0.5 font-bold">
                  {batchCount} clips
                </span>
              )}
            </div>
            <div className="theme-title mt-1.5 truncate font-mono text-xs">
              {previewClip.content_type === 'image'
                ? 'Image clip'
                : previewClip.content_type === 'file'
                ? getClipFileSummary(previewClip)
                : previewClip.text_content || 'Empty clip'}
            </div>
          </div>
        );
      })()}
      {/* Left macOS Sidebar */}
      <Sidebar
        currentTab={currentTab}
        setCurrentTab={navigateToTab}
        selectedBinId={selectedBinId}
        setSelectedBinId={setSelectedBinId}
        bins={bins}
        onRefreshBins={fetchBins}
        onOpenNewBinModal={() => {
          setEditingBin(null);
          setIsBinModalOpen(true);
        }}
        onEditBin={(bin) => {
          setEditingBin(bin);
          setIsBinModalOpen(true);
        }}
        onDeleteBin={(bin) => setBinToDelete(bin)}
        onBinContextMenu={(x, y, bin) => setBinContextMenu({ x, y, bin })}
        onClipDropOnBin={handleAssignClipToBin}
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
        onClearHistory={() => setClearHistoryMode('purge')}
        pinnedCount={pinnedCount}
        protectedCount={protectedCount}
        notesCount={notesCount}
        trashedCount={trashedClips.length}
        totalClipCount={totalClipCount}
        isCollapsed={isSidebarCollapsed}
        setIsCollapsed={setIsSidebarCollapsed}
        sidebarWidth={sidebarWidth}
      />

      {/* Sidebar Resizer Handle (Only active when sidebar is expanded) */}
      {!isSidebarCollapsed && (
        <div
          onPointerDown={handleSidebarPointerDown}
          className="column-resizer relative w-[1px] h-screen cursor-col-resize z-30 shrink-0 select-none touch-none"
          title="Resize Sidebar"
        >
          <div className={`column-resizer-line w-[1px] h-full transition-colors ${isResizingSidebar ? 'is-active' : ''}`} />
          <div className="absolute inset-y-0 -left-1 -right-1 z-40 cursor-col-resize" />
        </div>
      )}

      {/* Main Content Area */}
      {currentTab === 'transformations' ? (
        <TransformationsView pipelines={pipelines} onRefreshPipelines={fetchPipelines} />
      ) : currentTab === 'activity' ? (
        <ActivityLogView />
      ) : currentTab === 'analytics' ? (
        <AnalyticsView />
      ) : currentTab === 'help' ? (
        <HelpView />
      ) : currentTab === 'settings' ? (
        <SettingsModal
          settings={appSettings}
          onUpdateSettings={handleUpdateSettings}
          blacklistApps={blacklistApps}
          onAddBlacklistApp={handleAddBlacklistApp}
          onRemoveBlacklistApp={handleRemoveBlacklistApp}
          onToggleBlacklistRule={handleToggleBlacklistRule}
          onRefreshPipelines={fetchPipelines}
          bins={bins}
          onRefreshBins={fetchBins}
          onRefreshClips={fetchClips}
          onClearHistory={(permanent) => setClearHistoryMode(permanent ? 'purge' : 'trash')}
          onResetColumnWidths={resetColumnWidths}
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
              className="h-[60px] border-b px-3 flex items-center justify-between col-list-header cursor-default titlebar-drag-handle shrink-0"
            >
              <div className="flex items-center space-x-2 titlebar-drag-handle min-w-0 flex-1 mr-2">
                {currentTab === 'search' ? (
                  <Search className="theme-text-main w-4 h-4 titlebar-drag-handle shrink-0" />
                ) : (
                  <Clipboard className="theme-text-main w-4 h-4 titlebar-drag-handle shrink-0" />
                )}
                <h2 className="theme-title text-xs font-bold uppercase tracking-wider titlebar-drag-handle truncate">
                  {currentTab === 'search'
                    ? 'Search'
                    : currentTab === 'pinned'
                    ? 'Pinned'
                    : currentTab === 'protected'
                    ? 'Protected'
                    : currentTab === 'notes'
                    ? 'Noted'
                    : currentTab === 'sequential'
                    ? 'Queue'
                    : currentTab === 'trash'
                    ? 'Trashed'
                    : selectedBinId
                    ? bins.find((b) => b.id === selectedBinId)?.name || 'Bin'
                    : 'All'}
                </h2>
                {currentTab === 'search' && (
                  <span
                    className="theme-badge min-w-5 rounded-md border px-1.5 py-0.5 text-center font-mono text-[10px] font-semibold"
                    aria-label={`${displayedClips.length} search ${displayedClips.length === 1 ? 'result' : 'results'}`}
                    title={`${displayedClips.length} ${displayedClips.length === 1 ? 'Result' : 'Results'}`}
                  >
                    {displayedClips.length}
                  </span>
                )}
              </div>

              {/* Global Controls & Status Badges */}
              <div className="flex items-center space-x-1.5 shrink-0">
                {ignoredAppStatus && (
                  <span className="theme-status-danger text-[10px] px-2 py-0.5 rounded border font-mono flex items-center animate-in fade-in">
                    Ignored: {ignoredAppStatus.app_name}
                  </span>
                )}

                {currentTab === 'trash' && (
                  <button
                    onClick={handleEmptyTrash}
                    disabled={trashedClips.length === 0}
                    className="theme-status-danger px-2 py-1 rounded-lg border text-xs font-semibold disabled:opacity-40 transition-colors cursor-pointer flex items-center space-x-1"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                    <span>Empty Trash</span>
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
                  title={isClipboardPaused ? 'Resume History' : 'Pause History'}
                >
                  <Pause
                    className={`w-4 h-4 ${isClipboardPaused ? 'fill-current animate-pulse' : ''}`}
                    strokeWidth={2.5}
                  />
                </button>

                {/* Copy Queue Record/Stop Toggle Button */}
                <button
                  onClick={handleToggleCopyQueue}
                  className={`list-toolbar-button w-7 h-7 flex items-center justify-center rounded-lg border transition-[background-color,border-color,color] cursor-pointer ${
                    seqStatus?.is_active
                      ? 'is-queue-active shadow-sm'
                      : ''
                  }`}
                  title={seqStatus?.is_active ? `Stop Queue (${seqStatus.queue.length})` : 'Start Queue'}
                >
                  {seqStatus?.is_active ? (
                    <Square className="w-3.5 h-3.5 fill-current animate-pulse" strokeWidth={2.5} />
                  ) : (
                    <Disc className="w-4 h-4 transition-colors" strokeWidth={2.5} />
                  )}
                </button>
              </div>
            </div>

            {/* Sequential Paste Top Header Banner if active */}
            {currentTab === 'sequential' && (
              <div className={`queue-controls-region p-3 border-b ${seqStatus?.is_active ? 'is-active' : ''}`}>
                <SequentialQueueBar
                  status={seqStatus}
                  onRefresh={fetchSequentialStatus}
                />
              </div>
            )}

            {/* Clips List Content */}
            <div className="relative flex-1 min-h-0">
              <PinnedClipShelf
                clips={pinnedShelfClips}
                stackedClipIds={stackedPinnedClipIds}
                selectedClipId={selectedClip?.id}
                onSelect={selectPinnedShelfClip}
                onRevealAll={() => clipListRef.current?.scrollTo({ top: 0, behavior: 'smooth' })}
                onWheel={(event) => {
                  if (clipListRef.current) clipListRef.current.scrollTop += event.deltaY;
                }}
              />
              <div
                ref={clipListRef}
                data-clip-list
                className="h-full overflow-y-auto pl-3 pr-3 py-3 space-y-2.5 custom-scrollbar"
                onScroll={(event) => handleClipListScroll(event.currentTarget)}
                onClick={(event) => {
                  if (event.target === event.currentTarget) clearClipSelection();
                }}
              >
                {displayedClips.length === 0 ? (
                  <EmptyClipList
                    currentTab={currentTab}
                    searchQuery={searchQuery}
                    selectedBin={selectedBinId === null ? undefined : binsById.get(selectedBinId)}
                  />
                ) : (
                  displayedClips.map((clip, index) => {
                  const queueIndex = clip.text_content ? queuedIndexMap.get(clip.text_content) : undefined;
                  const primaryBin = clip.bin_id === null ? undefined : binsById.get(clip.bin_id);
                  const baseViewPolicy = getClipViewPolicy(currentTab, clip);
                  const viewPolicy = hasRestrictedSelection && selectedClipIds.has(clip.id)
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
                      reorderOffsetY={pinnedReorderOffsets[clip.id] ?? 0}
                      viewPolicy={viewPolicy}
                      isQueueMode={currentTab === 'sequential'}
                      queueIndex={queueIndex}
                      primaryBinName={primaryBin?.name}
                      primaryBinIcon={primaryBin?.icon}
                      rowHeight={appSettings.rowHeight}
                      filePreviewMode={appSettings.filePreviewMode}
                      filePreviewMaxMb={appSettings.filePreviewMaxMb}
                      selectionVersion={clipSelectionVersion}
                      trashEnabled={appSettings.enableTrash}
                      searchQuery={currentTab === 'search' ? searchQuery : undefined}
                      setDraggedClipId={setDraggedClipId}
                      onPointerDragStart={(id) => {
                        setHoveredClipId(null);
                        beginPinnedReorderPreview(id);
                        setDraggedClipId(id);
                      }}
                      onPointerDragMove={(x, y) => {
                        updatePointerDropTarget(x, y);
                        updatePinnedReorderPreview(x, y, clip.id);
                        setClipDragPreview({ clipId: clip.id, x, y });
                      }}
                      onPointerDragEnd={handleClipPointerDragEnd}
                      onPointerDragCancel={() => {
                        setPointerDropTargetBinId(null);
                        setPointerDropTargetAction(null);
                        cancelPinnedReorderPreview();
                        setClipDragPreview(null);
                      }}
                      onSelect={(e) => {
                        setSelectedIndex(index);

                        if (e.metaKey || e.ctrlKey) {
                          setSelectedClipIds((prev) => {
                            const next = new Set(prev);
                            if (next.has(clip.id)) {
                              next.delete(clip.id);
                              if (selectedClip?.id === clip.id) {
                                const remaining = Array.from(next);
                                const lastId = remaining[remaining.length - 1];
                                const nextSelected = displayedClips.find((c) => c.id === lastId);
                                setSelectedClip(nextSelected || null);
                              }
                            } else {
                              next.add(clip.id);
                              setSelectedClip(clip);
                            }
                            return next;
                          });
                        } else if (e.shiftKey && selectedClip) {
                          const currIdx = displayedClips.findIndex((c) => c.id === clip.id);
                          const lastIdx = displayedClips.findIndex((c) => c.id === selectedClip.id);
                          if (currIdx !== -1 && lastIdx !== -1) {
                            const start = Math.min(currIdx, lastIdx);
                            const end = Math.max(currIdx, lastIdx);
                            const rangeIds = displayedClips.slice(start, end + 1).map((c) => c.id);
                            setSelectedClipIds(new Set(rangeIds));
                          }
                        } else {
                          const isOnlySelectedClip = selectedClip?.id === clip.id && selectedClipIds.size <= 1;
                          if (isOnlySelectedClip) {
                            clearClipSelection();
                          } else {
                            setSelectedClip(clip);
                            setSelectedClipIds(new Set([clip.id]));
                          }
                        }
                      }}
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
                        if (idx === 0) {
                          invoke('pop_sequential_paste').then(fetchSequentialStatus);
                        } else if (idx !== -1) {
                          invoke('remove_sequential_item_by_index', { index: idx }).then(fetchSequentialStatus);
                        }
                      }}
                      onCopy={() => handleCopyClip(clip)}
                      onContextMenu={(e) => {
                        e.preventDefault();
                        setSelectedIndex(index);
                        setSelectedClip(clip);
                        setSelectedClipIds(new Set([clip.id]));
                        setContextMenu({
                          x: e.clientX,
                          y: e.clientY,
                          clip,
                        });
                      }}
                    />
                  );
                  })
                )}
              </div>
            </div>

            {/* Floating Glass Batch Action Bar */}
            {selectedClipIds.size > 1 && selectedClipViewPolicy.showOrganizeBatchActions && !hasRestrictedSelection && (
              <div className="batch-action-bar absolute bottom-4 left-1/2 -translate-x-1/2 border rounded-2xl px-3 py-1.5 shadow-2xl flex items-center space-x-2 text-[11px] whitespace-nowrap animate-in fade-in slide-in-from-bottom-2 duration-150 max-w-[calc(100%-1.5rem)] select-none">
                <span className="batch-action-count font-bold font-mono text-[11px] px-2 py-0.5 rounded-full border whitespace-nowrap shrink-0">
                  {selectedClipIds.size}
                </span>
                <div className="batch-action-divider h-3.5 w-px shrink-0" />
                <button
                  onClick={() => {
                    const ids = Array.from(selectedClipIds);
                    const idSet = new Set(ids);
                    setAllClips((previous) => {
                      const newlyPinned = previous
                        .filter((clip) => idSet.has(clip.id))
                        .map((clip, index) => ({ ...clip, is_pinned: true, pin_order: index }));
                      const existingPinned = previous
                        .filter((clip) => clip.is_pinned && !idSet.has(clip.id))
                        .map((clip) => ({ ...clip, pin_order: (clip.pin_order ?? 0) + newlyPinned.length }));
                      return sortClipsForTimeline([
                        ...newlyPinned,
                        ...existingPinned,
                        ...previous.filter((clip) => !clip.is_pinned && !idSet.has(clip.id)),
                      ]);
                    });
                    invoke('batch_pin_clips', { ids, pinState: true }).catch((err) => {
                      console.error(err);
                      fetchClips();
                    });
                  }}
                  className="batch-action-button flex items-center space-x-1 transition-colors font-medium cursor-pointer whitespace-nowrap shrink-0"
                  title="Pin Selected"
                >
                  <Pin className="pin-icon w-3.5 h-3.5 shrink-0" />
                  <span>Pin</span>
                </button>
                <button
                  onClick={() => {
                    const ids = Array.from(selectedClipIds);
                    const idSet = new Set(ids);
                    setAllClips((previous) => {
                      const updated = previous.map((clip) => idSet.has(clip.id)
                        ? { ...clip, is_pinned: false, pin_order: 0 }
                        : clip);
                      return sortClipsForTimeline(updated);
                    });
                    invoke('batch_pin_clips', { ids, pinState: false }).catch((err) => {
                      console.error(err);
                      fetchClips();
                    });
                  }}
                  className="batch-action-button flex items-center space-x-1 transition-colors font-medium cursor-pointer whitespace-nowrap shrink-0"
                  title="Unpin Selected"
                >
                  <Pin className="theme-text-muted w-3.5 h-3.5 opacity-60 shrink-0" />
                  <span>Unpin</span>
                </button>
                <div className="batch-action-divider h-3.5 w-px shrink-0" />
                <button
                  onClick={() => handleBatchTrash()}
                  className="batch-action-button is-danger flex items-center space-x-1 transition-colors font-medium cursor-pointer whitespace-nowrap shrink-0"
                  title={appSettings.enableTrash ? 'Move Selected to Trash' : 'Delete Selected Permanently'}
                >
                  <Trash2 className="w-3.5 h-3.5 shrink-0" />
                  <span>Trash</span>
                </button>
                <button
                  onClick={clearClipSelection}
                  className="batch-action-button p-0.5 rounded-full transition-colors cursor-pointer shrink-0 ml-0.5"
                  title="Deselect"
                >
                  <X className="w-3.5 h-3.5 shrink-0" />
                </button>
              </div>
            )}
          </div>

          {/* List Resizer Handle (Exact 1px visual border line with grab target extending to right) */}
          <div
            onPointerDown={handleListPointerDown}
            className="column-resizer relative w-[1px] h-screen cursor-col-resize z-20 shrink-0 select-none touch-none"
            title="Resize Clip List"
          >
            <div className={`column-resizer-line w-[1px] h-full transition-colors ${isResizingList ? 'is-active' : ''}`} />
            <div className="absolute inset-y-0 left-0 -right-2 z-20 cursor-col-resize" />
          </div>

          {/* Right Detail Preview Panel */}
          <ClipPreview
            clip={selectedClip}
            viewPolicy={selectedClipViewPolicy}
            bins={bins}
            pipelines={pipelines}
            onUpdateClip={handlePreviewClipUpdate}
            onAssignBin={handleAssignBin}
            onTogglePin={handleTogglePin}
            onToggleProtected={handleToggleProtected}
            onDeleteClip={selectedClipViewPolicy.state === 'trash' ? handlePurgeClipPermanently : handleDeleteClip}
            onUpdateClipNote={handleUpdateClipNoteLocally}
            isTransforming={selectedClip ? transformingClipIds.has(selectedClip.id) : false}
            transformError={selectedClip ? transformErrorsByClipId.get(selectedClip.id) : undefined}
            onOpenTransformations={() => navigateToTab('transformations')}
            trashEnabled={appSettings.enableTrash}
            filePreviewMode={appSettings.filePreviewMode}
            filePreviewMaxMb={appSettings.filePreviewMaxMb}
          />
        </div>
      )}

      {/* Right Click Context Menu */}
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          clip={contextMenu.clip}
          viewPolicy={getClipViewPolicy(currentTab, contextMenu.clip)}
          selectedCount={selectedClipIds.has(contextMenu.clip.id) ? selectedClipIds.size : 1}
          bins={bins}
          onClose={() => setContextMenu(null)}
          onCopy={() => handleCopyClip(contextMenu.clip)}
          onAssignBin={(binId) => assignClipToBin(
            contextMenu.clip.id,
            binId,
            { includeSelection: true },
          )}
          onRunTransform={(transform) => handleRunTransformForClip(contextMenu.clip, transform)}
          onOpenTransformations={() => navigateToTab('transformations')}
          onAddNote={() => handlePromptAddNote(contextMenu.clip)}
          onDeleteNote={() => handleDeleteNoteFromClip(contextMenu.clip.id)}
          onAddToStack={() => handleAddToSequentialStack(contextMenu.clip)}
          onTogglePin={() => handleTogglePin(contextMenu.clip.id)}
          onToggleProtected={() => handleToggleProtected(contextMenu.clip.id)}
          onDelete={(e) => handleDeleteClip(contextMenu.clip.id, e?.altKey)}
          onRestore={() => handleRestoreClip(contextMenu.clip.id)}
          onPurge={() => handlePurgeClipPermanently(contextMenu.clip.id)}
          trashEnabled={appSettings.enableTrash}
        />
      )}

      {/* Root-Level macOS Right-Click Context Menu for Custom Bins */}
      {binContextMenu && (
        <BinContextMenu
          menu={binContextMenu}
          onEdit={(bin) => {
            setBinContextMenu(null);
            setEditingBin(bin);
            setIsBinModalOpen(true);
          }}
          onDelete={(bin) => {
            setBinContextMenu(null);
            setBinToDelete(bin);
          }}
        />
      )}

      {/* Custom Bin Creator / Editor Modal */}
      <BinModal
        key={editingBin ? `edit-${editingBin.id}` : 'new-bin'}
        isOpen={isBinModalOpen}
        editingBin={editingBin}
        onClose={() => {
          setIsBinModalOpen(false);
          setEditingBin(null);
        }}
        onRefreshBins={fetchBins}
      />

      {/* Delete Bin Confirmation Modal */}
      {binToDelete && (
        <DeleteBinDialog
          bin={binToDelete}
          bins={bins}
          onCancel={() => setBinToDelete(null)}
          onConfirm={async (bin, disposition, destinationBinId) => {
            try {
              await invoke('delete_bin', {
                id: bin.id,
                disposition,
                destinationBinId,
              });
              setBinToDelete(null);
              await Promise.all([fetchBins(), fetchClips(), fetchTrashedClips()]);
              if (selectedBinId === bin.id) {
                navigateToTab('all');
                setSelectedBinId(null);
              }
            } catch (err) {
              console.error(err);
            }
          }}
        />
      )}

      {/* Add / Edit Note Modal */}
      {notePromptClip && (
        <ClipNoteDialog
          clip={notePromptClip}
          text={notePromptText}
          onTextChange={setNotePromptText}
          onCancel={() => setNotePromptClip(null)}
          onSave={async (clip, note) => {
            handleUpdateClipNoteLocally(clip.id, note);
            setNotePromptClip(null);
            try {
              await invoke('update_clip_note', { clipId: clip.id, note });
            } catch (error) {
              console.error(error);
              fetchClips();
            }
          }}
        />
      )}

      {/* Clear History Confirmation Modal */}
      {clearHistoryMode && (
        <ClearHistoryDialog
          mode={clearHistoryMode}
          onCancel={() => setClearHistoryMode(null)}
          onConfirm={handleClearHistory}
        />
      )}
    </div>
  );
}
