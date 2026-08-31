import React from 'react';
import { PanelLeftOpen } from 'lucide-react';

import { translate } from '../localization/runtime';
import type { Bin } from '../types';
import { formatEmojiIcon } from '../utils/emoji';
import type { ClipDropAction } from '../utils/clipCollections';
import { handleWindowDragDoubleClick, startWindowDrag } from '../utils/windowDrag';

interface CompactNavItem {
  tab: string;
  title: string;
  tooltip?: string;
  icon: React.ReactNode;
  dropAction?: ClipDropAction;
}

interface CollapsedSidebarProps {
  binsEnabled: boolean;
  bins: Bin[];
  clipNavItems: CompactNavItem[];
  toolNavItems: CompactNavItem[];
  currentTab: string;
  selectedBinId: number | null;
  isClipDragging: boolean;
  disabledDropActions: ClipDropAction[];
  pointerDropTargetAction?: ClipDropAction | null;
  hoveredControl: string | null;
  isHoverMuted: boolean;
  setIsCollapsed: (collapsed: boolean) => void;
  navigateTo: (tab: string) => void;
  selectBin: (id: number) => void;
  getDropActionTitle: (action: ClipDropAction) => string;
  onPointerEnter: () => void;
  onPointerMove: (event: React.PointerEvent<HTMLElement>) => void;
  onPointerLeave: () => void;
}

export function CollapsedSidebar({
  binsEnabled,
  bins,
  clipNavItems,
  toolNavItems,
  currentTab,
  selectedBinId,
  isClipDragging,
  disabledDropActions,
  pointerDropTargetAction,
  hoveredControl,
  isHoverMuted,
  setIsCollapsed,
  navigateTo,
  selectBin,
  getDropActionTitle,
  onPointerEnter,
  onPointerMove,
  onPointerLeave,
}: CollapsedSidebarProps) {
  return (
    <aside
      onPointerEnter={onPointerEnter}
      onPointerMove={onPointerMove}
      onPointerLeave={onPointerLeave}
      className={`w-[100px] col-sidebar h-screen flex flex-col items-center border-e select-none ${isHoverMuted ? 'suppress-sidebar-hover' : ''}`}
    >
      <div
        onMouseDown={startWindowDrag}
        onDoubleClick={handleWindowDragDoubleClick}
        className="platform-sidebar-header h-[56px] w-full cursor-default titlebar-drag-handle shrink-0"
      >
        <button
          data-sidebar-hover-key="expand-header"
          onClick={() => setIsCollapsed(false)}
          disabled={isClipDragging}
          className={`platform-framed-only sidebar-control-muted ui-control-radius w-9 h-9 items-center justify-center p-0 transition-colors duration-75 border titlebar-no-drag ${isClipDragging ? 'border-transparent cursor-default' : `cursor-pointer ${hoveredControl === 'expand-header' ? 'sidebar-item-hovered' : 'border-transparent'}`}`}
          title={translate('component.sidebar.expandSidebar')}
        >
          <PanelLeftOpen className="h-5 w-5 rtl:-scale-x-100" />
        </button>
      </div>
      <div data-pasted-scroll-key="sidebar:collapsed" className="w-full flex-1 overflow-y-auto overflow-x-hidden sidebar-scroll-container flex flex-col items-center gap-1.5 py-2 px-1 custom-scrollbar">
        <button
          data-sidebar-hover-key="expand"
          onClick={() => setIsCollapsed(false)}
          disabled={isClipDragging}
          className={`platform-macos-only sidebar-control-muted ui-control-radius w-9 h-9 items-center justify-center p-0 transition-colors duration-75 border shrink-0 ${isClipDragging ? 'border-transparent cursor-default' : `cursor-pointer ${hoveredControl === 'expand' ? 'sidebar-item-hovered' : 'border-transparent'}`}`}
          title={translate('component.sidebar.expandSidebar')}
        >
          <PanelLeftOpen className="h-5 w-5 rtl:-scale-x-100" />
        </button>
        <div className="w-full flex items-center justify-center py-1 shrink-0"><div className="w-8 border-t sidebar-divider" /></div>
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
                  ? 'sidebar-item-active cursor-pointer'
                  : hoveredControl === `clip:${item.tab}`
                  ? 'sidebar-item-hovered border-transparent cursor-pointer'
                  : 'sidebar-item-idle border-transparent cursor-pointer'
              }`}
              title={isClipDragging && item.dropAction ? getDropActionTitle(item.dropAction) : item.tooltip ?? item.title}
            >
              {item.icon}
            </button>
          );
        })}
        {binsEnabled && bins.length > 0 && (
          <div className="w-full flex items-center justify-center py-1 shrink-0"><div className="w-8 border-t sidebar-divider" /></div>
        )}
        {binsEnabled && bins.map((bin) => (
          <button
            key={bin.id}
            data-sidebar-hover-key={`bin:${bin.id}`}
            onClick={() => selectBin(bin.id)}
            disabled={isClipDragging}
            className={`ui-control-radius w-9 h-9 flex items-center justify-center p-0 transition-colors duration-75 border shrink-0 cursor-pointer ${
              currentTab === 'bin' && selectedBinId === bin.id
                ? 'sidebar-item-active'
                : hoveredControl === `bin:${bin.id}`
                ? 'sidebar-item-hovered border-transparent'
                : 'sidebar-item-idle border-transparent'
            }`}
            title={bin.name}
          >
            <span className="text-sm">{formatEmojiIcon(bin.icon)}</span>
          </button>
        ))}
        <div className="w-full flex items-center justify-center py-1 shrink-0"><div className="w-8 border-t sidebar-divider" /></div>
        {toolNavItems.map((item) => (
          <button
            key={item.tab}
            data-sidebar-hover-key={`tool:${item.tab}`}
            onClick={() => navigateTo(item.tab)}
            disabled={isClipDragging}
            className={`ui-control-radius w-9 h-9 flex items-center justify-center p-0 transition-colors duration-75 border shrink-0 cursor-pointer ${
              currentTab === item.tab
                ? 'sidebar-item-active cursor-pointer'
                : hoveredControl === `tool:${item.tab}`
                ? 'sidebar-item-hovered border-transparent cursor-pointer'
                : 'sidebar-item-idle border-transparent cursor-pointer'
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
