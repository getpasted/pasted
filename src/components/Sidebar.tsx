import React from 'react';
import { formatEmojiIcon } from '../utils/emoji';
import { startWindowDrag } from '../utils/windowDrag';
import {
  Clipboard,
  Pin,
  ListOrdered,
  Sliders,
  Settings,
  Trash2,
  Plus,
  Search,
  PanelLeftClose,
  PanelLeftOpen,
  Sparkles,
  Edit3,
  StickyNote,
  Activity,
  BarChart3,
  HelpCircle,
  Shield,
} from 'lucide-react';
import { Bin, SequentialStatus } from '../types';

interface SidebarProps {
  currentTab: string;
  setCurrentTab: (tab: string) => void;
  selectedBinId: number | null;
  setSelectedBinId: (id: number | null) => void;
  bins: Bin[];
  onRefreshBins?: () => void;
  onOpenNewBinModal: () => void;
  onEditBin?: (bin: Bin) => void;
  onDeleteBin?: (bin: Bin) => void;
  onBinContextMenu?: (x: number, y: number, bin: Bin) => void;
  searchQuery: string;
  setSearchQuery: (q: string) => void;
  seqStatus: SequentialStatus | null;
  onClearHistory?: () => void;
  totalClipCount: number;
  pinnedCount?: number;
  protectedCount?: number;
  notesCount?: number;
  trashedCount?: number;
  isCollapsed: boolean;
  setIsCollapsed: (collapsed: boolean | ((prev: boolean) => boolean)) => void;
  sidebarWidth?: number;
  onClipDropOnBin?: (clipId: number, binId: number) => void;
  draggedClipId?: number | null;
  pointerDropTargetBinId?: number | null;
  disabledDropBinId?: number | null;
}

export const Sidebar: React.FC<SidebarProps> = ({
  currentTab,
  setCurrentTab,
  selectedBinId,
  setSelectedBinId,
  bins,
  onOpenNewBinModal,
  onEditBin,
  onDeleteBin,
  onBinContextMenu,
  onClipDropOnBin,
  draggedClipId,
  pointerDropTargetBinId,
  disabledDropBinId,
  searchQuery,
  setSearchQuery,
  seqStatus,
  totalClipCount,
  pinnedCount = 0,
  protectedCount = 0,
  notesCount = 0,
  trashedCount = 0,
  isCollapsed,
  setIsCollapsed,
  sidebarWidth = 240,
}) => {

  // Section Collapse State
  const [isClipsOpen, setIsClipsOpen] = React.useState(true);
  const [isBinsOpen, setIsBinsOpen] = React.useState(true);
  const [isToolsOpen, setIsToolsOpen] = React.useState(true);

  // Bin Drag & Drop Reorder State with 150ms Debounce
  const [activeDragBinId, setActiveDragBinId] = React.useState<number | null>(null);
  const [dropTargetBinId, setDropTargetBinId] = React.useState<number | null>(null);
  const [isSearchFocused, setIsSearchFocused] = React.useState(false);
  const dragTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);

  const [binOrder, setBinOrder] = React.useState<number[]>(() => {
    try {
      const saved = localStorage.getItem('pasted_bin_order');
      return saved ? JSON.parse(saved) : [];
    } catch {
      return [];
    }
  });

  const sortedBins = React.useMemo(() => {
    if (!binOrder || binOrder.length === 0) return bins;
    return [...bins].sort((a, b) => {
      const indexA = binOrder.indexOf(a.id);
      const indexB = binOrder.indexOf(b.id);
      if (indexA === -1 && indexB === -1) return 0;
      if (indexA === -1) return 1;
      if (indexB === -1) return -1;
      return indexA - indexB;
    });
  }, [bins, binOrder]);

  const handlePointerDownBin = (binId: number) => {
    if (draggedClipId !== null && draggedClipId !== undefined) return;
    if (dragTimerRef.current) clearTimeout(dragTimerRef.current);
    dragTimerRef.current = setTimeout(() => {
      setActiveDragBinId(binId);
    }, 150);
  };

  const handlePointerUpBin = () => {
    if (dragTimerRef.current) {
      clearTimeout(dragTimerRef.current);
      dragTimerRef.current = null;
    }
    setActiveDragBinId(null);
  };

  const handlePointerEnterBin = (targetBinId: number) => {
    if (draggedClipId !== null && draggedClipId !== undefined) return;
    if (!activeDragBinId || activeDragBinId === targetBinId) return;

    const currentOrder = sortedBins.map((b) => b.id);
    const fromIndex = currentOrder.indexOf(activeDragBinId);
    const toIndex = currentOrder.indexOf(targetBinId);
    if (fromIndex === -1 || toIndex === -1) return;

    const newOrder = [...currentOrder];
    const [moved] = newOrder.splice(fromIndex, 1);
    newOrder.splice(toIndex, 0, moved);

    setBinOrder(newOrder);
    localStorage.setItem('pasted_bin_order', JSON.stringify(newOrder));
  };

  const getBinIcon = (iconName: string) => {
    return <span className="text-sm">{formatEmojiIcon(iconName)}</span>;
  };

  if (isCollapsed) {
    return (
      <aside className="w-[100px] col-sidebar h-screen flex flex-col items-center border-r border-[#2d2d2d] bg-[#212121]/90 backdrop-blur-xl select-none">
        {/* Dedicated 56px Top Header Drag Region for Traffic Lights */}
        <div
          onMouseDown={startWindowDrag}
          className="h-[56px] w-full cursor-default titlebar-drag-handle shrink-0"
        />

        {/* Scrollable Nav Items Container for small window heights */}
        <div className="w-full flex-1 overflow-y-auto overflow-x-hidden sidebar-scroll-container flex flex-col items-center gap-1.5 py-2 px-1 custom-scrollbar">
          {/* Expand Sidebar Toggle Button (Safely placed below traffic light zone) */}
          <button
            onClick={() => setIsCollapsed(false)}
            className="w-9 h-9 flex items-center justify-center p-0 text-gray-400 hover:text-white rounded-xl hover:bg-white/10 transition-colors duration-75 border border-transparent hover:border-white/10 shrink-0 cursor-pointer"
            title="Expand Sidebar (⌘\)"
          >
            <PanelLeftOpen className="w-5 h-5 text-gray-300" />
          </button>

          <div className="w-full flex items-center justify-center py-1 shrink-0">
            <div className="w-8 border-t border-white/10 sidebar-divider" />
          </div>

          <button
            onClick={() => {
              setCurrentTab('all');
              setSelectedBinId(null);
            }}
            className={`w-9 h-9 flex items-center justify-center p-0 rounded-xl transition-colors duration-75 border shrink-0 cursor-pointer ${
              currentTab === 'all' && selectedBinId === null
                ? 'sidebar-item-active bg-[#383838] text-white border-gray-600/70 shadow-sm'
                : 'sidebar-item-idle border-transparent text-gray-400 hover:bg-[#2a2a2a] hover:text-white'
            }`}
            title="All Clips"
          >
            <Clipboard className="w-5 h-5" />
          </button>

          <button
            onClick={() => {
              setCurrentTab('pinned');
              setSelectedBinId(null);
            }}
            className={`w-9 h-9 flex items-center justify-center p-0 rounded-xl transition-colors duration-75 border shrink-0 cursor-pointer ${
              currentTab === 'pinned'
                ? 'sidebar-item-active bg-[#383838] text-white border-gray-600/70 shadow-sm'
                : 'sidebar-item-idle border-transparent text-gray-400 hover:bg-[#2a2a2a] hover:text-white'
            }`}
            title="Pinned"
          >
            <Pin className="w-5 h-5 text-orange-500 fill-orange-500/20 pin-icon" />
          </button>

          <button
            onClick={() => {
              setCurrentTab('notes');
              setSelectedBinId(null);
            }}
            className={`w-9 h-9 flex items-center justify-center p-0 rounded-xl transition-colors duration-75 border shrink-0 cursor-pointer ${
              currentTab === 'notes'
                ? 'sidebar-item-active bg-[#383838] text-white border-gray-600/70 shadow-sm'
                : 'sidebar-item-idle border-transparent text-gray-400 hover:bg-[#2a2a2a] hover:text-white'
            }`}
            title="Clip Notes"
          >
            <StickyNote className="w-5 h-5 text-emerald-400" />
          </button>

          <button
            onClick={() => {
              setCurrentTab('sequential');
              setSelectedBinId(null);
            }}
            className={`w-9 h-9 flex items-center justify-center p-0 rounded-xl transition-colors duration-75 border shrink-0 cursor-pointer ${
              currentTab === 'sequential'
                ? 'sidebar-item-active bg-[#383838] text-white border-gray-600/70 shadow-sm'
                : 'sidebar-item-idle border-transparent text-gray-400 hover:bg-[#2a2a2a] hover:text-white'
            }`}
            title="Queue"
          >
            <ListOrdered className="w-5 h-5" />
          </button>

          {sortedBins.length > 0 && (
            <div className="w-full flex items-center justify-center py-1 shrink-0">
              <div className="w-8 border-t border-white/10 sidebar-divider" />
            </div>
          )}

          {sortedBins.slice(0, 4).map((b) => (
            <button
              key={b.id}
              onClick={() => {
                setCurrentTab('bin');
                setSelectedBinId(b.id);
              }}
              className={`w-9 h-9 flex items-center justify-center p-0 rounded-xl transition-colors duration-75 border shrink-0 cursor-pointer ${
                currentTab === 'bin' && selectedBinId === b.id
                  ? 'sidebar-item-active bg-[#383838] text-white border-gray-600/70 shadow-sm'
                  : 'sidebar-item-idle border-transparent text-gray-400 hover:bg-[#2a2a2a] hover:text-white'
              }`}
              title={b.name}
            >
              {getBinIcon(b.icon)}
            </button>
          ))}

          <div className="w-full flex items-center justify-center py-1 shrink-0">
            <div className="w-8 border-t border-white/10 sidebar-divider" />
          </div>

          <button
            onClick={() => {
              setCurrentTab('filters');
              setSelectedBinId(null);
            }}
            className={`w-9 h-9 flex items-center justify-center p-0 rounded-xl transition-colors duration-75 border shrink-0 cursor-pointer ${
              currentTab === 'filters'
                ? 'sidebar-item-active bg-[#383838] text-white border-gray-600/70 shadow-sm'
                : 'sidebar-item-idle border-transparent text-gray-400 hover:bg-[#2a2a2a] hover:text-white'
            }`}
            title="Filters & Operations"
          >
            <Sliders className="w-5 h-5" />
          </button>

          <button
            onClick={() => {
              setCurrentTab('settings');
              setSelectedBinId(null);
            }}
            className={`w-9 h-9 flex items-center justify-center p-0 rounded-xl transition-colors duration-75 border shrink-0 cursor-pointer ${
              currentTab === 'settings'
                ? 'sidebar-item-active bg-[#383838] text-white border-gray-600/70 shadow-sm'
                : 'sidebar-item-idle border-transparent text-gray-400 hover:bg-[#2a2a2a] hover:text-white'
            }`}
            title="Settings"
          >
            <Settings className="w-5 h-5" />
          </button>
        </div>
      </aside>
    );
  }

  return (
    <aside
      style={{ width: `${sidebarWidth}px` }}
      className="col-sidebar shrink-0 h-screen flex flex-col justify-between bg-[#212121]/90 backdrop-blur-xl select-none"
    >
      {/* Finder-esque Liquid Glass 60px Top Header */}
      <div
        onMouseDown={startWindowDrag}
        className="h-[60px] px-4 flex items-center justify-between border-b border-transparent cursor-default titlebar-drag-handle shrink-0"
      >
        <div className="flex items-center pl-20 titlebar-drag-handle" />
        <button
          onClick={() => setIsCollapsed(true)}
          className="p-1.5 text-gray-400 hover:text-white rounded-lg hover:bg-[#2c2c2c] transition-colors titlebar-no-drag cursor-pointer"
          title="Collapse Sidebar (⌘\)"
        >
          <PanelLeftClose className="w-4 h-4 text-gray-300" />
        </button>
      </div>

      {/* Sidebar Navigation Content (Scrollable) */}
      <div className="flex-1 overflow-y-auto sidebar-scroll-container px-2.5 py-2 space-y-3 text-[13px]">
        {/* Section 1: Clips */}
        <div>
          <div
            onClick={() => setIsClipsOpen(!isClipsOpen)}
            className="px-2.5 pb-1 flex items-center justify-between cursor-pointer select-none group"
            title="Click to toggle section"
          >
            <span className="text-[11px] font-semibold text-gray-400/90 group-hover:text-gray-200 transition-colors tracking-tight">
              Clips
            </span>
          </div>
          <div
            className={`transition-all duration-150 ease-in-out ${
              isClipsOpen ? 'max-h-96 opacity-100 mt-0 overflow-visible' : 'max-h-0 opacity-0 overflow-hidden'
            }`}
          >
            <nav className="space-y-0.5">
              <button
                onClick={() => {
                  setCurrentTab('all');
                  setSelectedBinId(null);
                }}
                className={`group w-full h-8 flex items-center justify-between px-2.5 rounded-md transition-colors duration-100 cursor-pointer ${
                  currentTab === 'all' && selectedBinId === null
                    ? 'sidebar-item-active bg-[#3b3b3e] text-white font-medium'
                    : 'sidebar-item-idle text-[#e3e3e5] hover:bg-white/5 font-normal'
                }`}
              >
                <div className="flex items-center space-x-3">
                  <Clipboard className="w-4 h-4 text-[#0a84ff] shrink-0" strokeWidth={1.8} />
                  <span className="truncate">All</span>
                </div>
                <span className="sidebar-badge text-[11px] px-1.5 py-0.5 rounded-md bg-white/10 text-gray-300 font-mono">
                  {totalClipCount}
                </span>
              </button>

              <button
                onClick={() => {
                  setCurrentTab('sequential');
                  setSelectedBinId(null);
                }}
                className={`group w-full h-8 flex items-center justify-between px-2.5 rounded-md transition-colors duration-100 cursor-pointer ${
                  currentTab === 'sequential'
                    ? 'sidebar-item-active bg-[#3b3b3e] text-white font-medium'
                    : 'sidebar-item-idle text-[#e3e3e5] hover:bg-white/5 font-normal'
                }`}
              >
                <div className="flex items-center space-x-3">
                  <ListOrdered className="w-4 h-4 text-purple-400 shrink-0" strokeWidth={1.8} />
                  <span className="truncate">Queue</span>
                </div>
                {seqStatus?.is_active && (
                  <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
                )}
              </button>

              <button
                onClick={() => {
                  setCurrentTab('pinned');
                  setSelectedBinId(null);
                }}
                className={`group w-full h-8 flex items-center justify-between px-2.5 rounded-md transition-colors duration-100 cursor-pointer ${
                  currentTab === 'pinned'
                    ? 'sidebar-item-active bg-[#3b3b3e] text-white font-medium'
                    : 'sidebar-item-idle text-[#e3e3e5] hover:bg-white/5 font-normal'
                }`}
              >
                <div className="flex items-center space-x-3">
                  <Pin className="w-4 h-4 text-orange-500 fill-orange-500/20 pin-icon shrink-0" strokeWidth={1.8} />
                  <span className="truncate">Pinned</span>
                </div>
                {!!pinnedCount && pinnedCount > 0 && (
                  <span className="sidebar-badge text-[11px] px-1.5 py-0.5 rounded-md bg-white/10 text-gray-300 font-mono">
                    {pinnedCount}
                  </span>
                )}
              </button>

              <button
                onClick={() => {
                  setCurrentTab('protected');
                  setSelectedBinId(null);
                }}
                className={`group w-full h-8 flex items-center justify-between px-2.5 rounded-md transition-colors duration-100 cursor-pointer ${
                  currentTab === 'protected'
                    ? 'sidebar-item-active bg-[#3b3b3e] text-white font-medium'
                    : 'sidebar-item-idle text-[#e3e3e5] hover:bg-white/5 font-normal'
                }`}
              >
                <div className="flex items-center space-x-3">
                  <Shield className="w-4 h-4 text-cyan-400 shrink-0" strokeWidth={1.8} />
                  <span className="truncate">Protected</span>
                </div>
                {!!protectedCount && protectedCount > 0 && (
                  <span className="sidebar-badge text-[11px] px-1.5 py-0.5 rounded-md bg-white/10 text-gray-300 font-mono">
                    {protectedCount}
                  </span>
                )}
              </button>

              <button
                onClick={() => {
                  setCurrentTab('notes');
                  setSelectedBinId(null);
                }}
                className={`group w-full h-8 flex items-center justify-between px-2.5 rounded-md transition-colors duration-100 cursor-pointer ${
                  currentTab === 'notes'
                    ? 'sidebar-item-active bg-[#3b3b3e] text-white font-medium'
                    : 'sidebar-item-idle text-[#e3e3e5] hover:bg-white/5 font-normal'
                }`}
              >
                <div className="flex items-center space-x-3">
                  <StickyNote className="w-4 h-4 text-emerald-400 shrink-0" strokeWidth={1.8} />
                  <span className="truncate">Noted</span>
                </div>
                {!!notesCount && notesCount > 0 && (
                  <span className="sidebar-badge text-[11px] px-1.5 py-0.5 rounded-md bg-white/10 text-gray-300 font-mono">
                    {notesCount}
                  </span>
                )}
              </button>

              <button
                onClick={() => {
                  setCurrentTab('trash');
                  setSelectedBinId(null);
                }}
                className={`group w-full h-8 flex items-center justify-between px-2.5 rounded-md transition-colors duration-100 cursor-pointer ${
                  currentTab === 'trash'
                    ? 'sidebar-item-active bg-[#3b3b3e] text-white font-medium'
                    : 'sidebar-item-idle text-[#e3e3e5] hover:bg-white/5 font-normal'
                }`}
              >
                <div className="flex items-center space-x-3">
                  <Trash2 className="w-4 h-4 text-rose-400 shrink-0" strokeWidth={1.8} />
                  <span className="truncate">Trashed</span>
                </div>
                {!!trashedCount && trashedCount > 0 && (
                  <span className="sidebar-badge text-[11px] px-1.5 py-0.5 rounded-md bg-rose-500/20 text-rose-300 font-mono">
                    {trashedCount}
                  </span>
                )}
              </button>
            </nav>
          </div>
        </div>

        {/* Section 2: Bins */}
        <div>
          <div
            onClick={() => setIsBinsOpen(!isBinsOpen)}
            className="px-2.5 pb-1 flex items-center justify-between cursor-pointer select-none group"
            title="Click to toggle section"
          >
            <span className="text-[11px] font-semibold text-gray-400/90 group-hover:text-gray-200 transition-colors tracking-tight">
              Bins
            </span>
            <button
              onClick={(e) => {
                e.stopPropagation();
                onOpenNewBinModal();
              }}
              className="sidebar-add-btn text-gray-400 hover:text-white p-0.5 rounded transition-colors cursor-pointer"
              title="Create Custom / Smart Bin"
            >
              <Plus className="w-3.5 h-3.5" strokeWidth={2} />
            </button>
          </div>
          <div
            className={`transition-all duration-150 ease-in-out ${
              isBinsOpen ? 'max-h-[500px] opacity-100 mt-0 overflow-visible' : 'max-h-0 opacity-0 overflow-hidden'
            }`}
          >
            <nav
              className="space-y-0.5"
              onPointerUp={handlePointerUpBin}
              onPointerLeave={handlePointerUpBin}
            >
              {sortedBins.map((b) => {
                const isDragging = activeDragBinId === b.id;
                const isManualBin = !b.smart_rule || b.smart_rule.trim() === '';
                const isClipDragging = draggedClipId !== null && draggedClipId !== undefined;
                const isDisabledDropTarget =
                  isClipDragging && disabledDropBinId === b.id;
                const isIneligibleSmartBin = isClipDragging && !isManualBin;
                const isDropTarget =
                  (dropTargetBinId === b.id || pointerDropTargetBinId === b.id) &&
                  isManualBin &&
                  !isDisabledDropTarget;

                return (
                  <div
                    key={b.id}
                    data-bin-drop-bin-id={isManualBin && !isDisabledDropTarget ? b.id : undefined}
                    role="button"
                    tabIndex={0}
                    title={
                      isDisabledDropTarget
                        ? 'Already assigned to this Bin'
                        : isIneligibleSmartBin
                        ? 'Smart Bin — populated automatically by rules'
                        : undefined
                    }
                    onPointerDown={() => handlePointerDownBin(b.id)}
                    onPointerEnter={() => handlePointerEnterBin(b.id)}
                    onPointerUp={handlePointerUpBin}
                    onDragOver={(e) => {
                      if (!isManualBin || isDisabledDropTarget) return;
                      e.preventDefault();
                      e.stopPropagation();
                      e.dataTransfer.dropEffect = 'copy';
                      if (dropTargetBinId !== b.id) {
                        setDropTargetBinId(b.id);
                      }
                    }}
                    onDragEnter={(e) => {
                      if (!isManualBin || isDisabledDropTarget) return;
                      e.preventDefault();
                      e.stopPropagation();
                      e.dataTransfer.dropEffect = 'copy';
                      setDropTargetBinId(b.id);
                    }}
                    onDragLeave={(e) => {
                      if (!isManualBin || isDisabledDropTarget) return;
                      e.preventDefault();
                      if (e.relatedTarget && e.currentTarget.contains(e.relatedTarget as Node)) {
                        return;
                      }
                      const rect = e.currentTarget.getBoundingClientRect();
                      if (
                        e.clientX >= rect.left &&
                        e.clientX <= rect.right &&
                        e.clientY >= rect.top &&
                        e.clientY <= rect.bottom
                      ) {
                        return;
                      }
                      setDropTargetBinId((prev) => (prev === b.id ? null : prev));
                    }}
                    onDrop={(e) => {
                      if (!isManualBin || isDisabledDropTarget) return;
                      e.preventDefault();
                      e.stopPropagation();
                      setDropTargetBinId(null);
                      const rawClipId = e.dataTransfer.getData('clip_id');
                      const rawText = e.dataTransfer.getData('text/plain');
                      const parsedClip = parseInt(rawClipId, 10);
                      const parsedText = parseInt(rawText, 10);
                      const targetClipId =
                        !isNaN(parsedClip) && parsedClip > 0
                          ? parsedClip
                          : !isNaN(parsedText) && parsedText > 0
                          ? parsedText
                          : draggedClipId;

                      if (targetClipId && onClipDropOnBin) {
                        onClipDropOnBin(targetClipId, b.id);
                      }
                    }}
                    onClick={() => {
                      if (dragTimerRef.current) {
                        clearTimeout(dragTimerRef.current);
                        dragTimerRef.current = null;
                      }
                      if (!activeDragBinId) {
                        setCurrentTab('bin');
                        setSelectedBinId(b.id);
                      }
                    }}
                    onContextMenu={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      if (onBinContextMenu) onBinContextMenu(e.clientX, e.clientY, b);
                    }}
                    className={`group w-full h-8 flex items-center justify-between px-2.5 rounded-md select-none transition-all duration-100 ${
                      isDisabledDropTarget || isIneligibleSmartBin
                        ? 'cursor-not-allowed'
                        : isClipDragging
                        ? 'cursor-grabbing'
                        : 'cursor-pointer active:cursor-grabbing'
                    } ${
                      isDropTarget
                        ? 'bg-emerald-500/15 border border-emerald-400/80 ring-2 ring-emerald-400/25 shadow-lg text-emerald-50 z-30 relative'
                        : isDisabledDropTarget || isIneligibleSmartBin
                        ? 'bg-white/[0.025] border border-white/5 text-gray-600 opacity-50 cursor-not-allowed'
                        : isClipDragging && isManualBin
                        ? 'bg-emerald-950/15 border border-dashed border-emerald-500/45 text-emerald-100 font-normal'
                        : isDragging
                        ? 'bg-[#0a84ff]/30 shadow-md ring-1 ring-inset ring-[#0a84ff]/70 rounded-md z-20 relative'
                        : currentTab === 'bin' && selectedBinId === b.id
                        ? 'sidebar-item-active bg-[#3b3b3e] text-white font-medium'
                        : 'sidebar-item-idle text-[#e3e3e5] hover:bg-white/5 font-normal'
                    }`}
                  >
                    <div className="flex items-center space-x-2.5 truncate pr-1">
                      <span className="shrink-0 text-[#0a84ff]">{getBinIcon(b.icon)}</span>
                      <span className="truncate">{b.name}</span>
                    </div>

                    {/* Right side container */}
                    <div className="flex items-center justify-end shrink-0 pl-1">
                      <div className={`flex items-center space-x-1.5 ${isClipDragging || isDragging ? '' : 'group-hover:hidden'}`}>
                        {b.smart_rule ? (
                          <span
                            title={`Smart Bin Rule Active (${b.clip_count ?? 0} matching clips)`}
                            className="sidebar-badge text-[11px] px-1.5 py-0.5 rounded-md bg-white/10 text-gray-300 font-mono flex items-center space-x-1"
                          >
                            <Sparkles className="w-3 h-3 text-amber-400 shrink-0" />
                            <span>{b.clip_count ?? 0}</span>
                          </span>
                        ) : (
                          !!b.clip_count && b.clip_count > 0 && (
                            <span className="sidebar-badge text-[11px] px-1.5 py-0.5 rounded-md bg-white/10 text-gray-300 font-mono">
                              {b.clip_count}
                            </span>
                          )
                        )}
                      </div>

                      {/* Hover State: Edit & Trash action buttons (hidden when dragging) */}
                      <div className={`${isClipDragging || isDragging ? 'hidden' : 'hidden group-hover:flex'} items-center space-x-1`}>
                        <button
                          type="button"
                          onPointerDown={(e) => e.stopPropagation()}
                          onMouseDown={(e) => e.stopPropagation()}
                          onClick={(e) => {
                            e.stopPropagation();
                            if (onEditBin) onEditBin(b);
                          }}
                          className="p-1 text-gray-400 hover:text-blue-400 hover:bg-white/10 rounded transition-colors cursor-pointer"
                          title="Edit Bin"
                        >
                          <Edit3 className="w-3.5 h-3.5" />
                        </button>
                        <button
                          type="button"
                          onPointerDown={(e) => e.stopPropagation()}
                          onMouseDown={(e) => e.stopPropagation()}
                          onClick={(e) => {
                            e.stopPropagation();
                            if (onDeleteBin) onDeleteBin(b);
                          }}
                          className="p-1 text-gray-400 hover:text-red-400 hover:bg-white/10 rounded transition-colors cursor-pointer"
                          title="Delete Bin"
                        >
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

        {/* Section 3: Tools */}
        <div>
          <div
            onClick={() => setIsToolsOpen(!isToolsOpen)}
            className="px-2.5 pb-1 flex items-center justify-between cursor-pointer select-none group"
            title="Click to toggle section"
          >
            <span className="text-[11px] font-semibold text-gray-400/90 group-hover:text-gray-200 transition-colors tracking-tight">
              Tools
            </span>
          </div>
          <div
            className={`transition-all duration-150 ease-in-out ${
              isToolsOpen ? 'max-h-96 opacity-100 mt-0 overflow-visible' : 'max-h-0 opacity-0 overflow-hidden'
            }`}
          >
            <nav className="space-y-0.5">
              <button
                onClick={() => {
                  setCurrentTab('analytics');
                  setSelectedBinId(null);
                }}
                className={`group w-full h-8 flex items-center space-x-3 px-2.5 rounded-md transition-colors duration-100 cursor-pointer ${
                  currentTab === 'analytics'
                    ? 'sidebar-item-active bg-[#3b3b3e] text-white font-medium'
                    : 'sidebar-item-idle text-[#e3e3e5] hover:bg-white/5 font-normal'
                }`}
              >
                <BarChart3 className="w-4 h-4 text-purple-400 shrink-0" strokeWidth={1.8} />
                <span className="truncate">Analytics & Insights</span>
              </button>

              <button
                onClick={() => {
                  setCurrentTab('filters');
                  setSelectedBinId(null);
                }}
                className={`group w-full h-8 flex items-center space-x-3 px-2.5 rounded-md transition-colors duration-100 cursor-pointer ${
                  currentTab === 'filters'
                    ? 'sidebar-item-active bg-[#3b3b3e] text-white font-medium'
                    : 'sidebar-item-idle text-[#e3e3e5] hover:bg-white/5 font-normal'
                }`}
              >
                <Sliders className="w-4 h-4 text-[#0a84ff] shrink-0" strokeWidth={1.8} />
                <span className="truncate">Filters & Operations</span>
              </button>

              <button
                onClick={() => {
                  setCurrentTab('activity');
                  setSelectedBinId(null);
                }}
                className={`group w-full h-8 flex items-center space-x-3 px-2.5 rounded-md transition-colors duration-100 cursor-pointer ${
                  currentTab === 'activity'
                    ? 'sidebar-item-active bg-[#3b3b3e] text-white font-medium'
                    : 'sidebar-item-idle text-[#e3e3e5] hover:bg-white/5 font-normal'
                }`}
              >
                <Activity className="w-4 h-4 text-cyan-400 shrink-0" strokeWidth={1.8} />
                <span className="truncate">Activity Log</span>
              </button>

              <button
                onClick={() => {
                  setCurrentTab('help');
                  setSelectedBinId(null);
                }}
                className={`group w-full h-8 flex items-center space-x-3 px-2.5 rounded-md transition-colors duration-100 cursor-pointer ${
                  currentTab === 'help'
                    ? 'sidebar-item-active bg-[#3b3b3e] text-white font-medium'
                    : 'sidebar-item-idle text-[#e3e3e5] hover:bg-white/5 font-normal'
                }`}
              >
                <HelpCircle className="w-4 h-4 text-cyan-400 shrink-0" strokeWidth={1.8} />
                <span className="truncate">Help & Documentation</span>
              </button>

              <button
                onClick={() => {
                  setCurrentTab('settings');
                  setSelectedBinId(null);
                }}
                className={`group w-full h-8 flex items-center space-x-3 px-2.5 rounded-md transition-colors duration-100 cursor-pointer ${
                  currentTab === 'settings'
                    ? 'sidebar-item-active bg-[#3b3b3e] text-white font-medium'
                    : 'sidebar-item-idle text-[#e3e3e5] hover:bg-white/5 font-normal'
                }`}
              >
                <Settings className="w-4 h-4 text-[#0a84ff] shrink-0" strokeWidth={1.8} />
                <span className="truncate">Settings</span>
              </button>
            </nav>
          </div>
        </div>
      </div>

      {/* Pinned Bottom Search Bar Footer */}
      <div className="p-2.5 border-t border-white/10 shrink-0 relative">
        {isSearchFocused && !searchQuery.includes(':') && (
          <div className="absolute bottom-11 left-2.5 right-2.5 bg-[#1c1e26]/95 backdrop-blur-xl border border-cyan-500/40 rounded-xl p-1.5 shadow-2xl z-50 text-xs space-y-0.5 animate-in fade-in slide-in-from-bottom-2 duration-150">
            {[
              { prefix: 'regex:', desc: 'Regex' },
              { prefix: 'app:', desc: 'App' },
              { prefix: 'type:', desc: 'Type' },
              { prefix: 'has:note', desc: 'Notes' },
              { prefix: 'is:pinned', desc: 'Pinned' },
              { prefix: 'is:protected', desc: 'Protected' },
            ].map((s) => (
              <div
                key={s.prefix}
                onMouseDown={(e) => {
                  e.preventDefault();
                  setSearchQuery(s.prefix);
                }}
                className="px-2 py-1 rounded-lg hover:bg-cyan-950/80 hover:text-cyan-300 cursor-pointer flex items-center justify-between transition-colors"
              >
                <span className="font-mono font-bold text-cyan-400 text-[11px]">{s.prefix}</span>
                <span className="text-[10px] text-gray-400 font-medium">{s.desc}</span>
              </div>
            ))}
          </div>
        )}

        <div className="relative titlebar-no-drag">
          <Search className="w-3.5 h-3.5 absolute left-3 top-2 text-gray-400/80" />
          <input
            type="text"
            autoComplete="off"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck={false}
            placeholder="Search (try regex: app: type:)..."
            value={searchQuery}
            onFocus={() => setIsSearchFocused(true)}
            onBlur={() => setIsSearchFocused(false)}
            onKeyDown={(e) => {
              if (e.key === 'Escape') {
                setIsSearchFocused(false);
                (e.target as HTMLInputElement).blur();
              }
            }}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full h-7 bg-[#2c2c2e]/80 border border-white/10 rounded-md pl-8 pr-2.5 text-[12px] text-gray-200 placeholder-gray-400/60 focus:outline-none focus:border-[#0a84ff]/80 transition-all titlebar-no-drag"
          />
        </div>
      </div>
    </aside>
  );
};
