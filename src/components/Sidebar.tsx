import React from 'react';
import { formatEmojiIcon } from '../utils/emoji';
import { binTextColor } from '../utils/binColor';
import { startWindowDrag } from '../utils/windowDrag';
import {
  Clipboard,
  Pin,
  ListOrdered,
  Workflow,
  Settings,
  Trash2,
  Plus,
  ChevronUp,
  PanelLeftClose,
  PanelLeftOpen,
  Sparkles,
  Edit3,
  StickyNote,
  Activity,
  BarChart3,
  HelpCircle,
  Shield,
  X,
} from 'lucide-react';
import { Bin, SequentialStatus } from '../types';
import { useSidebarBinOrder } from '../hooks/useSidebarBinOrder';
import { getClipCollection, getSystemClipCollections, type ClipCollectionIcon, type ClipDropAction } from '../utils/clipCollections';
import type { FeatureId } from '../utils/features';
import { OverflowText } from './OverflowText';

const SEARCH_HELPERS = [
  { prefix: 'regex:', desc: 'Regex' },
  { prefix: 'app:', desc: 'App' },
  { prefix: 'type:', desc: 'Type' },
  { prefix: 'has:note', desc: 'Notes' },
  { prefix: 'is:pinned', desc: 'Pinned' },
  { prefix: 'is:protected', desc: 'Protected' },
  { prefix: 'is:trashed', desc: 'Trashed' },
] as const;

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
  onSearchFocus: () => void;
  onEmptySearchEscape: () => void;
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
  pointerDropTargetAction?: ClipDropAction | null;
  disabledDropBinId?: number | null;
  disabledDropActions?: ClipDropAction[];
  features: Record<FeatureId, boolean>;
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
  pointerDropTargetAction,
  disabledDropBinId,
  disabledDropActions = [],
  features,
  searchQuery,
  setSearchQuery,
  onSearchFocus,
  onEmptySearchEscape,
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
  const [isSearchMenuOpen, setIsSearchMenuOpen] = React.useState(false);
  const [activeSearchMenuIndex, setActiveSearchMenuIndex] = React.useState(-1);
  const searchMenuRootRef = React.useRef<HTMLDivElement | null>(null);
  const searchInputRef = React.useRef<HTMLInputElement | null>(null);
  const searchMenuItemRefs = React.useRef<Array<HTMLButtonElement | null>>([]);
  const searchHelpers = React.useMemo(
    () => SEARCH_HELPERS.filter(({ prefix }) => {
      if (prefix === 'has:note') return features.notes;
      if (prefix === 'is:pinned') return features.pinning;
      if (prefix === 'is:protected') return features.protection;
      if (prefix === 'is:trashed') return features.trash;
      return true;
    }),
    [features.notes, features.pinning, features.protection, features.trash],
  );

  const closeSearchMenu = (returnFocus = false) => {
    setIsSearchMenuOpen(false);
    setActiveSearchMenuIndex(-1);
    if (returnFocus) requestAnimationFrame(() => searchInputRef.current?.focus());
  };

  const focusSearchMenuItem = (index: number) => {
    const normalizedIndex = (index + searchHelpers.length) % searchHelpers.length;
    setActiveSearchMenuIndex(normalizedIndex);
    requestAnimationFrame(() => searchMenuItemRefs.current[normalizedIndex]?.focus());
  };

  React.useEffect(() => {
    if (!isSearchMenuOpen) return undefined;
    const closeOnOutsidePointer = (event: PointerEvent) => {
      if (!searchMenuRootRef.current?.contains(event.target as Node)) {
        closeSearchMenu();
      }
    };
    const closeOnOutsideFocus = (event: FocusEvent) => {
      if (!searchMenuRootRef.current?.contains(event.target as Node)) {
        closeSearchMenu();
      }
    };
    document.addEventListener('pointerdown', closeOnOutsidePointer);
    document.addEventListener('focusin', closeOnOutsideFocus);
    return () => {
      document.removeEventListener('pointerdown', closeOnOutsidePointer);
      document.removeEventListener('focusin', closeOnOutsideFocus);
    };
  }, [isSearchMenuOpen]);

  React.useEffect(() => {
    const focusSearch = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== 'f') return;
      event.preventDefault();
      setIsSearchMenuOpen(false);
      setActiveSearchMenuIndex(-1);
      onSearchFocus();
      requestAnimationFrame(() => {
        searchInputRef.current?.focus();
        searchInputRef.current?.select();
      });
    };
    window.addEventListener('keydown', focusSearch);
    return () => window.removeEventListener('keydown', focusSearch);
  }, [onSearchFocus]);

  const [isPostDragHoverSuppressed, setIsPostDragHoverSuppressed] = React.useState(false);
  const [hoveredSidebarControl, setHoveredSidebarControl] = React.useState<string | null>(null);
  const wasClipDraggingRef = React.useRef(false);
  const wasBinReorderingRef = React.useRef(false);
  const isPointerOverSidebarRef = React.useRef(false);
  const lastSidebarPointerRef = React.useRef<{ x: number; y: number } | null>(null);
  const isClipDragging = draggedClipId !== null && draggedClipId !== undefined;
  React.useEffect(() => {
    if (isClipDragging) closeSearchMenu();
  }, [isClipDragging]);
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
    if (wasClipDraggingRef.current && !isClipDragging) {
      setIsPostDragHoverSuppressed(isPointerOverSidebarRef.current);
    } else if (wasBinReorderingRef.current && !isBinReorderActive) {
      setIsPostDragHoverSuppressed(false);
      const pointer = lastSidebarPointerRef.current;
      if (isPointerOverSidebarRef.current && pointer) {
        const frame = requestAnimationFrame(() => {
          const control = document
            .elementFromPoint(pointer.x, pointer.y)
            ?.closest<HTMLElement>('[data-sidebar-hover-key]');
          setHoveredSidebarControl(control?.dataset.sidebarHoverKey ?? null);
        });
        wasClipDraggingRef.current = isClipDragging;
        wasBinReorderingRef.current = isBinReorderActive;
        return () => cancelAnimationFrame(frame);
      }
    }
    wasClipDraggingRef.current = isClipDragging;
    wasBinReorderingRef.current = isBinReorderActive;
  }, [isAnySidebarDrag, isBinReorderActive, isClipDragging]);

  const handleSidebarPointerEnter = () => {
    isPointerOverSidebarRef.current = true;
  };

  const handleSidebarPointerMove = (event: React.PointerEvent<HTMLElement>) => {
    lastSidebarPointerRef.current = { x: event.clientX, y: event.clientY };
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
    lastSidebarPointerRef.current = null;
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

  const collectionIcon = (icon: ClipCollectionIcon) => {
    if (icon === 'queue') return <ListOrdered className="sidebar-icon-secondary w-5 h-5" />;
    if (icon === 'pin') return <Pin className="sidebar-icon-success w-5 h-5 pin-icon" />;
    if (icon === 'protect') return <Shield className="sidebar-icon-info w-5 h-5" />;
    if (icon === 'note') return <StickyNote className="sidebar-icon-note w-5 h-5" />;
    if (icon === 'trash') return <Trash2 className="sidebar-icon-danger w-5 h-5" />;
    return <Clipboard className="sidebar-icon-primary w-5 h-5" />;
  };
  const clipNavItems = getSystemClipCollections(features).map((collection) => ({
    ...collection,
    icon: collectionIcon(collection.icon),
    dropAction: collection.capabilities.dropAction,
  }));
  const allToolNavItems: Array<{ tab: string; label: string; title: string; icon: React.ReactElement<{ className: string; strokeWidth?: number }>; feature?: FeatureId }> = [
    { tab: 'analytics', label: 'Analytics & Insights', title: 'Analytics & Insights', icon: <BarChart3 className="sidebar-icon-primary w-5 h-5" />, feature: 'analytics' },
    { tab: 'transformations', label: 'Transformations', title: 'Transformations', icon: <Workflow className="sidebar-icon-primary w-5 h-5" />, feature: 'transformations' },
    { tab: 'activity', label: 'Activity Log', title: 'Activity Log', icon: <Activity className="sidebar-icon-info w-5 h-5" />, feature: 'activityLog' },
    { tab: 'help', label: 'Help & Documentation', title: 'Help & Documentation', icon: <HelpCircle className="sidebar-icon-info w-5 h-5" />, feature: 'help' },
    { tab: 'settings', label: 'Settings', title: 'Settings', icon: <Settings className="sidebar-icon-primary w-5 h-5" /> },
  ];
  const toolNavItems = allToolNavItems.filter(({ feature }) => !feature || features[feature]);

  const clipCountByTab: Record<string, number> = {
    all: totalClipCount,
    sequential: seqStatus?.total_count ?? 0,
    pinned: pinnedCount,
    protected: protectedCount,
    notes: notesCount,
    trash: trashedCount,
  };

  const getDropActionTitle = (action: ClipDropAction) => {
    if (!disabledDropActions.includes(action)) {
      if (action === 'queue') return 'Add to Queue';
      if (action === 'pin') return 'Pin';
      if (action === 'protect') return 'Protect';
      return 'Move to Trash';
    }
    if (action === 'queue') return 'Text Clips Only';
    if (action === 'pin') return 'Already Pinned';
    if (action === 'protect') return 'Already Protected';
    return 'Protected';
  };

  if (isCollapsed) {
    return (
      <aside
        onPointerEnter={handleSidebarPointerEnter}
        onPointerMove={handleSidebarPointerMove}
        onPointerLeave={handleSidebarPointerLeave}
        className={`w-[100px] col-sidebar h-screen flex flex-col items-center border-r backdrop-blur-xl select-none ${isSidebarHoverMuted ? 'suppress-sidebar-hover' : ''}`}
      >
        {/* macOS reserves this header for overlaid traffic lights. Native framed
            platforms can use it for the sidebar control immediately. */}
        <div
          onMouseDown={startWindowDrag}
          className="platform-sidebar-header h-[56px] w-full cursor-default titlebar-drag-handle shrink-0"
        >
          <button
            data-sidebar-hover-key="expand-header"
            onClick={() => setIsCollapsed(false)}
            disabled={isClipDragging}
            className={`platform-framed-only sidebar-control-muted ui-control-radius w-9 h-9 items-center justify-center p-0 transition-colors duration-75 border titlebar-no-drag ${isClipDragging ? 'border-transparent cursor-default' : `cursor-pointer ${hoveredSidebarControl === 'expand-header' ? 'sidebar-item-hovered' : 'border-transparent'}`}`}
            title="Expand Sidebar"
          >
            <PanelLeftOpen className="w-5 h-5" />
          </button>
        </div>

        {/* Scrollable Nav Items Container for small window heights */}
        <div className="w-full flex-1 overflow-y-auto overflow-x-hidden sidebar-scroll-container flex flex-col items-center gap-1.5 py-2 px-1 custom-scrollbar">
          {/* macOS keeps this below the overlaid traffic-light safe area. */}
          <button
            data-sidebar-hover-key="expand"
            onClick={() => setIsCollapsed(false)}
            disabled={isClipDragging}
            className={`platform-macos-only sidebar-control-muted ui-control-radius w-9 h-9 items-center justify-center p-0 transition-colors duration-75 border shrink-0 ${isClipDragging ? 'border-transparent cursor-default' : `cursor-pointer ${hoveredSidebarControl === 'expand' ? 'sidebar-item-hovered' : 'border-transparent'}`}`}
            title="Expand Sidebar"
          >
            <PanelLeftOpen className="w-5 h-5" />
          </button>

          <div className="w-full flex items-center justify-center py-1 shrink-0">
            <div className="w-8 border-t sidebar-divider" />
          </div>

          {clipNavItems.map((item) => {
            const isActionDisabled = item.dropAction !== undefined && disabledDropActions.includes(item.dropAction);
            const isEligibleAction = isClipDragging && item.dropAction !== undefined && !isActionDisabled;
            const isActionTarget = isEligibleAction && pointerDropTargetAction === item.dropAction;
            return (
              <button
                key={item.tab}
                data-sidebar-hover-key={`clip:${item.tab}`}
                data-clip-drop-action={isEligibleAction ? item.dropAction : undefined}
                onClick={isClipDragging ? undefined : () => navigateTo(item.tab)}
                disabled={isClipDragging && !isEligibleAction}
                className={`ui-control-radius w-9 h-9 flex items-center justify-center p-0 transition-colors duration-75 border shrink-0 ${
                  isActionTarget
                    ? `sidebar-action-drop sidebar-action-drop-${item.dropAction} sidebar-action-drop-target cursor-grabbing`
                    : isEligibleAction
                    ? `sidebar-action-drop sidebar-action-drop-${item.dropAction} sidebar-action-drop-eligible cursor-grabbing`
                    : isClipDragging
                    ? 'sidebar-action-drop-ineligible cursor-default'
                    : currentTab === item.tab && (item.tab !== 'all' || selectedBinId === null)
                    ? 'sidebar-item-active shadow-sm cursor-pointer'
                    : hoveredSidebarControl === `clip:${item.tab}`
                    ? 'sidebar-item-hovered border-transparent cursor-pointer'
                    : 'sidebar-item-idle border-transparent cursor-pointer'
                }`}
                title={isClipDragging && item.dropAction ? getDropActionTitle(item.dropAction) : item.tooltip ?? item.title}
              >
                {item.icon}
              </button>
            );
          })}

          {features.bins && sortedBins.length > 0 && (
            <div className="w-full flex items-center justify-center py-1 shrink-0">
              <div className="w-8 border-t sidebar-divider" />
            </div>
          )}

          {features.bins && sortedBins.map((b) => (
            <button
              key={b.id}
              data-sidebar-hover-key={`bin:${b.id}`}
              onClick={() => {
                setCurrentTab('bin');
                setSelectedBinId(b.id);
              }}
              disabled={isClipDragging}
              className={`ui-control-radius w-9 h-9 flex items-center justify-center p-0 transition-colors duration-75 border shrink-0 cursor-pointer ${
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
              className={`ui-control-radius w-9 h-9 flex items-center justify-center p-0 transition-colors duration-75 border shrink-0 cursor-pointer ${
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
      {/* Only macOS needs an in-content titlebar row for overlaid traffic
          lights. Framed platforms place the collapse control beside Clips. */}
      <div
        onMouseDown={isClipDragging ? undefined : startWindowDrag}
        className="platform-macos-only h-[60px] px-4 items-center justify-between border-b border-transparent cursor-default titlebar-drag-handle shrink-0"
      >
        <div className="sidebar-titlebar-leading flex items-center titlebar-drag-handle" />
        <button
          data-sidebar-hover-key="collapse"
          onClick={() => setIsCollapsed(true)}
          disabled={isClipDragging}
          className={`sidebar-control-muted p-1.5 rounded-lg transition-colors titlebar-no-drag ${isClipDragging ? 'cursor-default' : `cursor-pointer ${hoveredSidebarControl === 'collapse' ? 'sidebar-item-hovered' : ''}`}`}
          title="Collapse Sidebar"
        >
          <PanelLeftClose className="w-4 h-4" />
        </button>
      </div>

      {/* Sidebar Navigation Content (Scrollable) */}
      <div className="flex-1 overflow-y-auto sidebar-scroll-container px-2.5 py-2 space-y-3 text-[0.8125rem]">
        {/* Section 1: Clips */}
        <div>
          <div
            data-sidebar-hover-key="section:clips"
            onClick={isClipDragging ? undefined : () => setIsClipsOpen(!isClipsOpen)}
            className={`px-2.5 pb-1 flex items-center justify-between select-none ${isClipDragging ? 'cursor-default' : 'cursor-pointer'}`}
            title="Toggle Clips"
          >
            <span className={`sidebar-section-label text-[11px] font-semibold transition-colors tracking-tight ${hoveredSidebarControl === 'section:clips' ? 'is-hovered' : ''}`}>
              Clips
            </span>
            <button
              data-sidebar-hover-key="collapse-framed"
              onClick={(event) => {
                event.stopPropagation();
                setIsCollapsed(true);
              }}
              disabled={isClipDragging}
              className={`platform-framed-only sidebar-control-muted h-7 w-7 items-center justify-center rounded-lg transition-colors titlebar-no-drag ${isClipDragging ? 'cursor-default' : `cursor-pointer ${hoveredSidebarControl === 'collapse-framed' ? 'sidebar-item-hovered' : ''}`}`}
              title="Collapse Sidebar"
            >
              <PanelLeftClose className="h-4 w-4" />
            </button>
          </div>
          <div
            className={`transition-[background-color,border-color,color,opacity,transform] duration-150 ease-in-out ${
              isClipsOpen ? 'max-h-96 opacity-100 mt-0 overflow-visible' : 'max-h-0 opacity-0 overflow-hidden'
            }`}
          >
            <nav className="space-y-0.5">
              {clipNavItems.map((item) => {
                const count = clipCountByTab[item.tab];
                const isActionDisabled = item.dropAction !== undefined && disabledDropActions.includes(item.dropAction);
                const isEligibleAction = isClipDragging && item.dropAction !== undefined && !isActionDisabled;
                const isActionTarget = isEligibleAction && pointerDropTargetAction === item.dropAction;
                return (
                  <button
                    key={item.tab}
                    data-sidebar-hover-key={`clip:${item.tab}`}
                    data-clip-drop-action={isEligibleAction ? item.dropAction : undefined}
                    onClick={isClipDragging ? undefined : () => navigateTo(item.tab)}
                    disabled={isClipDragging && !isEligibleAction}
                    title={isClipDragging && item.dropAction ? getDropActionTitle(item.dropAction) : undefined}
                    className={`sidebar-nav-row justify-between transition-colors duration-100 ${
                      isActionTarget
                        ? `sidebar-action-drop sidebar-action-drop-${item.dropAction} sidebar-action-drop-target cursor-grabbing`
                        : isEligibleAction
                        ? `sidebar-action-drop sidebar-action-drop-${item.dropAction} sidebar-action-drop-eligible cursor-grabbing`
                        : isClipDragging
                        ? 'sidebar-action-drop-ineligible cursor-default'
                        : currentTab === item.tab && (item.tab !== 'all' || selectedBinId === null)
                        ? 'sidebar-item-active font-medium'
                        : hoveredSidebarControl === `clip:${item.tab}`
                        ? 'sidebar-item-hovered font-normal'
                        : 'sidebar-item-idle font-normal cursor-pointer'
                    }`}
                  >
                    <div className="flex items-center gap-3 min-w-0">
                      <span className="sidebar-nav-icon">
                        {React.cloneElement(item.icon, { className: item.icon.props.className.replace('w-5 h-5', 'w-4 h-4 shrink-0'), strokeWidth: 1.8 })}
                      </span>
                      <OverflowText text={item.label} className="truncate" />
                    </div>
                    {item.tab === 'sequential' && seqStatus?.is_active ? (
                      <span className="theme-status-success-dot w-2 h-2 rounded-full animate-pulse" />
                    ) : count > 0 ? (
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
        {features.bins && <div>
          <div
            data-sidebar-hover-key="section:bins"
            onClick={isClipDragging ? undefined : () => setIsBinsOpen(!isBinsOpen)}
            className={`px-2.5 pb-1 flex items-center justify-between select-none ${isClipDragging ? 'cursor-default' : 'cursor-pointer'}`}
            title="Toggle Bins"
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
              title="New Bin"
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
                const binCollection = getClipCollection('bin', b);
                const isManualBin = Boolean(binCollection?.capabilities.acceptsClipDrop);
                const isDisabledDropTarget =
                  isClipDragging && disabledDropBinId === b.id;
                const isIneligibleSmartBin = isClipDragging && !isManualBin;
                const isDropTarget =
                  (dropTargetBinId === b.id || pointerDropTargetBinId === b.id) &&
                  isManualBin &&
                  !isDisabledDropTarget;
                const isBinHovered = !isSidebarHoverMuted && hoveredSidebarControl === `bin:${b.id}`;
                const dropAccent = binTextColor(b.color) ?? 'var(--accent-primary)';

                return (
                  <div
                    key={b.id}
                    data-stable-reorder-id={String(b.id)}
                    style={{
                      '--sidebar-bin-drop-color': dropAccent,
                      ...(binReorderOffsets[b.id] !== undefined ? {
                        transform: `translateY(${binReorderOffsets[b.id]}px)`,
                        zIndex: activeDragBinId === b.id ? 20 : 10,
                      } : {}),
                    } as React.CSSProperties}
                    data-sidebar-hover-key={`bin:${b.id}`}
                    data-bin-drop-id={isManualBin && !isDisabledDropTarget ? b.id : undefined}
                    role="button"
                    tabIndex={0}
                    title={
                      isDisabledDropTarget
                        ? 'Already in This Bin'
                        : isIneligibleSmartBin
                        ? 'Smart Bin — Automatic'
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
                        ? 'sidebar-bin-drop-target'
                        : isDisabledDropTarget || isIneligibleSmartBin
                        ? 'sidebar-bin-ineligible'
                        : isClipDragging && isManualBin
                        ? 'sidebar-bin-drop-eligible font-normal'
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
                      <OverflowText text={b.name} className="truncate" style={{ color: binTextColor(b.color) }} />
                    </div>

                    {/* Right side container */}
                    <div className="flex items-center justify-end shrink-0 pl-1">
                      <div className={`flex items-center space-x-1.5 ${isBinHovered && !isDragging ? 'hidden' : ''}`}>
                        {b.smart_rule && (b.clip_count ?? 0) > 0 ? (
                          <span
                            title={`Smart Bin · ${b.clip_count} Matches`}
                            className="sidebar-badge text-[11px] px-1.5 py-0.5 rounded-md font-mono flex items-center space-x-1"
                          >
                            <Sparkles className="theme-note-text w-3 h-3 shrink-0" />
                            <span>{b.clip_count}</span>
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
        </div>}

        {/* Section 3: Tools */}
        <div>
          <div
            data-sidebar-hover-key="section:tools"
            onClick={isClipDragging ? undefined : () => setIsToolsOpen(!isToolsOpen)}
            className={`px-2.5 pb-1 flex items-center justify-between select-none ${isClipDragging ? 'cursor-default' : 'cursor-pointer'}`}
            title="Toggle Tools"
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
                  <OverflowText text={item.label} className="truncate" />
                </button>
              ))}
            </nav>
          </div>
        </div>
      </div>

      {/* Pinned Bottom Search Bar Footer */}
      <div ref={searchMenuRootRef} className="sidebar-divider p-2.5 border-t shrink-0 relative">
        {!isClipDragging && isSearchMenuOpen && (
          <div
            id="sidebar-search-filters"
            role="menu"
            aria-label="Search filters"
            className="theme-menu absolute bottom-11 left-2.5 right-2.5 rounded-xl border p-1.5 text-xs font-medium select-none"
          >
              {searchHelpers.map((s, index) => (
              <button
                ref={(element) => {
                  searchMenuItemRefs.current[index] = element;
                }}
                type="button"
                role="menuitem"
                key={s.prefix}
                onMouseDown={(e) => {
                  e.preventDefault();
                }}
                onClick={() => {
                  setSearchQuery(s.prefix);
                  closeSearchMenu(true);
                }}
                onFocus={() => setActiveSearchMenuIndex(index)}
                onKeyDown={(event) => {
                  if (event.key === 'ArrowDown') {
                    event.preventDefault();
                    focusSearchMenuItem(index + 1);
                  } else if (event.key === 'ArrowUp') {
                    event.preventDefault();
                    focusSearchMenuItem(index - 1);
                  } else if (event.key === 'Home') {
                    event.preventDefault();
                    focusSearchMenuItem(0);
                  } else if (event.key === 'End') {
                    event.preventDefault();
                    focusSearchMenuItem(searchHelpers.length - 1);
                  } else if (event.key === 'Escape') {
                    event.preventDefault();
                    event.stopPropagation();
                    closeSearchMenu(true);
                  } else if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    setSearchQuery(s.prefix);
                    closeSearchMenu(true);
                  }
                }}
                className={`theme-menu-item w-full px-2.5 py-1.5 rounded-lg cursor-pointer flex items-center justify-between gap-3 text-left ${activeSearchMenuIndex === index ? 'is-selected' : ''}`}
              >
                <span className="font-mono text-[11px] font-semibold">{s.prefix}</span>
                <span className="theme-text-subtle text-[10px]">{s.desc}</span>
              </button>
            ))}
          </div>
        )}

        <div className="relative titlebar-no-drag">
          <input
            ref={searchInputRef}
            data-sidebar-search-input
            type="text"
            disabled={isClipDragging}
            autoComplete="off"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck={false}
            placeholder="Search all clips"
            value={searchQuery}
            onFocus={() => {
              onSearchFocus();
            }}
            onKeyDown={(e) => {
              if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
                e.preventDefault();
                if (!isSearchMenuOpen) setIsSearchMenuOpen(true);
                focusSearchMenuItem(e.key === 'ArrowDown' ? 0 : searchHelpers.length - 1);
              } else if (e.key === 'Escape') {
                e.preventDefault();
                if (isSearchMenuOpen) closeSearchMenu();
                else {
                  (e.target as HTMLInputElement).blur();
                  if (!searchQuery.trim()) onEmptySearchEscape();
                }
              }
            }}
            onChange={(e) => setSearchQuery(e.target.value)}
            className={`sidebar-search-input theme-input w-full h-7 border rounded-md pl-2.5 ${searchQuery ? 'pr-14' : 'pr-8'} text-[12px] focus:outline-none transition-colors titlebar-no-drag`}
          />
          {searchQuery && (
            <button
              type="button"
              disabled={isClipDragging}
              aria-label="Clear search"
              title="Clear Search"
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => {
                setSearchQuery('');
                closeSearchMenu();
                onSearchFocus();
                requestAnimationFrame(() => searchInputRef.current?.focus());
              }}
              className="sidebar-search-clear theme-menu-item absolute right-6 top-1 grid h-5 w-5 place-items-center rounded"
            >
              <X className="h-3 w-3" aria-hidden="true" />
            </button>
          )}
          <button
            type="button"
            disabled={isClipDragging}
            aria-label="Search filters"
            aria-haspopup="menu"
            aria-expanded={isSearchMenuOpen}
            aria-controls="sidebar-search-filters"
            title="Search Filters"
            onClick={() => {
              onSearchFocus();
              setIsSearchMenuOpen((open) => {
                if (open) setActiveSearchMenuIndex(-1);
                return !open;
              });
              requestAnimationFrame(() => searchInputRef.current?.focus());
            }}
            onKeyDown={(event) => {
              if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
                event.preventDefault();
                setIsSearchMenuOpen(true);
                focusSearchMenuItem(event.key === 'ArrowDown' ? 0 : searchHelpers.length - 1);
              } else if (event.key === 'Escape' && isSearchMenuOpen) {
                event.preventDefault();
                closeSearchMenu();
              }
            }}
            className={`theme-menu-item absolute right-1 top-1 grid h-5 w-5 place-items-center rounded ${isSearchMenuOpen ? 'is-selected' : ''}`}
          >
            <ChevronUp className={`h-3.5 w-3.5 transition-transform ${isSearchMenuOpen ? 'rotate-180' : ''}`} aria-hidden="true" />
          </button>
        </div>
      </div>
    </aside>
  );
};
