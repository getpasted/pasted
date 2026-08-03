import { useEffect, useState, useCallback, useMemo } from 'react';
import { safeInvoke as invoke } from './utils/tauri';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ClipItem, Bin } from './types';
import { Sidebar } from './components/Sidebar';
import { ClipCard } from './components/ClipCard';
import { ClipPreview } from './components/ClipPreview';
import { SequentialQueueBar } from './components/SequentialQueueBar';
import { FilterManager } from './components/FilterManager';
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
import { useClipBinDrag } from './hooks/useClipBinDrag';
import { useAppData } from './hooks/useAppData';
import { useClipActions } from './hooks/useClipActions';
import { Clipboard, Trash2, Pause, Disc, Square, Pin, X } from 'lucide-react';
import './App.css';

export default function App() {
  const [isHudView, setIsHudView] = useState<boolean>(false);

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
    filters,
    sequentialStatus: seqStatus,
    totalClipCount,
    setTotalClipCount,
    isClipboardPaused,
    ignoredAppStatus,
    fetchClips,
    fetchTrashedClips,
    fetchBins,
    fetchFilters,
    fetchSequentialStatus,
    toggleClipboardPause: handleToggleClipboardPause,
    restoreClip: handleRestoreClip,
    purgeClipPermanently: handlePurgeClipPermanently,
    emptyTrash: handleEmptyTrash,
  } = useAppData(appSettings.enableSounds);

  const [selectedClip, setSelectedClip] = useState<ClipItem | null>(null);
  const [selectedClipIds, setSelectedClipIds] = useState<Set<number>>(new Set());
  const [hoveredClipId, setHoveredClipId] = useState<number | null>(null);
  const [, setSelectedIndex] = useState<number>(0);
  const [currentTab, setCurrentTab] = useState<string>('all');
  const [selectedBinId, setSelectedBinId] = useState<number | null>(null);
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [isBinModalOpen, setIsBinModalOpen] = useState<boolean>(false);
  const [editingBin, setEditingBin] = useState<Bin | null>(null);
  const [clearHistoryMode, setClearHistoryMode] = useState<ClearHistoryMode | null>(null);
  const isClearConfirmOpen = clearHistoryMode !== null;
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState<boolean>(false);

  const handleToggleCopyQueue = async () => {
    try {
      if (seqStatus?.is_active) {
        await invoke('stop_sequential_paste');
      } else {
        await invoke('start_sequential_paste');
        setCurrentTab('sequential');
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

  // Keep selected clip valid on view switch
  useEffect(() => {
    if (displayedClips.length > 0) {
      setSelectedClip((prev) => {
        if (prev) {
          const found = displayedClips.find((c) => c.id === prev.id);
          return found || displayedClips[0];
        }
        return displayedClips[0];
      });
    } else {
      setSelectedClip(null);
    }
  }, [displayedClips]);

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
          handleDeleteClip(selectedClip.id);
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [displayedClips, selectedClip]);

  const clipSelectionVersion = useMemo(
    () => `${selectedClip?.id ?? ''}:${Array.from(selectedClipIds).sort((a, b) => a - b).join(',')}`,
    [selectedClip?.id, selectedClipIds]
  );

  const {
    togglePin: handleTogglePin,
    toggleProtected: handleToggleProtected,
    deleteSelectedClips: handleBatchTrash,
    deleteClip: handleDeleteClip,
    copyClip: handleCopyClip,
    assignClipToBin,
    applyFilterToClip: handleApplyFilterToClip,
    addToSequentialStack: handleAddToSequentialStack,
    updateClipNoteLocally: handleUpdateClipNoteLocally,
    deleteNoteFromClip: handleDeleteNoteFromClip,
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
  });

  const handleAssignClipToBin = useCallback(
    (clipId: number, binId: number) => assignClipToBin(
      clipId,
      binId,
      { includeSelection: true, playSound: true },
    ),
    [assignClipToBin],
  );

  const {
    draggedClipId,
    setDraggedClipId,
    pointerDropTargetBinId,
    setPointerDropTargetBinId,
    clipDragPreview,
    setClipDragPreview,
    disabledDropBinId,
    getPointerDropTarget,
    finishClipPointerDrag: handleClipPointerDragEnd,
  } = useClipBinDrag({
    allClips,
    setAllClips,
    bins,
    selectedClipIds,
    fetchClips,
    assignClipToBin: handleAssignClipToBin,
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

  const handlePromptAddNote = (clip: ClipItem) => {
    setNotePromptClip(clip);
    setNotePromptText(clip.note || '');
  };

  const handleClearHistory = async () => {
    if (!clearHistoryMode) return;
    try {
      await invoke(clearHistoryMode === 'purge' ? 'purge_unpinned_clips' : 'trash_unpinned_clips');
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
    <div className={`flex h-screen w-screen overflow-hidden bg-[#171717] text-gray-100 font-sans ${clipDragPreview ? 'cursor-grabbing' : ''} ${
      draggedClipId !== null ? 'is-dragging-clip' : ''
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
            className="fixed z-[100000] w-64 pointer-events-none rounded-xl border border-cyan-400/70 bg-[#252525]/95 px-3 py-2.5 shadow-2xl shadow-black/60 ring-1 ring-white/10"
            style={{
              left: clipDragPreview.x + 14,
              top: clipDragPreview.y + 14,
              transform: 'rotate(1.5deg)',
            }}
          >
            <div className="flex items-center justify-between gap-3 text-[10px] text-gray-400">
              <span className="truncate font-semibold text-gray-300">{previewClip.source_app}</span>
              {batchCount > 1 && (
                <span className="shrink-0 rounded-full bg-cyan-500 px-2 py-0.5 font-bold text-black">
                  {batchCount} clips
                </span>
              )}
            </div>
            <div className="mt-1.5 truncate font-mono text-xs text-gray-100">
              {previewClip.content_type === 'image' ? 'Image clip' : previewClip.text_content || 'Empty clip'}
            </div>
          </div>
        );
      })()}
      {/* Left macOS Sidebar */}
      <Sidebar
        currentTab={currentTab}
        setCurrentTab={setCurrentTab}
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
        disabledDropBinId={disabledDropBinId}
        searchQuery={searchQuery}
        setSearchQuery={setSearchQuery}
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
          title="Drag to resize sidebar width"
        >
          <div className={`column-resizer-line w-[1px] h-full transition-colors ${isResizingSidebar ? 'is-active' : ''}`} />
          <div className="absolute inset-y-0 -left-1 -right-1 z-40 cursor-col-resize" />
        </div>
      )}

      {/* Main Content Area */}
      {currentTab === 'filters' || currentTab === 'operations' ? (
        <FilterManager filters={filters} onRefreshFilters={fetchFilters} />
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
          filters={filters}
          onRefreshFilters={fetchFilters}
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
            className="shrink-0 col-list h-screen flex flex-col bg-[#171717] overflow-hidden"
          >
            {/* Finder Header Title Bar */}
            <div
              onMouseDown={startWindowDrag}
              className="h-[60px] border-b border-[#2b2b2b] bg-[#171717]/80 backdrop-blur-md px-3 flex items-center justify-between col-list-header cursor-default titlebar-drag-handle shrink-0"
            >
              <div className="flex items-center space-x-2 titlebar-drag-handle min-w-0 flex-1 mr-2">
                <Clipboard className="w-4 h-4 text-gray-300 titlebar-drag-handle shrink-0" />
                <h2 className="text-xs font-bold text-gray-200 uppercase tracking-wider titlebar-drag-handle truncate">
                  {currentTab === 'pinned'
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
              </div>

              {/* Global Controls & Status Badges */}
              <div className="flex items-center space-x-1.5 shrink-0">
                {ignoredAppStatus && (
                  <span className="text-[10px] px-2 py-0.5 rounded bg-red-950/80 border border-red-800/60 text-red-300 font-mono flex items-center animate-in fade-in">
                    Ignored: {ignoredAppStatus.app_name}
                  </span>
                )}

                {currentTab === 'trash' && (
                  <button
                    onClick={handleEmptyTrash}
                    disabled={trashedClips.length === 0}
                    className="px-2 py-1 rounded-lg bg-rose-600/20 hover:bg-rose-600/30 text-rose-300 border border-rose-500/30 text-xs font-semibold disabled:opacity-40 transition-all cursor-pointer flex items-center space-x-1"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                    <span>Empty Trash</span>
                  </button>
                )}

                {/* Pause History Toggle Button */}
                <button
                  onClick={handleToggleClipboardPause}
                  className={`w-7 h-7 flex items-center justify-center rounded-lg transition-all cursor-pointer ${
                    isClipboardPaused
                      ? 'bg-amber-500/20 text-amber-400 border border-amber-500/40 shadow-sm'
                      : 'text-gray-400 hover:text-gray-200 hover:bg-gray-800 border border-transparent'
                  }`}
                  title={isClipboardPaused ? 'Resume History Recording' : 'Pause History Recording (for sensitive items/passwords)'}
                >
                  <Pause
                    className={`w-4 h-4 ${isClipboardPaused ? 'fill-amber-400 text-amber-400 animate-pulse' : 'text-gray-400'}`}
                    strokeWidth={2.5}
                  />
                </button>

                {/* Copy Queue Record/Stop Toggle Button */}
                <button
                  onClick={handleToggleCopyQueue}
                  className={`w-7 h-7 flex items-center justify-center rounded-lg transition-all cursor-pointer ${
                    seqStatus?.is_active
                      ? 'bg-purple-500/20 text-purple-300 border border-purple-500/40 shadow-sm'
                      : 'text-gray-400 hover:text-gray-200 hover:bg-gray-800 border border-transparent'
                  }`}
                  title={seqStatus?.is_active ? `Stop Queue Recording (${seqStatus.queue.length} items queued)` : 'Start Queue Recording'}
                >
                  {seqStatus?.is_active ? (
                    <Square className="w-3.5 h-3.5 fill-purple-400 text-purple-400 animate-pulse" strokeWidth={2.5} />
                  ) : (
                    <Disc className="w-4 h-4 text-gray-400 hover:text-purple-400 transition-colors" strokeWidth={2.5} />
                  )}
                </button>
              </div>
            </div>

            {/* Sequential Paste Top Header Banner if active */}
            {currentTab === 'sequential' && (
              <div className="p-3 bg-purple-950/30 border-b border-purple-500/30">
                <SequentialQueueBar
                  status={seqStatus}
                  onRefresh={fetchSequentialStatus}
                />
              </div>
            )}

            {/* Clips List Content */}
            <div className="flex-1 overflow-y-auto pl-3 pr-3 py-3 space-y-2.5 custom-scrollbar">
              {displayedClips.length === 0 ? (
                <div className="h-full flex flex-col items-center justify-center text-center p-6 text-gray-500 select-none">
                  <Clipboard className="w-10 h-10 mb-3 opacity-30 stroke-1" />
                  <p className="text-xs font-medium text-gray-400">No clips found</p>
                  <p className="text-[11px] text-gray-600 mt-1">
                    {searchQuery ? 'Try matching another search term' : 'Copied items will automatically show up here'}
                  </p>
                </div>
              ) : (
                displayedClips.map((clip, index) => {
                  const queueIndex = clip.text_content ? queuedIndexMap.get(clip.text_content) : undefined;

                  return (
                    <ClipCard
                      key={clip.id}
                      clip={clip}
                      isSelected={selectedClipIds.size > 0 ? selectedClipIds.has(clip.id) : selectedClip?.id === clip.id}
                      isHovered={hoveredClipId === clip.id}
                      showActions={selectedClip?.id === clip.id}
                      isDragging={draggedClipId === clip.id}
                      isDragInProgress={draggedClipId !== null}
                      isTrashMode={currentTab === 'trash'}
                      isQueueMode={currentTab === 'sequential'}
                      queueIndex={queueIndex}
                      rowHeight={appSettings.rowHeight}
                      selectionVersion={clipSelectionVersion}
                      setDraggedClipId={setDraggedClipId}
                      onPointerDragStart={(id) => {
                        setHoveredClipId(null);
                        setDraggedClipId(id);
                      }}
                      onPointerDragMove={(x, y) => {
                        setPointerDropTargetBinId(getPointerDropTarget(x, y));
                        setClipDragPreview({ clipId: clip.id, x, y });
                      }}
                      onPointerDragEnd={handleClipPointerDragEnd}
                      onPointerDragCancel={() => {
                        setPointerDropTargetBinId(null);
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
                          setSelectedClip(clip);
                          setSelectedClipIds(new Set([clip.id]));
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

            {/* Floating Glass Batch Action Bar */}
            {selectedClipIds.size > 1 && (
              <div className="absolute bottom-4 left-1/2 -translate-x-1/2 z-40 bg-[#1c1e26]/95 backdrop-blur-xl border border-cyan-500/40 rounded-2xl px-3 py-1.5 shadow-2xl flex items-center space-x-2 text-[11px] whitespace-nowrap animate-in fade-in slide-in-from-bottom-2 duration-150 max-w-[calc(100%-1.5rem)] select-none">
                <span className="font-bold text-cyan-400 font-mono text-[11px] bg-cyan-950/90 px-2 py-0.5 rounded-full border border-cyan-800/60 whitespace-nowrap shrink-0">
                  {selectedClipIds.size}
                </span>
                <div className="h-3.5 w-px bg-gray-700/80 shrink-0" />
                <button
                  onClick={() => {
                    const ids = Array.from(selectedClipIds);
                    setAllClips((prev) =>
                      prev.map((c) => (ids.includes(c.id) ? { ...c, is_pinned: true } : c))
                    );
                    invoke('batch_pin_clips', { ids, pinState: true }).catch((err) => {
                      console.error(err);
                      fetchClips();
                    });
                  }}
                  className="flex items-center space-x-1 hover:text-cyan-300 transition-colors font-medium cursor-pointer whitespace-nowrap shrink-0"
                  title="Pin All Selected"
                >
                  <Pin className="w-3.5 h-3.5 text-orange-400 shrink-0" />
                  <span>Pin</span>
                </button>
                <button
                  onClick={() => {
                    const ids = Array.from(selectedClipIds);
                    setAllClips((prev) =>
                      prev.map((c) => (ids.includes(c.id) ? { ...c, is_pinned: false } : c))
                    );
                    invoke('batch_pin_clips', { ids, pinState: false }).catch((err) => {
                      console.error(err);
                      fetchClips();
                    });
                  }}
                  className="flex items-center space-x-1 hover:text-cyan-300 transition-colors font-medium cursor-pointer whitespace-nowrap shrink-0"
                  title="Unpin All Selected"
                >
                  <Pin className="w-3.5 h-3.5 text-gray-400 opacity-60 shrink-0" />
                  <span>Unpin</span>
                </button>
                <div className="h-3.5 w-px bg-gray-700/80 shrink-0" />
                <button
                  onClick={() => handleBatchTrash()}
                  className="flex items-center space-x-1 text-red-400 hover:text-red-300 transition-colors font-medium cursor-pointer whitespace-nowrap shrink-0"
                  title="Trash Selected"
                >
                  <Trash2 className="w-3.5 h-3.5 shrink-0" />
                  <span>Trash</span>
                </button>
                <button
                  onClick={() => setSelectedClipIds(new Set())}
                  className="p-0.5 text-gray-400 hover:text-white rounded-full hover:bg-gray-800 transition-colors cursor-pointer shrink-0 ml-0.5"
                  title="Deselect All"
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
            title="Drag to resize clips list width"
          >
            <div className={`column-resizer-line w-[1px] h-full transition-colors ${isResizingList ? 'is-active' : ''}`} />
            <div className="absolute inset-y-0 left-0 -right-2 z-20 cursor-col-resize" />
          </div>

          {/* Right Detail Preview Panel */}
          <ClipPreview
            clip={selectedClip}
            bins={bins}
            filters={filters}
            onUpdateClip={fetchClips}
            onAssignBin={handleAssignBin}
            onDeleteClip={handleDeleteClip}
            onUpdateClipNote={handleUpdateClipNoteLocally}
          />
        </div>
      )}

      {/* Right Click Context Menu */}
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          clip={contextMenu.clip}
          selectedCount={selectedClipIds.has(contextMenu.clip.id) ? selectedClipIds.size : 1}
          bins={bins}
          filters={filters}
          onClose={() => setContextMenu(null)}
          onCopy={() => handleCopyClip(contextMenu.clip)}
          onAssignBin={(binId) => assignClipToBin(
            contextMenu.clip.id,
            binId,
            { includeSelection: true },
          )}
          onApplyFilter={(filter) => handleApplyFilterToClip(contextMenu.clip, filter)}
          onAddNote={() => handlePromptAddNote(contextMenu.clip)}
          onDeleteNote={() => handleDeleteNoteFromClip(contextMenu.clip.id)}
          onAddToStack={() => handleAddToSequentialStack(contextMenu.clip)}
          onTogglePin={() => handleTogglePin(contextMenu.clip.id)}
          onToggleProtected={() => handleToggleProtected(contextMenu.clip.id)}
          onDelete={(e) => handleDeleteClip(contextMenu.clip.id, e?.altKey)}
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
          onCancel={() => setBinToDelete(null)}
          onConfirm={async (bin) => {
            try {
              await invoke('delete_bin', { id: bin.id });
              setBinToDelete(null);
              fetchBins();
              if (selectedBinId === bin.id) {
                setCurrentTab('all');
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
