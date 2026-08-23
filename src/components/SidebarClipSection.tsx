import React from 'react';
import { PanelLeftClose } from 'lucide-react';

import { translate } from '../localization/runtime';
import type { SequentialStatus } from '../types';
import type { ClipDropAction } from '../utils/clipCollections';
import { OverflowText } from './OverflowText';
import type { SidebarNavItem } from './sidebarNavigationModel';

interface SidebarClipSectionProps {
  items: SidebarNavItem[];
  counts: Record<string, number>;
  currentTab: string;
  selectedBinId: number | null;
  seqStatus: SequentialStatus | null;
  isOpen: boolean;
  isClipDragging: boolean;
  disabledDropActions: ClipDropAction[];
  pointerDropTargetAction?: ClipDropAction | null;
  hoveredControl: string | null;
  setIsOpen: (open: boolean) => void;
  setIsCollapsed: (collapsed: boolean) => void;
  navigateTo: (tab: string) => void;
  getDropActionTitle: (action: ClipDropAction) => string;
}

export function SidebarClipSection({
  items,
  counts,
  currentTab,
  selectedBinId,
  seqStatus,
  isOpen,
  isClipDragging,
  disabledDropActions,
  pointerDropTargetAction,
  hoveredControl,
  setIsOpen,
  setIsCollapsed,
  navigateTo,
  getDropActionTitle,
}: SidebarClipSectionProps) {
  return (
    <div>
      <div
        data-sidebar-hover-key="section:clips"
        onClick={isClipDragging ? undefined : () => setIsOpen(!isOpen)}
        className={`px-2.5 pb-1 flex items-center justify-between select-none ${isClipDragging ? 'cursor-default' : 'cursor-pointer'}`}
        title={translate('component.sidebar.toggleClips')}
      >
        <span className={`sidebar-section-label text-[11px] font-semibold transition-colors tracking-tight ${hoveredControl === 'section:clips' ? 'is-hovered' : ''}`}>
          {translate('component.sidebar.clips')}
        </span>
        <button
          data-sidebar-hover-key="collapse-framed"
          onClick={(event) => {
            event.stopPropagation();
            setIsCollapsed(true);
          }}
          disabled={isClipDragging}
          className={`platform-framed-only sidebar-control-muted h-7 w-7 items-center justify-center rounded-lg transition-colors titlebar-no-drag ${isClipDragging ? 'cursor-default' : `cursor-pointer ${hoveredControl === 'collapse-framed' ? 'sidebar-item-hovered' : ''}`}`}
          title={translate('component.sidebar.collapseSidebar')}
        >
          <PanelLeftClose className="h-4 w-4 rtl:-scale-x-100" />
        </button>
      </div>
      <div className={`transition-[background-color,border-color,color,opacity,transform] duration-150 ease-in-out ${isOpen ? 'max-h-96 opacity-100 mt-0 overflow-visible' : 'max-h-0 opacity-0 overflow-hidden'}`}>
        <nav className="space-y-0.5">
          {items.map((item) => {
            const count = counts[item.tab];
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
                className={`sidebar-nav-row justify-between transition-colors duration-100 ${isActionTarget ? `sidebar-action-drop sidebar-action-drop-${item.dropAction} sidebar-action-drop-target cursor-grabbing` : isEligibleAction ? `sidebar-action-drop sidebar-action-drop-${item.dropAction} sidebar-action-drop-eligible cursor-grabbing` : isClipDragging ? 'sidebar-action-drop-ineligible cursor-default' : currentTab === item.tab && (item.tab !== 'all' || selectedBinId === null) ? 'sidebar-item-active font-medium' : hoveredControl === `clip:${item.tab}` ? 'sidebar-item-hovered font-normal' : 'sidebar-item-idle font-normal cursor-pointer'}`}
              >
                <div className="flex items-center gap-3 min-w-0">
                  <span className="sidebar-nav-icon">{React.cloneElement(item.icon, { className: item.icon.props.className.replace('w-5 h-5', 'w-4 h-4 shrink-0'), strokeWidth: 1.8 })}</span>
                  <OverflowText text={item.label} className="truncate" />
                </div>
                {item.tab === 'sequential' && seqStatus?.is_active ? (
                  <span className="theme-status-success-dot w-2 h-2 rounded-full animate-pulse" />
                ) : count > 0 ? (
                  <span className={`sidebar-badge text-[11px] px-1.5 py-0.5 rounded-md font-mono ${item.tab === 'trash' ? 'is-danger' : ''}`}>{count}</span>
                ) : null}
              </button>
            );
          })}
        </nav>
      </div>
    </div>
  );
}
