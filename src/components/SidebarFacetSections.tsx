import {
  AppWindow,
  Camera,
  CircleHelp,
  Clipboard,
  Files,
  FileText,
  FileType2,
  Globe2,
  Image as ImageIcon,
  MonitorCog,
  Smartphone,
  TerminalSquare,
} from 'lucide-react';

import { translate } from '../localization/runtime';
import type { ClipContentType } from '../types';
import type { SidebarFacetItem } from '../hooks/useSidebarFacets';
import { ContentTypeIcon } from './ContentTypeIcon';
import { OverflowText } from './OverflowText';
import { SafeRasterImage } from './SafeRasterImage';

export type SidebarFacetSectionId = 'clipTypes' | 'types' | 'fileFormats' | 'sources';

export interface SidebarFacetSection {
  id: SidebarFacetSectionId;
  label: string;
  open: boolean;
  setOpen: (open: boolean) => void;
  items: SidebarFacetItem[];
}

function sourceFallbackIcon(source: string | null | undefined) {
  const normalized = source?.trim().toLowerCase() ?? '';
  const className = 'sidebar-icon-primary h-4 w-4 shrink-0';
  if (normalized.includes('screenshot') || normalized.includes('screencapture')) return <Camera className={className} strokeWidth={1.8} />;
  if (normalized === 'system clipboard' || normalized === 'clipboard') return <Clipboard className={className} strokeWidth={1.8} />;
  if (normalized === 'continuity' || normalized === 'universal clipboard') return <Smartphone className={className} strokeWidth={1.8} />;
  if (normalized === 'macos system' || normalized === 'windows system' || normalized === 'linux system' || normalized === 'system') return <MonitorCog className={className} strokeWidth={1.8} />;
  if (normalized === 'browser' || normalized === 'web browser') return <Globe2 className={className} strokeWidth={1.8} />;
  if (normalized.includes('terminal') || normalized === 'pasted cli') return <TerminalSquare className={className} strokeWidth={1.8} />;
  if (!normalized || normalized === 'unknown' || normalized === 'unknown source') return <CircleHelp className={className} strokeWidth={1.8} />;
  return <AppWindow className={className} strokeWidth={1.8} />;
}

interface SidebarFacetSectionsProps {
  sections: SidebarFacetSection[];
  sourceIcons: Record<string, string>;
  currentTab: string;
  isClipDragging: boolean;
  hoveredControl: string | null;
  navigateTo: (tab: string) => void;
}

export function SidebarFacetSections({ sections, sourceIcons, currentTab, isClipDragging, hoveredControl, navigateTo }: SidebarFacetSectionsProps) {
  return <>{sections.map((section) => (
    <div key={section.id}>
      <div
        data-sidebar-hover-key={`section:${section.id}`}
        onClick={isClipDragging ? undefined : () => section.setOpen(!section.open)}
        className={`px-2.5 pb-1 flex items-center justify-between select-none ${isClipDragging ? 'cursor-default' : 'cursor-pointer'}`}
        title={translate('component.sidebar.toggleLabel', { label: section.label })}
      >
        <span className={`sidebar-section-label text-[11px] font-semibold transition-colors tracking-tight ${hoveredControl === `section:${section.id}` ? 'is-hovered' : ''}`}>{section.label}</span>
      </div>
      <div className={`grid transition-[grid-template-rows,opacity] duration-150 ease-in-out ${section.open ? 'grid-rows-[1fr] opacity-100' : 'grid-rows-[0fr] opacity-0'}`}>
        <nav className="min-h-0 space-y-0.5 overflow-hidden">
          {section.items.map((item) => (
            <button
              key={item.route}
              data-sidebar-hover-key={`${section.id}:${item.route}`}
              onClick={() => navigateTo(item.route)}
              disabled={isClipDragging}
              className={`sidebar-nav-row justify-between gap-3 transition-colors duration-100 ${isClipDragging ? 'cursor-default opacity-50' : currentTab === item.route ? 'sidebar-item-active font-medium cursor-pointer' : hoveredControl === `${section.id}:${item.route}` ? 'sidebar-item-hovered font-normal cursor-pointer' : 'sidebar-item-idle font-normal cursor-pointer'}`}
            >
              <div className="flex min-w-0 items-center gap-3">
                <span className="sidebar-nav-icon">
                  {section.id === 'clipTypes'
                    ? item.value === 'text' ? <FileText className="sidebar-icon-primary h-4 w-4 shrink-0" /> : item.value === 'image' ? <ImageIcon className="sidebar-icon-primary h-4 w-4 shrink-0" /> : <Files className="sidebar-icon-primary h-4 w-4 shrink-0" />
                    : section.id === 'fileFormats' ? <FileType2 className="sidebar-icon-primary h-4 w-4 shrink-0" />
                    : section.id === 'types' ? <ContentTypeIcon type={item.value as ClipContentType} className="sidebar-icon-primary h-4 w-4 shrink-0" />
                    : sourceIcons[item.value] ? <SafeRasterImage source={sourceIcons[item.value]} alt="" className="h-4 w-4 shrink-0 object-contain" /> : sourceFallbackIcon(item.value)}
                </span>
                <OverflowText text={item.label} className="truncate" />
              </div>
              <span className="sidebar-badge rounded-md px-1.5 py-0.5 font-mono text-[11px]">{item.count}</span>
            </button>
          ))}
        </nav>
      </div>
    </div>
  ))}</>;
}
