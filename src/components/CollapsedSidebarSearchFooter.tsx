import { Search } from 'lucide-react';
import { translate } from '../localization/runtime';

interface CollapsedSidebarSearchFooterProps {
  disabled: boolean;
  hovered: boolean;
  onOpen: () => void;
}

export function CollapsedSidebarSearchFooter({
  disabled,
  hovered,
  onOpen,
}: CollapsedSidebarSearchFooterProps) {
  return (
    <div className="sidebar-divider flex h-[55px] w-full shrink-0 items-center justify-center border-t">
      <button
        type="button"
        data-sidebar-hover-key="search"
        disabled={disabled}
        onClick={onOpen}
        aria-label={translate('component.sidebar.searchAllClips')}
        aria-haspopup="dialog"
        title={translate('component.sidebar.searchAllClips')}
        className={`sidebar-control-muted ui-control-radius grid h-9 w-9 place-items-center transition-colors ${disabled ? 'cursor-default' : `cursor-pointer ${hovered ? 'sidebar-item-hovered' : ''}`}`}
      >
        <Search className="h-4 w-4" aria-hidden="true" />
      </button>
    </div>
  );
}
