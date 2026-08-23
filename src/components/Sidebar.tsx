import React from 'react';
import { PanelLeftClose } from 'lucide-react';

import { useSidebarBinOrder } from '../hooks/useSidebarBinOrder';
import { useSidebarFacets } from '../hooks/useSidebarFacets';
import { useSidebarHoverState } from '../hooks/useSidebarHoverState';
import { useLocalization } from '../localization/LocalizationProvider';
import { translate } from '../localization/runtime';
import type { Bin, ClipCollectionSummary, SequentialStatus } from '../types';
import type { SidebarSectionId, SidebarSectionState } from '../utils/appUiState';
import type { ClipDropAction } from '../utils/clipCollections';
import type { FeatureId } from '../utils/features';
import { handleWindowDragDoubleClick, startWindowDrag } from '../utils/windowDrag';
import { CollapsedSidebar } from './CollapsedSidebar';
import { useContentTypes } from './ContentTypeProvider';
import { SidebarBinsSection } from './SidebarBinsSection';
import { SidebarClipSection } from './SidebarClipSection';
import { SidebarFacetSections, type SidebarFacetSection } from './SidebarFacetSections';
import { buildClipCountByTab, buildSidebarNavigation } from './sidebarNavigationModel';
import { SidebarSearchFooter } from './SidebarSearchFooter';
import { SidebarToolsSection } from './SidebarToolsSection';

interface SidebarProps {
  currentTab: string;
  setCurrentTab: (tab: string) => void;
  selectedBinId: number | null;
  setSelectedBinId: (id: number | null) => void;
  bins: Bin[];
  clipCollectionSummary: ClipCollectionSummary;
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
  sectionState: SidebarSectionState;
  onSectionStateChange: (section: SidebarSectionId, open: boolean) => void;
}

const SidebarComponent: React.FC<SidebarProps> = ({
  currentTab,
  setCurrentTab,
  selectedBinId,
  setSelectedBinId,
  bins,
  clipCollectionSummary,
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
  isCollapsed,
  setIsCollapsed,
  sidebarWidth = 240,
  sectionState,
  onSectionStateChange,
}) => {
  const { locale } = useLocalization();
  const { definitions: contentTypes } = useContentTypes();
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
  const {
    hoveredControl: hoveredSidebarControl,
    isHoverMuted: isSidebarHoverMuted,
    onPointerEnter: handleSidebarPointerEnter,
    onPointerMove: handleSidebarPointerMove,
    onPointerLeave: handleSidebarPointerLeave,
  } = useSidebarHoverState(isClipDragging, isBinReorderActive);
  const { clipNavItems, toolNavItems } = buildSidebarNavigation(features);
  const clipCountByTab = buildClipCountByTab(clipCollectionSummary, totalClipCount, seqStatus);
  const { typeItems, clipTypeItems, fileFormatItems, sourceItems, sourceIcons } = useSidebarFacets(
    clipCollectionSummary,
    contentTypes,
    locale,
    features.sources,
  );

  const navigateTo = (tab: string) => {
    setCurrentTab(tab);
    setSelectedBinId(null);
  };
  const setSectionOpen = (section: SidebarSectionId) => (open: boolean) => {
    onSectionStateChange(section, open);
  };
  const allFacetSections: SidebarFacetSection[] = [
    {
      id: 'clipTypes',
      label: translate('component.sidebar.clipTypes'),
      open: sectionState.clipTypes,
      setOpen: setSectionOpen('clipTypes'),
      items: clipTypeItems,
    },
    {
      id: 'types',
      label: translate('component.sidebar.contentTypes'),
      open: sectionState.types,
      setOpen: setSectionOpen('types'),
      items: typeItems,
    },
    {
      id: 'fileFormats',
      label: translate('component.sidebar.fileFormats'),
      open: sectionState.fileFormats,
      setOpen: setSectionOpen('fileFormats'),
      items: fileFormatItems,
    },
    {
      id: 'sources',
      label: translate('component.sidebar.sources'),
      open: sectionState.sources,
      setOpen: setSectionOpen('sources'),
      items: sourceItems,
    },
  ];
  const facetSections = allFacetSections.filter(
    (section) => features[section.id] && section.items.length > 0,
  );
  const getDropActionTitle = (action: ClipDropAction) => {
    if (!disabledDropActions.includes(action)) {
      if (action === 'queue') return 'Add to Queue';
      if (action === 'pin') return 'Pin';
      if (action === 'protect') return 'Protect';
      if (action === 'conceal') return translate('action.conceal');
      return 'Move to Trash';
    }
    if (action === 'queue') return 'Text Clips Only';
    if (action === 'pin') return 'Already Pinned';
    if (action === 'protect') return 'Already Protected';
    if (action === 'conceal') return translate('component.sidebar.alreadyConcealed');
    return 'Protected';
  };

  if (isCollapsed) {
    return (
      <CollapsedSidebar
        binsEnabled={features.bins}
        bins={sortedBins}
        clipNavItems={clipNavItems}
        toolNavItems={toolNavItems}
        currentTab={currentTab}
        selectedBinId={selectedBinId}
        isClipDragging={isClipDragging}
        disabledDropActions={disabledDropActions}
        pointerDropTargetAction={pointerDropTargetAction}
        hoveredControl={hoveredSidebarControl}
        isHoverMuted={isSidebarHoverMuted}
        setIsCollapsed={setIsCollapsed}
        navigateTo={navigateTo}
        selectBin={(id) => {
          setCurrentTab('bin');
          setSelectedBinId(id);
        }}
        getDropActionTitle={getDropActionTitle}
        onPointerEnter={handleSidebarPointerEnter}
        onPointerMove={handleSidebarPointerMove}
        onPointerLeave={handleSidebarPointerLeave}
      />
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
      <div
        onMouseDown={isClipDragging ? undefined : startWindowDrag}
        onDoubleClick={isClipDragging ? undefined : handleWindowDragDoubleClick}
        className="platform-macos-only h-[60px] px-4 items-center justify-between border-b border-transparent cursor-default titlebar-drag-handle shrink-0"
      >
        <div className="sidebar-titlebar-leading flex items-center titlebar-drag-handle" />
        <button
          data-sidebar-hover-key="collapse"
          onClick={() => setIsCollapsed(true)}
          disabled={isClipDragging}
          className={`sidebar-control-muted p-1.5 rounded-lg transition-colors titlebar-no-drag ${isClipDragging ? 'cursor-default' : `cursor-pointer ${hoveredSidebarControl === 'collapse' ? 'sidebar-item-hovered' : ''}`}`}
          title={translate('component.sidebar.collapseSidebar')}
        >
          <PanelLeftClose className="h-4 w-4 rtl:-scale-x-100" />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto sidebar-scroll-container px-2.5 pt-2 pb-3 space-y-3 text-[0.8125rem]">
        <SidebarClipSection
          items={clipNavItems}
          counts={clipCountByTab}
          currentTab={currentTab}
          selectedBinId={selectedBinId}
          seqStatus={seqStatus}
          isOpen={sectionState.clips}
          isClipDragging={isClipDragging}
          disabledDropActions={disabledDropActions}
          pointerDropTargetAction={pointerDropTargetAction}
          hoveredControl={hoveredSidebarControl}
          setIsOpen={setSectionOpen('clips')}
          setIsCollapsed={setIsCollapsed}
          navigateTo={navigateTo}
          getDropActionTitle={getDropActionTitle}
        />
        {features.bins && (
          <SidebarBinsSection
            bins={sortedBins}
            isOpen={sectionState.bins}
            setIsOpen={setSectionOpen('bins')}
            currentTab={currentTab}
            selectedBinId={selectedBinId}
            isClipDragging={isClipDragging}
            draggedClipId={draggedClipId}
            disabledDropBinId={disabledDropBinId}
            pointerDropTargetBinId={pointerDropTargetBinId}
            activeDragBinId={activeDragBinId}
            reorderOffsets={binReorderOffsets}
            isReorderSettling={isBinReorderSettling}
            isHoverMuted={isSidebarHoverMuted}
            hoveredControl={hoveredSidebarControl}
            binListRef={binListRef}
            onStartBinDrag={handlePointerDownBin}
            consumeBinDragClick={consumeBinDragClick}
            onOpenNewBin={onOpenNewBinModal}
            onSelectBin={(id) => {
              setCurrentTab('bin');
              setSelectedBinId(id);
            }}
            onClipDropOnBin={onClipDropOnBin}
            onEditBin={onEditBin}
            onDeleteBin={onDeleteBin}
            onBinContextMenu={onBinContextMenu}
          />
        )}
        <SidebarFacetSections
          sections={facetSections}
          sourceIcons={sourceIcons}
          currentTab={currentTab}
          isClipDragging={isClipDragging}
          hoveredControl={hoveredSidebarControl}
          navigateTo={navigateTo}
        />
        <SidebarToolsSection
          items={toolNavItems}
          currentTab={currentTab}
          isOpen={sectionState.tools}
          isClipDragging={isClipDragging}
          hoveredControl={hoveredSidebarControl}
          setIsOpen={setSectionOpen('tools')}
          navigateTo={navigateTo}
        />
      </div>

      {features.search && (
        <SidebarSearchFooter
          features={features}
          isDragActive={isClipDragging}
          searchQuery={searchQuery}
          setSearchQuery={setSearchQuery}
          onSearchFocus={onSearchFocus}
          onEmptySearchEscape={onEmptySearchEscape}
        />
      )}
    </aside>
  );
};

export const Sidebar = React.memo(SidebarComponent);
