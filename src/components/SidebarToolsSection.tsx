import React from 'react';

import { translate } from '../localization/runtime';
import { OverflowText } from './OverflowText';
import type { SidebarNavItem } from './sidebarNavigationModel';

interface SidebarToolsSectionProps {
  items: SidebarNavItem[];
  currentTab: string;
  isOpen: boolean;
  isClipDragging: boolean;
  hoveredControl: string | null;
  setIsOpen: (open: boolean) => void;
  navigateTo: (tab: string) => void;
}

export function SidebarToolsSection({ items, currentTab, isOpen, isClipDragging, hoveredControl, setIsOpen, navigateTo }: SidebarToolsSectionProps) {
  return (
    <div>
      <div
        data-sidebar-hover-key="section:tools"
        onClick={isClipDragging ? undefined : () => setIsOpen(!isOpen)}
        className={`px-2.5 pb-1 flex items-center justify-between select-none ${isClipDragging ? 'cursor-default' : 'cursor-pointer'}`}
        title={translate('component.sidebar.toggleTools')}
      >
        <span className={`sidebar-section-label text-[11px] font-semibold transition-colors tracking-tight ${hoveredControl === 'section:tools' ? 'is-hovered' : ''}`}>
          {translate('component.sidebar.tools')}
        </span>
      </div>
      <div className={`transition-[background-color,border-color,color,opacity,transform] duration-150 ease-in-out ${isOpen ? 'max-h-96 opacity-100 mt-0 overflow-visible' : 'max-h-0 opacity-0 overflow-hidden'}`}>
        <nav className="space-y-0.5">
          {items.map((item) => (
            <button
              key={item.tab}
              data-sidebar-hover-key={`tool:${item.tab}`}
              onClick={() => navigateTo(item.tab)}
              disabled={isClipDragging}
              className={`sidebar-nav-row gap-3 transition-colors duration-100 cursor-pointer ${currentTab === item.tab ? 'sidebar-item-active font-medium' : hoveredControl === `tool:${item.tab}` ? 'sidebar-item-hovered font-normal' : 'sidebar-item-idle font-normal'}`}
            >
              <span className="sidebar-nav-icon">{React.cloneElement(item.icon, { className: item.icon.props.className.replace('w-5 h-5', 'w-4 h-4 shrink-0'), strokeWidth: 1.8 })}</span>
              <OverflowText text={item.label} className="truncate" />
            </button>
          ))}
        </nav>
      </div>
    </div>
  );
}
