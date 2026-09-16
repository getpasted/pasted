import { useCallback, useEffect, useRef, useState, type Dispatch, type SetStateAction } from 'react';
import type { Bin } from '../types';
import {
  isClipCollectionRoute,
  resolveSearchExit,
  type ClipViewLocation,
} from '../utils/appNavigation';
import {
  CLOSED_SEARCH_DIALOG,
  closeSearchDialog,
  commitSearchDialog,
  openSearchDialog,
  updateSearchDraft,
} from '../utils/searchDialog';

interface UseSearchDialogControllerOptions {
  enabled: boolean;
  bins: Bin[];
  currentTab: string;
  selectedBinId: number | null;
  initialClipView: ClipViewLocation;
  committedQuery: string;
  setCommittedQuery: Dispatch<SetStateAction<string>>;
  setCurrentTab: Dispatch<SetStateAction<string>>;
  setSelectedBinId: Dispatch<SetStateAction<number | null>>;
}

export function useSearchDialogController({
  enabled,
  bins,
  currentTab,
  selectedBinId,
  initialClipView,
  committedQuery,
  setCommittedQuery,
  setCurrentTab,
  setSelectedBinId,
}: UseSearchDialogControllerOptions) {
  const [dialog, setDialog] = useState(CLOSED_SEARCH_DIALOG);
  const lastClipViewRef = useRef(initialClipView);

  useEffect(() => {
    if (isClipCollectionRoute(currentTab)) {
      lastClipViewRef.current = { tab: currentTab, binId: currentTab === 'bin' ? selectedBinId : null };
    }
  }, [currentTab, selectedBinId]);

  const exitSearch = useCallback(() => {
    const target = resolveSearchExit(lastClipViewRef.current, new Set(bins.map(({ id }) => id)));
    setSelectedBinId(target.binId);
    setCurrentTab(target.tab);
  }, [bins, setCurrentTab, setSelectedBinId]);

  const open = useCallback(() => {
    if (!enabled || document.querySelector('[role="dialog"][aria-modal="true"]')) return;
    setDialog(openSearchDialog(committedQuery));
  }, [committedQuery, enabled]);
  const close = useCallback(() => setDialog(closeSearchDialog()), []);
  const clear = useCallback(() => setCommittedQuery(''), [setCommittedQuery]);
  const runSearch = useCallback((query: string) => {
    if (!enabled) return;
    setCommittedQuery(query);
    setSelectedBinId(null);
    setCurrentTab('search');
    setDialog(closeSearchDialog());
  }, [enabled, setCommittedQuery, setCurrentTab, setSelectedBinId]);
  const setDraft = useCallback((draft: string) => {
    setDialog((current) => updateSearchDraft(current, draft));
  }, []);
  const submit = useCallback(() => {
    runSearch(commitSearchDialog(dialog));
  }, [dialog, runSearch]);

  useEffect(() => {
    if (!enabled) {
      close();
      clear();
    }
  }, [clear, close, enabled]);

  useEffect(() => {
    const handleSearchShortcut = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        if (dialog.isOpen || currentTab !== 'search') return;
        queueMicrotask(() => {
          if (!event.defaultPrevented && !document.querySelector('[role="dialog"][aria-modal="true"]')) {
            if (committedQuery) clear();
            else exitSearch();
          }
        });
        return;
      }
      if (!enabled || !(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== 'f') return;
      const activeDialog = document.querySelector('[role="dialog"][aria-modal="true"]');
      if (activeDialog && !dialog.isOpen) return;
      event.preventDefault();
      if (!dialog.isOpen) open();
      requestAnimationFrame(() => {
        const input = document.querySelector<HTMLInputElement>('[data-search-dialog-input]');
        input?.focus();
        input?.select();
      });
    };
    window.addEventListener('keydown', handleSearchShortcut);
    return () => window.removeEventListener('keydown', handleSearchShortcut);
  }, [clear, committedQuery, currentTab, dialog.isOpen, enabled, exitSearch, open]);

  return {
    isSearchDialogOpen: dialog.isOpen,
    searchDraft: dialog.draft,
    setSearchDraft: setDraft,
    openSearchDialog: open,
    closeSearchDialog: close,
    clearSearch: clear,
    runSearch,
    submitSearchDialog: submit,
  };
}
