import { PanelLeftOpen } from 'lucide-react';
import { translate } from '../localization/runtime';
import { handleWindowDragDoubleClick, startWindowDrag } from '../utils/windowDrag';

interface CollapsedSidebarHeaderProps {
  disabled: boolean;
  hovered: boolean;
  onExpand: () => void;
}

export function CollapsedSidebarHeader({
  disabled,
  hovered,
  onExpand,
}: CollapsedSidebarHeaderProps) {
  return (
    <div
      onMouseDown={startWindowDrag}
      onDoubleClick={handleWindowDragDoubleClick}
      className="platform-sidebar-header h-[56px] w-full cursor-default titlebar-drag-handle shrink-0"
    >
      <button
        data-sidebar-hover-key="expand-header"
        onClick={onExpand}
        disabled={disabled}
        aria-label={translate('component.sidebar.expandSidebar')}
        className={`platform-framed-only sidebar-control-muted ui-control-radius w-9 h-9 items-center justify-center p-0 transition-colors duration-75 border titlebar-no-drag ${disabled ? 'border-transparent cursor-default' : `cursor-pointer ${hovered ? 'sidebar-item-hovered' : 'border-transparent'}`}`}
        title={translate('component.sidebar.expandSidebar')}
      >
        <PanelLeftOpen className="h-5 w-5 rtl:-scale-x-100" />
      </button>
    </div>
  );
}
