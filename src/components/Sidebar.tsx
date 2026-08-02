import React from 'react';
import { invoke } from '@tauri-apps/api/core';
import { formatEmojiIcon } from '../utils/emoji';
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
import { Board, SequentialStatus } from '../types';

interface SidebarProps {
  currentTab: string;
  setCurrentTab: (tab: string) => void;
  selectedBoardId: number | null;
  setSelectedBoardId: (id: number | null) => void;
  boards: Board[];
  onRefreshBoards: () => void;
  onOpenNewBoardModal: () => void;
  onEditBoard?: (board: Board) => void;
  onBoardContextMenu?: (x: number, y: number, board: Board) => void;
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
}

export const Sidebar: React.FC<SidebarProps> = ({
  currentTab,
  setCurrentTab,
  selectedBoardId,
  setSelectedBoardId,
  boards,
  onRefreshBoards,
  onOpenNewBoardModal,
  onEditBoard,
  onBoardContextMenu,
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

  // Board Drag & Drop Reorder State with 150ms Debounce
  const [activeDragBoardId, setActiveDragBoardId] = React.useState<number | null>(null);
  const dragTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);

  const [boardOrder, setBoardOrder] = React.useState<number[]>(() => {
    try {
      const saved = localStorage.getItem('pasted_board_order');
      return saved ? JSON.parse(saved) : [];
    } catch {
      return [];
    }
  });

  const sortedBoards = React.useMemo(() => {
    if (!boardOrder || boardOrder.length === 0) return boards;
    return [...boards].sort((a, b) => {
      const indexA = boardOrder.indexOf(a.id);
      const indexB = boardOrder.indexOf(b.id);
      if (indexA === -1 && indexB === -1) return 0;
      if (indexA === -1) return 1;
      if (indexB === -1) return -1;
      return indexA - indexB;
    });
  }, [boards, boardOrder]);

  const handlePointerDownBoard = (boardId: number) => {
    if (dragTimerRef.current) clearTimeout(dragTimerRef.current);
    dragTimerRef.current = setTimeout(() => {
      setActiveDragBoardId(boardId);
    }, 150);
  };

  const handlePointerUpBoard = () => {
    if (dragTimerRef.current) {
      clearTimeout(dragTimerRef.current);
      dragTimerRef.current = null;
    }
    setActiveDragBoardId(null);
  };

  const handlePointerEnterBoard = (targetBoardId: number) => {
    if (!activeDragBoardId || activeDragBoardId === targetBoardId) return;

    const currentOrder = sortedBoards.map((b) => b.id);
    const fromIndex = currentOrder.indexOf(activeDragBoardId);
    const toIndex = currentOrder.indexOf(targetBoardId);
    if (fromIndex === -1 || toIndex === -1) return;

    const newOrder = [...currentOrder];
    const [moved] = newOrder.splice(fromIndex, 1);
    newOrder.splice(toIndex, 0, moved);

    setBoardOrder(newOrder);
    localStorage.setItem('pasted_board_order', JSON.stringify(newOrder));
  };

  const getBoardIcon = (iconName: string) => {
    return <span className="text-sm">{formatEmojiIcon(iconName)}</span>;
  };

  if (isCollapsed) {
    return (
      <aside className="w-[100px] col-sidebar h-screen flex flex-col items-center border-r border-[#2d2d2d] bg-[#212121]/90 backdrop-blur-xl select-none">
        {/* Dedicated 56px Top Header Drag Region for Traffic Lights */}
        <div
          data-tauri-drag-region
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
              setSelectedBoardId(null);
            }}
            className={`w-9 h-9 flex items-center justify-center p-0 rounded-xl transition-colors duration-75 border shrink-0 cursor-pointer ${
              currentTab === 'all' && selectedBoardId === null
                ? 'sidebar-item-active bg-[#383838] text-white border-gray-600/70 shadow-sm'
                : 'sidebar-item-idle border-transparent text-gray-400 hover:bg-[#2a2a2a] hover:text-white'
            }`}
            title="All History"
          >
            <Clipboard className="w-5 h-5" />
          </button>

          <button
            onClick={() => {
              setCurrentTab('pinned');
              setSelectedBoardId(null);
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
              setSelectedBoardId(null);
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
              setSelectedBoardId(null);
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

          {sortedBoards.length > 0 && (
            <div className="w-full flex items-center justify-center py-1 shrink-0">
              <div className="w-8 border-t border-white/10 sidebar-divider" />
            </div>
          )}

          {sortedBoards.slice(0, 4).map((b) => (
            <button
              key={b.id}
              onClick={() => {
                setCurrentTab('board');
                setSelectedBoardId(b.id);
              }}
              className={`w-9 h-9 flex items-center justify-center p-0 rounded-xl transition-colors duration-75 border shrink-0 cursor-pointer ${
                currentTab === 'board' && selectedBoardId === b.id
                  ? 'sidebar-item-active bg-[#383838] text-white border-gray-600/70 shadow-sm'
                  : 'sidebar-item-idle border-transparent text-gray-400 hover:bg-[#2a2a2a] hover:text-white'
              }`}
              title={b.name}
            >
              {getBoardIcon(b.icon)}
            </button>
          ))}

          <div className="w-full flex items-center justify-center py-1 shrink-0">
            <div className="w-8 border-t border-white/10 sidebar-divider" />
          </div>

          <button
            onClick={() => {
              setCurrentTab('filters');
              setSelectedBoardId(null);
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
              setSelectedBoardId(null);
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
        data-tauri-drag-region
        className="h-[60px] px-4 flex items-center justify-between border-b border-transparent cursor-default titlebar-drag-handle shrink-0"
      >
        <div data-tauri-drag-region className="flex items-center pl-20 titlebar-drag-handle" />
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
                  setSelectedBoardId(null);
                }}
                className={`group w-full h-8 flex items-center justify-between px-2.5 rounded-md transition-colors duration-100 cursor-pointer ${
                  currentTab === 'all' && selectedBoardId === null
                    ? 'sidebar-item-active bg-[#3b3b3e] text-white font-medium'
                    : 'sidebar-item-idle text-[#e3e3e5] hover:bg-white/5 font-normal'
                }`}
              >
                <div className="flex items-center space-x-3">
                  <Clipboard className="w-4 h-4 text-[#0a84ff] shrink-0" strokeWidth={1.8} />
                  <span className="truncate">All History</span>
                </div>
                <span className="sidebar-badge text-[11px] px-1.5 py-0.5 rounded-md bg-white/10 text-gray-300 font-mono">
                  {totalClipCount}
                </span>
              </button>

              <button
                onClick={() => {
                  setCurrentTab('sequential');
                  setSelectedBoardId(null);
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
                  setSelectedBoardId(null);
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
                  setSelectedBoardId(null);
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
                  setSelectedBoardId(null);
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
                  setSelectedBoardId(null);
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
                onOpenNewBoardModal();
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
              onPointerUp={handlePointerUpBoard}
              onPointerLeave={handlePointerUpBoard}
            >
              {sortedBoards.map((b) => {
                const isDragging = activeDragBoardId === b.id;

                return (
                  <button
                    key={b.id}
                    type="button"
                    onPointerDown={() => handlePointerDownBoard(b.id)}
                    onPointerEnter={() => handlePointerEnterBoard(b.id)}
                    onPointerUp={handlePointerUpBoard}
                    onClick={() => {
                      if (dragTimerRef.current) {
                        clearTimeout(dragTimerRef.current);
                        dragTimerRef.current = null;
                      }
                      if (!activeDragBoardId) {
                        setCurrentTab('board');
                        setSelectedBoardId(b.id);
                      }
                    }}
                    onContextMenu={(e) => {
                      e.preventDefault();
                      if (onBoardContextMenu) onBoardContextMenu(e.clientX, e.clientY, b);
                    }}
                    className={`group w-full h-8 flex items-center justify-between px-2.5 rounded-md select-none transition-all duration-100 ${
                      sortedBoards.length > 1 ? 'cursor-grab active:cursor-grabbing' : 'cursor-pointer'
                    } ${
                      isDragging
                        ? 'bg-[#0a84ff]/30 shadow-md ring-1 ring-inset ring-[#0a84ff]/70 rounded-md z-20 relative'
                        : currentTab === 'board' && selectedBoardId === b.id
                        ? 'sidebar-item-active bg-[#3b3b3e] text-white font-medium'
                        : 'sidebar-item-idle text-[#e3e3e5] hover:bg-white/5 font-normal'
                    }`}
                  >
                    <div className="flex items-center space-x-2.5 truncate pr-1">
                      <span className="shrink-0 text-[#0a84ff]">{getBoardIcon(b.icon)}</span>
                      <span className="truncate">{b.name}</span>
                    </div>

                    {/* Right side container */}
                    <div className="flex items-center justify-end shrink-0 pl-1">
                      {/* Default State: Smart Rule Icon + Clip Count Badge */}
                      <div className={`flex items-center space-x-1.5 ${isDragging ? '' : 'group-hover:hidden'}`}>
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
                      <div className={`${isDragging ? 'hidden' : 'hidden group-hover:flex'} items-center space-x-1`}>
                        <button
                          type="button"
                          onPointerDown={(e) => e.stopPropagation()}
                          onMouseDown={(e) => e.stopPropagation()}
                          onClick={(e) => {
                            e.stopPropagation();
                            if (onEditBoard) onEditBoard(b);
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
                          onClick={async (e) => {
                            e.stopPropagation();
                            if (confirm(`Delete bin "${b.name}"?`)) {
                              try {
                                await invoke('delete_board', { id: b.id });
                                onRefreshBoards();
                                if (selectedBoardId === b.id) {
                                  setCurrentTab('all');
                                  setSelectedBoardId(null);
                                }
                              } catch (err) {
                                console.error(err);
                              }
                            }
                          }}
                          className="p-1 text-gray-400 hover:text-red-400 hover:bg-white/10 rounded transition-colors cursor-pointer"
                          title="Delete Bin"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    </div>
                  </button>
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
                  setSelectedBoardId(null);
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
                  setSelectedBoardId(null);
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
                  setSelectedBoardId(null);
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
                  setSelectedBoardId(null);
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
                  setSelectedBoardId(null);
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
      <div className="p-2.5 border-t border-white/10 shrink-0">
        <div className="relative titlebar-no-drag">
          <Search className="w-3.5 h-3.5 absolute left-3 top-2 text-gray-400/80" />
          <input
            type="text"
            placeholder="Search clipboard..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full h-7 bg-[#2c2c2e]/80 border border-white/10 rounded-md pl-8 pr-2.5 text-[12px] text-gray-200 placeholder-gray-400/60 focus:outline-none focus:border-[#0a84ff]/80 transition-all titlebar-no-drag"
          />
        </div>
      </div>
    </aside>
  );
};
