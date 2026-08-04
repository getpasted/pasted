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
import { useSidebarBinOrder } from '../hooks/useSidebarBinOrder';

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

  const [dropTargetBinId, setDropTargetBinId] = React.useState<number | null>(null);
  const [isSearchFocused, setIsSearchFocused] = React.useState(false);
  const [isPostDragHoverSuppressed, setIsPostDragHoverSuppressed] = React.useState(false);
  const [hoveredSidebarControl, setHoveredSidebarControl] = React.useState<string | null>(null);
  const wasClipDraggingRef = React.useRef(false);
  const isPointerOverSidebarRef = React.useRef(false);
  const isClipDragging = draggedClipId !== null && draggedClipId !== undefined;
  const {
    activeDragBinId,
    sortedBins,
    binListRef,
    binReorderOffsets,
    isBinReorderSettling,
    isBinReorderActive,
    startBinDrag: handlePointerDownBin,
    consumeBinDragClick,
  } = useSidebarBinOrder(bins, isClipDragging);
  const isAnySidebarDrag = isClipDragging || isBinReorderActive;
  const isSidebarHoverMuted = isAnySidebarDrag || isPostDragHoverSuppressed;

  React.useLayoutEffect(() => {
    if (isAnySidebarDrag) setHoveredSidebarControl(null);
    if (wasClipDraggingRef.current && !isAnySidebarDrag) {
      setIsPostDragHoverSuppressed(isPointerOverSidebarRef.current);
    }
    wasClipDraggingRef.current = isAnySidebarDrag;
  }, [isAnySidebarDrag]);

  const handleSidebarPointerEnter = () => {
    isPointerOverSidebarRef.current = true;
  };

  const handleSidebarPointerMove = (event: React.PointerEvent<HTMLElement>) => {
    if (isSidebarHoverMuted) {
      if (hoveredSidebarControl !== null) setHoveredSidebarControl(null);
      return;
    }
    const control = (event.target as HTMLElement).closest<HTMLElement>('[data-sidebar-hover-key]');
    const nextKey = control && event.currentTarget.contains(control)
      ? control.dataset.sidebarHoverKey ?? null
      : null;
    if (nextKey !== hoveredSidebarControl) setHoveredSidebarControl(nextKey);
  };

  const handleSidebarPointerLeave = () => {
    isPointerOverSidebarRef.current = false;
    setHoveredSidebarControl(null);
    if (!isAnySidebarDrag) setIsPostDragHoverSuppressed(false);
  };

  const getBinIcon = (iconName: string) => {
    return <span className="text-sm">{formatEmojiIcon(iconName)}</span>;
  };

  const navigateTo = (tab: string) => {
    setCurrentTab(tab);
    setSelectedBinId(null);
  };

  const clipNavItems = [
    { tab: 'all', label: 'All', title: 'All Clips', icon: <Clipboard className="sidebar-icon-primary w-5 h-5" /> },
    { tab: 'sequential', label: 'Queue', title: 'Queue', icon: <ListOrdered className="sidebar-icon-secondary w-5 h-5" /> },
    { tab: 'pinned', label: 'Pinned', title: 'Pinned', icon: <Pin className="sidebar-icon-warning w-5 h-5 pin-icon" /> },
    { tab: 'protected', label: 'Protected', title: 'Protected', icon: <Shield className="sidebar-icon-info w-5 h-5" /> },
    { tab: 'notes', label: 'Noted', title: 'Noted', icon: <StickyNote className="sidebar-icon-success w-5 h-5" /> },
    { tab: 'trash', label: 'Trashed', title: 'Trashed', icon: <Trash2 className="sidebar-icon-danger w-5 h-5" /> },
  ];
  const toolNavItems = [
    { tab: 'analytics', label: 'Analytics & Insights', title: 'Analytics & Insights', icon: <BarChart3 className="sidebar-icon-secondary w-5 h-5" /> },
    { tab: 'filters', label: 'Filters & Operations', title: 'Filters & Operations', icon: <Sliders className="sidebar-icon-primary w-5 h-5" /> },
    { tab: 'activity', label: 'Activity Log', title: 'Activity Log', icon: <Activity className="sidebar-icon-info w-5 h-5" /> },
    { tab: 'help', label: 'Help & Documentation', title: 'Help & Documentation', icon: <HelpCircle className="sidebar-icon-info w-5 h-5" /> },
    { tab: 'settings', label: 'Settings', title: 'Settings', icon: <Settings className="sidebar-icon-primary w-5 h-5" /> },
  ];

  const clipCountByTab: Record<string, number> = {
    all: totalClipCount,
    pinned: pinnedCount,
    protected: protectedCount,
    notes: notesCount,
    trash: trashedCount,
  };

  if (isCollapsed) {
    return (
      <aside
        onPointerEnter={handleSidebarPointerEnter}
        onPointerMove={handleSidebarPointerMove}
        onPointerLeave={handleSidebarPointerLeave}
        className={`w-[100px] col-sidebar h-screen flex flex-col items-center border-r backdrop-blur-xl select-none ${isSidebarHoverMuted ? 'suppress-sidebar-hover' : ''}`}
      >
        {/* Dedicated 56px Top Header Drag Region for Traffic Lights */}
        <div
          onMouseDown={startWindowDrag}
          className="h-[56px] w-full cursor-default titlebar-drag-handle shrink-0"
        />

        {/* Scrollable Nav Items Container for small window heights */}
        <div className="w-full flex-1 overflow-y-auto overflow-x-hidden sidebar-scroll-container flex flex-col items-center gap-1.5 py-2 px-1 custom-scrollbar">
          {/* Expand Sidebar Toggle Button (Safely placed below traffic light zone) */}
          <button
            data-sidebar-hover-key="expand"
            onClick={() => setIsCollapsed(false)}
            disabled={isClipDragging}
            className={`sidebar-control-muted w-9 h-9 flex items-center justify-center p-0 rounded-xl transition-colors duration-75 border shrink-0 ${isClipDragging ? 'border-transparent cursor-default' : `cursor-pointer ${hoveredSidebarControl === 'expand' ? 'sidebar-item-hovered' : 'border-transparent'}`}`}
            title="Expand Sidebar (⌘\)"
          >
            <PanelLeftOpen className="w-5 h-5" />
          </button>

          <div className="w-full flex items-center justify-center py-1 shrink-0">
            <div className="w-8 border-t sidebar-divider" />
          </div>

          {clipNavItems.map((item) => (
            <button
              key={item.tab}
              data-sidebar-hover-key={`clip:${item.tab}`}
              onClick={() => navigateTo(item.tab)}
              disabled={isClipDragging}
              className={`w-9 h-9 flex items-center justify-center p-0 rounded-xl transition-colors duration-75 border shrink-0 cursor-pointer ${
                currentTab === item.tab && (item.tab !== 'all' || selectedBinId === null)
                  ? 'sidebar-item-active shadow-sm'
                  : hoveredSidebarControl === `clip:${item.tab}`
                  ? 'sidebar-item-hovered border-transparent'
                  : 'sidebar-item-idle border-transparent'
              }`}
              title={item.title}
            >
              {item.icon}
            </button>
          ))}

          {sortedBins.length > 0 && (
            <div className="w-full flex items-center justify-center py-1 shrink-0">
              <div className="w-8 border-t sidebar-divider" />
            </div>
          )}

          {sortedBins.map((b) => (
            <button
              key={b.id}
              data-sidebar-hover-key={`bin:${b.id}`}
              onClick={() => {
                setCurrentTab('bin');
                setSelectedBinId(b.id);
              }}
              disabled={isClipDragging}
              className={`w-9 h-9 flex items-center justify-center p-0 rounded-xl transition-colors duration-75 border shrink-0 cursor-pointer ${
                currentTab === 'bin' && selectedBinId === b.id
                  ? 'sidebar-item-active shadow-sm'
                  : hoveredSidebarControl === `bin:${b.id}`
                  ? 'sidebar-item-hovered border-transparent'
                  : 'sidebar-item-idle border-transparent'
              }`}
              title={b.name}
            >
              {getBinIcon(b.icon)}
            </button>
          ))}

          <div className="w-full flex items-center justify-center py-1 shrink-0">
            <div className="w-8 border-t sidebar-divider" />
          </div>

          {toolNavItems.map((item) => (
            <button
              key={item.tab}
              data-sidebar-hover-key={`tool:${item.tab}`}
              onClick={() => navigateTo(item.tab)}
              disabled={isClipDragging}
              className={`w-9 h-9 flex items-center justify-center p-0 rounded-xl transition-colors duration-75 border shrink-0 cursor-pointer ${
                currentTab === item.tab
                  ? 'sidebar-item-active shadow-sm'
                  : hoveredSidebarControl === `tool:${item.tab}`
                  ? 'sidebar-item-hovered border-transparent'
                  : 'sidebar-item-idle border-transparent'
              }`}
              title={item.title}
            >
              {item.icon}
            </button>
          ))}
        </div>
      </aside>
    );
  }

  return (
    <aside
      style={{ width: `${sidebarWidth}px` }}
      onPointerEnter={handleSidebarPointerEnter}
      onPointerMove={handleSidebarPointerMove}
      onPointerLeave={handleSidebarPointerLeave}
      className={`col-sidebar shrink-0 h-screen flex flex-col justify-between backdrop-blur-xl select-none ${isSidebarHoverMuted ? 'suppress-sidebar-hover' : ''}`}
    >
      {/* Finder-esque Liquid Glass 60px Top Header */}
      <div
        onMouseDown={isClipDragging ? undefined : startWindowDrag}
        className="h-[60px] px-4 flex items-center justify-between border-b border-transparent cursor-default titlebar-drag-handle shrink-0"
      >
        <div className="flex items-center pl-20 titlebar-drag-handle" />
        <button
          data-sidebar-hover-key="collapse"
          onClick={() => setIsCollapsed(true)}
          disabled={isClipDragging}
          className={`sidebar-control-muted p-1.5 rounded-lg transition-colors titlebar-no-drag ${isClipDragging ? 'cursor-default' : `cursor-pointer ${hoveredSidebarControl === 'collapse' ? 'sidebar-item-hovered' : ''}`}`}
          title="Collapse Sidebar (⌘\)"
        >
          <PanelLeftClose className="w-4 h-4" />
        </button>
      </div>

      {/* Sidebar Navigation Content (Scrollable) */}
      <div className="flex-1 overflow-y-auto sidebar-scroll-container px-2.5 py-2 space-y-3 text-[13px]">
        {/* Section 1: Clips */}
        <div>
          <div
            data-sidebar-hover-key="section:clips"
            onClick={isClipDragging ? undefined : () => setIsClipsOpen(!isClipsOpen)}
            className={`px-2.5 pb-1 flex items-center justify-between select-none ${isClipDragging ? 'cursor-default' : 'cursor-pointer'}`}
            title="Click to toggle section"
          >
            <span className={`sidebar-section-label text-[11px] font-semibold transition-colors tracking-tight ${hoveredSidebarControl === 'section:clips' ? 'is-hovered' : ''}`}>
              Clips
            </span>
          </div>
          <div
            className={`transition-[background-color,border-color,color,opacity,transform] duration-150 ease-in-out ${
              isClipsOpen ? 'max-h-96 opacity-100 mt-0 overflow-visible' : 'max-h-0 opacity-0 overflow-hidden'
            }`}
          >
            <nav className="space-y-0.5">
              {clipNavItems.map((item) => {
                const count = clipCountByTab[item.tab];
                return (
                  <button
                    key={item.tab}
                    data-sidebar-hover-key={`clip:${item.tab}`}
                    onClick={() => navigateTo(item.tab)}
                    disabled={isClipDragging}
                    className={`sidebar-nav-row justify-between transition-colors duration-100 cursor-pointer ${
                      currentTab === item.tab && (item.tab !== 'all' || selectedBinId === null)
                        ? 'sidebar-item-active font-medium'
                        : hoveredSidebarControl === `clip:${item.tab}`
                        ? 'sidebar-item-hovered font-normal'
                        : 'sidebar-item-idle font-normal'
                    }`}
                  >
                    <div className="flex items-center gap-3 min-w-0">
                      <span className="sidebar-nav-icon">
                        {React.cloneElement(item.icon, { className: item.icon.props.className.replace('w-5 h-5', 'w-4 h-4 shrink-0'), strokeWidth: 1.8 })}
                      </span>
                      <span className="truncate">{item.label}</span>
                    </div>
                    {item.tab === 'sequential' && seqStatus?.is_active ? (
                      <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
                    ) : (item.tab === 'all' || count > 0) ? (
                      <span className={`sidebar-badge text-[11px] px-1.5 py-0.5 rounded-md font-mono ${
                        item.tab === 'trash' ? 'is-danger' : ''
                      }`}>
                        {count}
                      </span>
                    ) : null}
                  </button>
                );
              })}
            </nav>
          </div>
        </div>

        {/* Section 2: Bins */}
        <div>
          <div
            data-sidebar-hover-key="section:bins"
            onClick={isClipDragging ? undefined : () => setIsBinsOpen(!isBinsOpen)}
            className={`px-2.5 pb-1 flex items-center justify-between select-none ${isClipDragging ? 'cursor-default' : 'cursor-pointer'}`}
            title="Click to toggle section"
          >
            <span className={`sidebar-section-label text-[11px] font-semibold transition-colors tracking-tight ${hoveredSidebarControl === 'section:bins' ? 'is-hovered' : ''}`}>
              Bins
            </span>
            <button
              data-sidebar-hover-key="create-bin"
              onClick={(e) => {
                e.stopPropagation();
                onOpenNewBinModal();
              }}
              disabled={isClipDragging}
              className={`sidebar-add-btn p-0.5 rounded transition-colors ${isClipDragging ? 'cursor-default' : `cursor-pointer ${hoveredSidebarControl === 'create-bin' ? 'is-hovered' : ''}`}`}
              title="Create Custom / Smart Bin"
            >
              <Plus className="w-3.5 h-3.5" strokeWidth={2} />
            </button>
          </div>
          <div
            className={`transition-[background-color,border-color,color,opacity,transform] duration-150 ease-in-out ${
              isBinsOpen ? 'max-h-[500px] opacity-100 mt-0 overflow-visible' : 'max-h-0 opacity-0 overflow-hidden'
            }`}
          >
            <nav
              ref={binListRef}
              className={`space-y-0.5 ${isBinReorderSettling ? 'is-settling-stable-reorder' : ''}`}
            >
              {sortedBins.map((b) => {
                const isDragging = activeDragBinId === b.id;
                const isManualBin = !b.smart_rule || b.smart_rule.trim() === '';
                const isDisabledDropTarget =
                  isClipDragging && disabledDropBinId === b.id;
                const isIneligibleSmartBin = isClipDragging && !isManualBin;
                const isDropTarget =
                  (dropTargetBinId === b.id || pointerDropTargetBinId === b.id) &&
                  isManualBin &&
                  !isDisabledDropTarget;
                const isBinHovered = !isSidebarHoverMuted && hoveredSidebarControl === `bin:${b.id}`;

                return (
                  <div
                    key={b.id}
                    data-stable-reorder-id={String(b.id)}
                    style={binReorderOffsets[b.id] !== undefined ? {
                      transform: `translateY(${binReorderOffsets[b.id]}px)`,
                      zIndex: activeDragBinId === b.id ? 20 : 10,
                    } : undefined}
                    data-sidebar-hover-key={`bin:${b.id}`}
                    data-bin-drop-id={isManualBin && !isDisabledDropTarget ? b.id : undefined}
                    role="button"
                    tabIndex={0}
                    title={
                      isDisabledDropTarget
                        ? 'Already assigned to this Bin'
                        : isIneligibleSmartBin
                        ? 'Smart Bin — populated automatically by rules'
                        : undefined
                    }
                    onPointerDown={(event) => handlePointerDownBin(String(b.id), event)}
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
                      if (consumeBinDragClick()) return;
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
                    className={`sidebar-nav-row justify-between select-none transition-[background-color,border-color,color,box-shadow,opacity,transform] duration-100 ${
                      isDisabledDropTarget || isIneligibleSmartBin
                        ? 'cursor-not-allowed'
                        : isClipDragging
                        ? 'cursor-grabbing'
                        : 'cursor-pointer active:cursor-grabbing'
                    } ${
                      isDropTarget
                        ? 'sidebar-bin-drop-target ring-2 ring-emerald-400/25 shadow-lg text-emerald-50 relative'
                        : isDisabledDropTarget || isIneligibleSmartBin
                        ? 'sidebar-bin-ineligible opacity-50 cursor-not-allowed'
                        : isClipDragging && isManualBin
                        ? 'sidebar-bin-drop-eligible text-emerald-100 font-normal'
                        : isDragging
                        ? 'sidebar-bin-drag-source rounded-md relative pointer-events-none'
                        : currentTab === 'bin' && selectedBinId === b.id
                        ? 'sidebar-item-active font-medium'
                        : isBinHovered
                        ? 'sidebar-item-hovered font-normal'
                        : 'sidebar-item-idle font-normal'
                    }`}
                  >
                    <div className="flex items-center gap-3 truncate pr-1 min-w-0">
                      <span className="sidebar-nav-icon sidebar-nav-icon-emoji sidebar-icon-primary">{getBinIcon(b.icon)}</span>
                      <span className="truncate">{b.name}</span>
                    </div>

                    {/* Right side container */}
                    <div className="flex items-center justify-end shrink-0 pl-1">
                      <div className={`flex items-center space-x-1.5 ${isBinHovered && !isDragging ? 'hidden' : ''}`}>
                        {b.smart_rule ? (
                          <span
                            title={`Smart Bin Rule Active (${b.clip_count ?? 0} matching clips)`}
                            className="sidebar-badge text-[11px] px-1.5 py-0.5 rounded-md font-mono flex items-center space-x-1"
                          >
                            <Sparkles className="w-3 h-3 text-amber-400 shrink-0" />
                            <span>{b.clip_count ?? 0}</span>
                          </span>
                        ) : (
                          !!b.clip_count && b.clip_count > 0 && (
                            <span className="sidebar-badge text-[11px] px-1.5 py-0.5 rounded-md font-mono">
                              {b.clip_count}
                            </span>
                          )
                        )}
                      </div>

                      {/* Hover State: Edit & Trash action buttons (hidden when dragging) */}
                      <div className={`${isBinHovered && !isDragging ? 'flex' : 'hidden'} items-center space-x-1`}>
                        <button
                          type="button"
                          onPointerDown={(e) => e.stopPropagation()}
                          onMouseDown={(e) => e.stopPropagation()}
                          onClick={(e) => {
                            e.stopPropagation();
                            if (onEditBin) onEditBin(b);
                          }}
                          className="sidebar-row-action is-edit p-1 rounded transition-colors cursor-pointer"
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
                          className="sidebar-row-action is-danger p-1 rounded transition-colors cursor-pointer"
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
            data-sidebar-hover-key="section:tools"
            onClick={isClipDragging ? undefined : () => setIsToolsOpen(!isToolsOpen)}
            className={`px-2.5 pb-1 flex items-center justify-between select-none ${isClipDragging ? 'cursor-default' : 'cursor-pointer'}`}
            title="Click to toggle section"
          >
            <span className={`sidebar-section-label text-[11px] font-semibold transition-colors tracking-tight ${hoveredSidebarControl === 'section:tools' ? 'is-hovered' : ''}`}>
              Tools
            </span>
          </div>
          <div
            className={`transition-[background-color,border-color,color,opacity,transform] duration-150 ease-in-out ${
              isToolsOpen ? 'max-h-96 opacity-100 mt-0 overflow-visible' : 'max-h-0 opacity-0 overflow-hidden'
            }`}
          >
            <nav className="space-y-0.5">
              {toolNavItems.map((item) => (
                <button
                  key={item.tab}
                  data-sidebar-hover-key={`tool:${item.tab}`}
                  onClick={() => navigateTo(item.tab)}
                  disabled={isClipDragging}
                  className={`sidebar-nav-row gap-3 transition-colors duration-100 cursor-pointer ${
                    currentTab === item.tab
                      ? 'sidebar-item-active font-medium'
                      : hoveredSidebarControl === `tool:${item.tab}`
                      ? 'sidebar-item-hovered font-normal'
                      : 'sidebar-item-idle font-normal'
                  }`}
                >
                  <span className="sidebar-nav-icon">
                    {React.cloneElement(item.icon, { className: item.icon.props.className.replace('w-5 h-5', 'w-4 h-4 shrink-0'), strokeWidth: 1.8 })}
                  </span>
                  <span className="truncate">{item.label}</span>
                </button>
              ))}
            </nav>
          </div>
        </div>
      </div>

      {/* Pinned Bottom Search Bar Footer */}
      <div className="sidebar-divider p-2.5 border-t shrink-0 relative">
        {!isClipDragging && isSearchFocused && !searchQuery.includes(':') && (
          <div className="sidebar-search-helper absolute bottom-11 left-2.5 right-2.5 backdrop-blur-xl rounded-xl p-1.5 text-xs space-y-0.5 animate-in fade-in slide-in-from-bottom-2 duration-150">
            {[
              { prefix: 'regex:', desc: 'Regex' },
              { prefix: 'app:', desc: 'App' },
              { prefix: 'type:', desc: 'Type' },
              { prefix: 'has:note', desc: 'Notes' },
              { prefix: 'is:pinned', desc: 'Pinned' },
              { prefix: 'is:protected', desc: 'Protected' },
            ].map((s) => (
              <button
                type="button"
                key={s.prefix}
                onMouseDown={(e) => {
                  e.preventDefault();
                  setSearchQuery(s.prefix);
                }}
                className="sidebar-search-helper-option w-full px-2 py-1 rounded-lg cursor-pointer flex items-center justify-between transition-colors"
              >
                <span className="sidebar-search-helper-prefix font-mono font-bold text-[11px]">{s.prefix}</span>
                <span className="sidebar-search-helper-description text-[10px] font-medium">{s.desc}</span>
              </button>
            ))}
          </div>
        )}

        <div className="relative titlebar-no-drag">
          <Search className="sidebar-search-icon w-3.5 h-3.5 absolute left-3 top-2" />
          <input
            type="text"
            disabled={isClipDragging}
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
            className="sidebar-search-input theme-input w-full h-7 border rounded-md pl-8 pr-2.5 text-[12px] focus:outline-none transition-colors titlebar-no-drag"
          />
        </div>
      </div>
    </aside>
  );
};
