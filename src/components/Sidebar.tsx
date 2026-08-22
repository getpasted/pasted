import React from 'react';
import { localizedSourceName } from '../localization/presentation';
import { handleWindowDragDoubleClick, startWindowDrag } from '../utils/windowDrag';
import {
  Clipboard,
  Pin,
  ListOrdered,
  Workflow,
  Settings,
  Trash2,
  PanelLeftClose,
  StickyNote,
  Activity,
  BarChart3,
  HelpCircle,
  Shield,
  EyeOff,
  AppWindow,
  Camera,
  CircleHelp,
  Globe2,
  MonitorCog,
  Smartphone,
  TerminalSquare,
  FileText,
  Image as ImageIcon,
  Files,
  FileType2,
  FilePenLine,
} from 'lucide-react';
import { Bin, ClipCollectionSummary, type ClipContentType, SequentialStatus } from '../types';
import { useSidebarBinOrder } from '../hooks/useSidebarBinOrder';
import { useSidebarHoverState } from '../hooks/useSidebarHoverState';
import { clipFacetRoute, getSystemClipCollections, type ClipCollectionIcon, type ClipDropAction } from '../utils/clipCollections';
import { CLIP_PROPERTY_ASSOCIATIONS } from '../utils/clipPropertyAssociations';
import type { FeatureId } from '../utils/features';
import { OverflowText } from './OverflowText';
import { SidebarSearchFooter } from './SidebarSearchFooter';
import { CollapsedSidebar } from './CollapsedSidebar';
import { SidebarBinsSection } from './SidebarBinsSection';
import { ContentTypeIcon } from './ContentTypeIcon';
import { useContentTypes } from './ContentTypeProvider';
import { contentTypeLabel } from '../utils/contentTypes';
import { safeInvoke as invoke } from '../utils/tauri';
import type { SidebarSectionId, SidebarSectionState } from '../utils/appUiState';
import { SafeRasterImage } from './SafeRasterImage';
import { translate } from '../localization/runtime';
import { useLocalization } from '../localization/LocalizationProvider';

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

  // Section Collapse State
  const isClipsOpen = sectionState.clips;
  const isBinsOpen = sectionState.bins;
  const isClipTypesOpen = sectionState.clipTypes;
  const isFileFormatsOpen = sectionState.fileFormats;
  const isTypesOpen = sectionState.types;
  const isSourcesOpen = sectionState.sources;
  const isToolsOpen = sectionState.tools;
  const setIsClipsOpen = (open: boolean) => onSectionStateChange('clips', open);
  const setIsBinsOpen = (open: boolean) => onSectionStateChange('bins', open);
  const setIsClipTypesOpen = (open: boolean) => onSectionStateChange('clipTypes', open);
  const setIsFileFormatsOpen = (open: boolean) => onSectionStateChange('fileFormats', open);
  const setIsTypesOpen = (open: boolean) => onSectionStateChange('types', open);
  const setIsSourcesOpen = (open: boolean) => onSectionStateChange('sources', open);
  const setIsToolsOpen = (open: boolean) => onSectionStateChange('tools', open);

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
  const sidebarHover = useSidebarHoverState(isClipDragging, isBinReorderActive);
  const {
    hoveredControl: hoveredSidebarControl,
    isHoverMuted: isSidebarHoverMuted,
    onPointerEnter: handleSidebarPointerEnter,
    onPointerMove: handleSidebarPointerMove,
    onPointerLeave: handleSidebarPointerLeave,
  } = sidebarHover;

  const sourceFallbackIcon = (source: string | null | undefined) => {
    const normalized = source?.trim().toLowerCase() ?? '';
    const className = 'sidebar-icon-primary h-4 w-4 shrink-0';
    if (normalized.includes('screenshot') || normalized.includes('screencapture')) {
      return <Camera className={className} strokeWidth={1.8} />;
    }
    if (normalized === 'system clipboard' || normalized === 'clipboard') {
      return <Clipboard className={className} strokeWidth={1.8} />;
    }
    if (normalized === 'continuity' || normalized === 'universal clipboard') {
      return <Smartphone className={className} strokeWidth={1.8} />;
    }
    if (normalized === 'macos system' || normalized === 'windows system' || normalized === 'linux system' || normalized === 'system') {
      return <MonitorCog className={className} strokeWidth={1.8} />;
    }
    if (normalized === 'browser' || normalized === 'web browser') {
      return <Globe2 className={className} strokeWidth={1.8} />;
    }
    if (normalized.includes('terminal') || normalized === 'pasted cli') {
      return <TerminalSquare className={className} strokeWidth={1.8} />;
    }
    if (!normalized || normalized === 'unknown' || normalized === 'unknown source') {
      return <CircleHelp className={className} strokeWidth={1.8} />;
    }
    return <AppWindow className={className} strokeWidth={1.8} />;
  };

  const navigateTo = (tab: string) => {
    setCurrentTab(tab);
    setSelectedBinId(null);
  };

  const collectionIcon = (icon: ClipCollectionIcon) => {
    if (icon === 'queue') return <ListOrdered className="sidebar-icon-secondary w-5 h-5" />;
    if (icon === 'pin') return <Pin className="sidebar-icon-success w-5 h-5 pin-icon" />;
    if (icon === 'protect') return <Shield className="sidebar-icon-info w-5 h-5" />;
    if (icon === 'conceal') return <EyeOff className="sidebar-icon-warning w-5 h-5" />;
    if (icon === 'name') return <FilePenLine className="sidebar-icon-named w-5 h-5" />;
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
    { tab: 'transformations', get label() { return translate('destination.transformations'); }, get title() { return translate('destination.transformations'); }, icon: <Workflow className="sidebar-icon-primary w-5 h-5" />, feature: 'transformations' },
    { tab: 'analytics', get label() { return translate('destination.insights'); }, get title() { return translate('destination.insights'); }, icon: <BarChart3 className="sidebar-icon-primary w-5 h-5" />, feature: 'analytics' },
    { tab: 'activity', get label() { return translate('destination.activity'); }, get title() { return translate('destination.activity'); }, icon: <Activity className="sidebar-icon-info w-5 h-5" />, feature: 'activityLog' },
    { tab: 'help', get label() { return translate('destination.help'); }, get title() { return translate('destination.help'); }, icon: <HelpCircle className="sidebar-icon-info w-5 h-5" />, feature: 'help' },
    { tab: 'settings', get label() { return translate('destination.settings'); }, get title() { return translate('destination.settings'); }, icon: <Settings className="sidebar-icon-primary w-5 h-5" /> },
  ];
  const toolNavItems = allToolNavItems.filter(({ feature }) => !feature || features[feature]);

  const typeItems = React.useMemo(() => {
    const order = new Map(contentTypes.map(({ id }, index) => [id, index]));
    const labels = new Map(contentTypes.map(({ id }) => [id, contentTypeLabel(id)]));
    return clipCollectionSummary.typeCounts.map(({ content_type: value, count }) => ({
      value,
      count,
      route: clipFacetRoute('content_type', value),
      label: labels.get(value) ?? value.split('_').map((part) => part.charAt(0).toUpperCase() + part.slice(1)).join(' '),
    })).sort((left, right) => (order.get(left.value) ?? Number.MAX_SAFE_INTEGER) - (order.get(right.value) ?? Number.MAX_SAFE_INTEGER));
  }, [clipCollectionSummary.typeCounts, contentTypes, locale]);
  const clipTypeItems = React.useMemo(() => {
    const definitions = [
      { value: 'text', label: translate('component.analyticsView.text') },
      { value: 'image', label: translate('component.analyticsView.image') },
      { value: 'file', label: translate('component.analyticsView.files') },
    ];
    const counts = new Map(clipCollectionSummary.clipTypeCounts.map(({ clip_type, count }) => [clip_type, count]));
    return definitions
      .map(({ value, label }) => ({ value, label, count: counts.get(value as 'text' | 'image' | 'file') ?? 0, route: clipFacetRoute('clip_type', value) }))
      .filter(({ count }) => count > 0);
  }, [clipCollectionSummary.clipTypeCounts, locale]);
  const fileFormatItems = React.useMemo(() => (
    clipCollectionSummary.fileFormatCounts.map(({ file_format: value, count }) => ({
      value,
      count,
      route: clipFacetRoute('file_format', value),
      label: value.toUpperCase(),
    }))
  ), [clipCollectionSummary.fileFormatCounts]);
  const sourceItems = React.useMemo(() => {
    return clipCollectionSummary.sourceCounts.map(({ name: value, count }) => ({
      value,
      count,
      route: clipFacetRoute('source', value),
      label: localizedSourceName(value),
    })).sort((left, right) => right.count - left.count || left.label.localeCompare(right.label));
  }, [clipCollectionSummary.sourceCounts]);
  const [sourceIcons, setSourceIcons] = React.useState<Record<string, string>>({});
  const sourceIconsRef = React.useRef<Record<string, string>>({});
  const requestedSourceIconsRef = React.useRef(new Set<string>());
  const sourceIconNames = React.useMemo(
    () => [...new Set(sourceItems.map(({ value }) => value))].sort((left, right) => left.localeCompare(right)).slice(0, 128),
    [sourceItems],
  );
  const sourceIconSignature = JSON.stringify(sourceIconNames);
  React.useEffect(() => {
    if (!features.sources || sourceIconNames.length === 0) return undefined;
    const missingSources = sourceIconNames.filter(
      (name) => !sourceIconsRef.current[name] && !requestedSourceIconsRef.current.has(name),
    );
    if (missingSources.length === 0) return undefined;
    missingSources.forEach((name) => requestedSourceIconsRef.current.add(name));
    void invoke<Record<string, string>>('get_source_icons', {
      sources: missingSources,
    }).then((icons) => {
      const merged = { ...sourceIconsRef.current, ...(icons ?? {}) };
      sourceIconsRef.current = merged;
      setSourceIcons(merged);
    }).catch((error) => {
      console.warn('Source icons are unavailable; restart Pasted after native updates.', error);
      missingSources.forEach((name) => requestedSourceIconsRef.current.delete(name));
    });
    return undefined;
  }, [features.sources, sourceIconSignature]);

  const clipCountByTab: Record<string, number> = {
    all: totalClipCount,
    sequential: seqStatus?.total_count ?? 0,
    notes: clipCollectionSummary.notedCount,
    trash: clipCollectionSummary.trashCount,
  };
  for (const association of CLIP_PROPERTY_ASSOCIATIONS) {
    clipCountByTab[association.membership] = clipCollectionSummary[association.countKey];
  }

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
      {/* Only macOS needs an in-content titlebar row for overlaid traffic
          lights. Framed platforms place the collapse control beside Clips. */}
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

      {/* Sidebar Navigation Content (Scrollable) */}
      <div className="flex-1 overflow-y-auto sidebar-scroll-container px-2.5 pt-2 pb-3 space-y-3 text-[0.8125rem]">
        {/* Section 1: Clips */}
        <div>
          <div
            data-sidebar-hover-key="section:clips"
            onClick={isClipDragging ? undefined : () => setIsClipsOpen(!isClipsOpen)}
            className={`px-2.5 pb-1 flex items-center justify-between select-none ${isClipDragging ? 'cursor-default' : 'cursor-pointer'}`}
            title={translate('component.sidebar.toggleClips')}
          >
            <span className={`sidebar-section-label text-[11px] font-semibold transition-colors tracking-tight ${hoveredSidebarControl === 'section:clips' ? 'is-hovered' : ''}`}>
              {translate('component.sidebar.clips')}
            </span>
            <button
              data-sidebar-hover-key="collapse-framed"
              onClick={(event) => {
                event.stopPropagation();
                setIsCollapsed(true);
              }}
              disabled={isClipDragging}
              className={`platform-framed-only sidebar-control-muted h-7 w-7 items-center justify-center rounded-lg transition-colors titlebar-no-drag ${isClipDragging ? 'cursor-default' : `cursor-pointer ${hoveredSidebarControl === 'collapse-framed' ? 'sidebar-item-hovered' : ''}`}`}
              title={translate('component.sidebar.collapseSidebar')}
            >
              <PanelLeftClose className="h-4 w-4 rtl:-scale-x-100" />
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

        {features.bins && (
          <SidebarBinsSection
            bins={sortedBins}
            isOpen={isBinsOpen}
            setIsOpen={setIsBinsOpen}
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
        {([
          { id: 'clipTypes', get label() { return translate('component.sidebar.clipTypes'); }, enabled: features.clipTypes, open: isClipTypesOpen, setOpen: setIsClipTypesOpen, items: clipTypeItems },
          { id: 'types', get label() { return translate('component.sidebar.contentTypes'); }, enabled: features.types, open: isTypesOpen, setOpen: setIsTypesOpen, items: typeItems },
          { id: 'fileFormats', get label() { return translate('component.sidebar.fileFormats'); }, enabled: features.fileFormats, open: isFileFormatsOpen, setOpen: setIsFileFormatsOpen, items: fileFormatItems },
          { id: 'sources', get label() { return translate('component.sidebar.sources'); }, enabled: features.sources, open: isSourcesOpen, setOpen: setIsSourcesOpen, items: sourceItems },
        ] as const).map((section) => section.enabled && section.items.length > 0 && (
          <div key={section.id}>
            <div
              data-sidebar-hover-key={`section:${section.id}`}
              onClick={isClipDragging ? undefined : () => section.setOpen(!section.open)}
              className={`px-2.5 pb-1 flex items-center justify-between select-none ${isClipDragging ? 'cursor-default' : 'cursor-pointer'}`}
              title={translate('component.sidebar.toggleLabel', { label: section.label })}
            >
              <span className={`sidebar-section-label text-[11px] font-semibold transition-colors tracking-tight ${hoveredSidebarControl === `section:${section.id}` ? 'is-hovered' : ''}`}>
                {section.label}
              </span>
            </div>
            <div className={`grid transition-[grid-template-rows,opacity] duration-150 ease-in-out ${section.open ? 'grid-rows-[1fr] opacity-100' : 'grid-rows-[0fr] opacity-0'}`}>
              <nav className="min-h-0 space-y-0.5 overflow-hidden">
                {section.items.map((item) => (
                  <button
                    key={item.route}
                    data-sidebar-hover-key={translate('component.sidebar.idRoute', { id: section.id, route: item.route })}
                    onClick={() => navigateTo(item.route)}
                    disabled={isClipDragging}
                    className={`sidebar-nav-row justify-between gap-3 transition-colors duration-100 ${isClipDragging ? 'cursor-default opacity-50' : currentTab === item.route ? 'sidebar-item-active font-medium cursor-pointer' : hoveredSidebarControl === `${section.id}:${item.route}` ? 'sidebar-item-hovered font-normal cursor-pointer' : 'sidebar-item-idle font-normal cursor-pointer'}`}
                  >
                    <div className="flex min-w-0 items-center gap-3">
                      <span className="sidebar-nav-icon">
                        {section.id === 'clipTypes'
                          ? item.value === 'text'
                            ? <FileText className="sidebar-icon-primary h-4 w-4 shrink-0" />
                            : item.value === 'image'
                            ? <ImageIcon className="sidebar-icon-primary h-4 w-4 shrink-0" />
                            : <Files className="sidebar-icon-primary h-4 w-4 shrink-0" />
                          : section.id === 'fileFormats'
                          ? <FileType2 className="sidebar-icon-primary h-4 w-4 shrink-0" />
                          : section.id === 'types'
                          ? <ContentTypeIcon type={item.value as ClipContentType} className="sidebar-icon-primary h-4 w-4 shrink-0" />
                          : sourceIcons[item.value]
                          ? <SafeRasterImage source={sourceIcons[item.value]} alt="" className="h-4 w-4 shrink-0 object-contain" />
                          : sourceFallbackIcon(item.value)}
                      </span>
                      <OverflowText text={item.label} className="truncate" />
                    </div>
                    <span className="sidebar-badge rounded-md px-1.5 py-0.5 font-mono text-[11px]">{item.count}</span>
                  </button>
                ))}
              </nav>
            </div>
          </div>
        ))}

        {/* Section 3: Tools */}
        <div>
          <div
            data-sidebar-hover-key="section:tools"
            onClick={isClipDragging ? undefined : () => setIsToolsOpen(!isToolsOpen)}
            className={`px-2.5 pb-1 flex items-center justify-between select-none ${isClipDragging ? 'cursor-default' : 'cursor-pointer'}`}
            title={translate('component.sidebar.toggleTools')}
          >
            <span className={`sidebar-section-label text-[11px] font-semibold transition-colors tracking-tight ${hoveredSidebarControl === 'section:tools' ? 'is-hovered' : ''}`}>
              {translate('component.sidebar.tools')}
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
