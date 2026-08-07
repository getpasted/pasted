import type { ReactNode } from 'react';
import { startWindowDrag } from '../utils/windowDrag';
import { OverflowText } from './OverflowText';

interface ToolPageHeaderProps {
  icon: ReactNode;
  title: string;
  description?: string;
  actions?: ReactNode;
}

export function ToolPageHeader({ icon, title, description, actions }: ToolPageHeaderProps) {
  return (
    <header
      className="theme-toolbar tool-page-header h-[60px] border-b px-6 flex items-center justify-between gap-4 shrink-0 titlebar-drag-handle"
      onMouseDown={startWindowDrag}
    >
      <div className="flex min-w-0 items-center gap-3">
        <span className="tool-page-header-icon flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border">
          {icon}
        </span>
        <div className="flex min-w-0 items-baseline gap-3">
          <h1 className="theme-title shrink-0 text-sm font-bold">{title}</h1>
          {description && (
            <OverflowText as="p" text={description} className="theme-text-muted min-w-0 truncate text-xs" />
          )}
        </div>
      </div>
      {actions && <div className="titlebar-no-drag shrink-0">{actions}</div>}
    </header>
  );
}
