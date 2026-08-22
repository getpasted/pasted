import React from 'react';
import { ChevronUp, X } from 'lucide-react';

import { translate } from '../localization/runtime';
import type { FeatureId } from '../utils/features';
import { EmbeddedMenu, MenuItem } from './AnchoredMenu';

const SEARCH_HELPERS = [
  { prefix: 'regex:', get desc() { return translate('component.sidebar.regex'); } },
  { prefix: 'clip:', get desc() { return translate('component.sidebar.clipTypes'); } },
  { prefix: 'content:', get desc() { return translate('component.sidebar.contentTypes'); } },
  { prefix: 'format:', get desc() { return translate('component.sidebar.fileFormats'); } },
  { prefix: 'source:', get desc() { return translate('component.sidebar.sources'); } },
  { prefix: 'has:note', get desc() { return translate('feature.notes.label'); } },
  { prefix: 'has:name', get desc() { return translate('feature.naming.label'); } },
  { prefix: 'is:pinned', get desc() { return translate('collection.pinned'); } },
  { prefix: 'is:protected', get desc() { return translate('collection.protected'); } },
  { prefix: 'is:trashed', get desc() { return translate('collection.trashed'); } },
] as const;

interface SidebarSearchFooterProps {
  features: Record<FeatureId, boolean>;
  isDragActive: boolean;
  searchQuery: string;
  setSearchQuery: (query: string) => void;
  onSearchFocus: () => void;
  onEmptySearchEscape: () => void;
}

export function SidebarSearchFooter({
  features,
  isDragActive,
  searchQuery,
  setSearchQuery,
  onSearchFocus,
  onEmptySearchEscape,
}: SidebarSearchFooterProps) {
  const [isOpen, setIsOpen] = React.useState(false);
  const [activeIndex, setActiveIndex] = React.useState(-1);
  const rootRef = React.useRef<HTMLDivElement | null>(null);
  const inputRef = React.useRef<HTMLInputElement | null>(null);
  const itemRefs = React.useRef<Array<HTMLButtonElement | null>>([]);
  const helpers = React.useMemo(
    () => SEARCH_HELPERS.filter(({ prefix }) => {
      if (prefix === 'clip:') return features.clipTypes;
      if (prefix === 'content:') return features.types;
      if (prefix === 'format:') return features.fileFormats;
      if (prefix === 'source:') return features.sources;
      if (prefix === 'has:note') return features.notes;
      if (prefix === 'is:pinned') return features.pinning;
      if (prefix === 'is:protected') return features.protection;
      if (prefix === 'is:trashed') return features.trash;
      return true;
    }),
    [features.clipTypes, features.fileFormats, features.notes, features.pinning, features.protection, features.sources, features.trash, features.types],
  );

  const close = React.useCallback((returnFocus = false) => {
    setIsOpen(false);
    setActiveIndex(-1);
    if (returnFocus) requestAnimationFrame(() => inputRef.current?.focus());
  }, []);

  const focusItem = (index: number) => {
    const normalizedIndex = (index + helpers.length) % helpers.length;
    setActiveIndex(normalizedIndex);
    requestAnimationFrame(() => itemRefs.current[normalizedIndex]?.focus());
  };

  React.useEffect(() => {
    if (!isOpen) return undefined;
    const closeOutside = (event: PointerEvent | FocusEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) close();
    };
    document.addEventListener('pointerdown', closeOutside);
    document.addEventListener('focusin', closeOutside);
    return () => {
      document.removeEventListener('pointerdown', closeOutside);
      document.removeEventListener('focusin', closeOutside);
    };
  }, [close, isOpen]);

  React.useEffect(() => {
    const focusSearch = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== 'f') return;
      event.preventDefault();
      close();
      onSearchFocus();
      requestAnimationFrame(() => {
        inputRef.current?.focus();
        inputRef.current?.select();
      });
    };
    window.addEventListener('keydown', focusSearch);
    return () => window.removeEventListener('keydown', focusSearch);
  }, [close, onSearchFocus]);

  React.useEffect(() => {
    if (isDragActive) close();
  }, [close, isDragActive]);

  const choose = (prefix: string) => {
    setSearchQuery(prefix);
    close(true);
  };

  return (
    <div ref={rootRef} className="sidebar-divider h-[55px] px-2.5 border-t shrink-0 relative flex items-center">
      {!isDragActive && isOpen && (
        <EmbeddedMenu id="sidebar-search-filters" ariaLabel={translate('component.sidebar.searchFilters')} className="absolute inset-x-2.5 bottom-12">
          {helpers.map((helper, index) => (
            <MenuItem
              ref={(element) => { itemRefs.current[index] = element; }}
              key={helper.prefix}
              active={activeIndex === index}
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => choose(helper.prefix)}
              onFocus={() => setActiveIndex(index)}
              onKeyDown={(event) => {
                if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
                  event.preventDefault();
                  focusItem(index + (event.key === 'ArrowDown' ? 1 : -1));
                } else if (event.key === 'Home' || event.key === 'End') {
                  event.preventDefault();
                  focusItem(event.key === 'Home' ? 0 : helpers.length - 1);
                } else if (event.key === 'Escape') {
                  event.preventDefault();
                  event.stopPropagation();
                  close(true);
                } else if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault();
                  choose(helper.prefix);
                }
              }}
              className="cursor-pointer justify-between gap-3 px-2.5 py-1.5"
            >
              <span className="font-mono text-[11px] font-semibold">{helper.prefix}</span>
              <span className="theme-text-subtle text-[10px]">{helper.desc}</span>
            </MenuItem>
          ))}
        </EmbeddedMenu>
      )}
      <div className="relative w-full titlebar-no-drag">
        <input
          ref={inputRef}
          data-sidebar-search-input
          type="text"
          disabled={isDragActive}
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck={false}
          placeholder={translate('component.sidebar.searchAllClips')}
          value={searchQuery}
          onFocus={onSearchFocus}
          onKeyDown={(event) => {
            if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
              event.preventDefault();
              if (!isOpen) setIsOpen(true);
              focusItem(event.key === 'ArrowDown' ? 0 : helpers.length - 1);
            } else if (event.key === 'Escape') {
              event.preventDefault();
              if (isOpen) close();
              else {
                event.currentTarget.blur();
                if (!searchQuery.trim()) onEmptySearchEscape();
              }
            }
          }}
          onChange={(event) => setSearchQuery(event.target.value)}
          className={`sidebar-search-input theme-input ui-field-radius h-[34px] w-full border ps-2.5 ${searchQuery ? 'pe-16' : 'pe-10'} text-xs focus:outline-none transition-colors titlebar-no-drag`}
        />
        {searchQuery && (
          <button
            type="button"
            disabled={isDragActive}
            aria-label={translate('component.sidebar.clearSearch')}
            title={translate('component.sidebar.clearSearch2')}
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => {
              setSearchQuery('');
              close();
              onSearchFocus();
              requestAnimationFrame(() => inputRef.current?.focus());
            }}
            className="sidebar-search-clear theme-menu-item absolute end-8 top-1/2 grid h-6 w-6 -translate-y-1/2 place-items-center rounded-md"
          >
            <X className="h-3.5 w-3.5" aria-hidden="true" />
          </button>
        )}
        <button
          type="button"
          disabled={isDragActive}
          aria-label={translate('component.sidebar.searchFilters')}
          aria-haspopup="menu"
          aria-expanded={isOpen}
          aria-controls="sidebar-search-filters"
          title={translate('component.sidebar.searchFilters2')}
          onClick={() => {
            onSearchFocus();
            setIsOpen((open) => {
              if (open) setActiveIndex(-1);
              return !open;
            });
            requestAnimationFrame(() => inputRef.current?.focus());
          }}
          onKeyDown={(event) => {
            if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
              event.preventDefault();
              setIsOpen(true);
              focusItem(event.key === 'ArrowDown' ? 0 : helpers.length - 1);
            } else if (event.key === 'Escape' && isOpen) {
              event.preventDefault();
              close();
            }
          }}
          className={`theme-menu-item absolute end-1 top-1/2 grid h-6 w-6 -translate-y-1/2 place-items-center rounded-md ${isOpen ? 'is-selected' : ''}`}
        >
          <ChevronUp className={`h-3.5 w-3.5 transition-transform ${isOpen ? 'rotate-180' : ''}`} aria-hidden="true" />
        </button>
      </div>
    </div>
  );
}
