import { Search, X } from 'lucide-react';

import { translate } from '../localization/runtime';

interface SidebarSearchFooterProps {
  isDragActive: boolean;
  searchQuery: string;
  onOpenSearch: () => void;
  onClearSearch: () => void;
}

export function SidebarSearchFooter({
  isDragActive,
  searchQuery,
  onOpenSearch,
  onClearSearch,
}: SidebarSearchFooterProps) {
  return (
    <div className="sidebar-divider flex h-[55px] shrink-0 items-center border-t px-2.5">
      <div className="relative w-full titlebar-no-drag">
        <input
          data-sidebar-search-input
          type="text"
          readOnly
          disabled={isDragActive}
          aria-label={translate('component.sidebar.searchAllClips')}
          aria-haspopup="dialog"
          placeholder={translate('component.sidebar.searchAllClips')}
          value={searchQuery}
          onMouseDown={(event) => event.preventDefault()}
          onClick={onOpenSearch}
          onKeyDown={(event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault();
              onOpenSearch();
            }
          }}
          className={`sidebar-search-input theme-input ui-field-radius h-[34px] w-full cursor-pointer border ps-2.5 ${searchQuery ? 'pe-10' : 'pe-9'} text-xs transition-colors focus:outline-none titlebar-no-drag`}
        />
        {searchQuery && (
          <button
            type="button"
            disabled={isDragActive}
            aria-label={translate('component.sidebar.clearSearch')}
            title={translate('component.sidebar.clearSearch2')}
            onMouseDown={(event) => {
              event.preventDefault();
              event.stopPropagation();
            }}
            onClick={(event) => {
              event.stopPropagation();
              onClearSearch();
            }}
            className="sidebar-search-clear theme-menu-item absolute end-1 top-1/2 grid h-6 w-6 -translate-y-1/2 place-items-center rounded-md"
          >
            <X className="h-3.5 w-3.5" aria-hidden="true" />
          </button>
        )}
        {!searchQuery && <Search className="theme-text-muted pointer-events-none absolute end-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2" aria-hidden="true" />}
      </div>
    </div>
  );
}
