import { useEffect, type Dispatch, type SetStateAction } from 'react';
import type { ClipItem } from '../types';
import { getClipCollection } from '../utils/clipCollections';
import { clipIdsForSelectAll, isSelectAllShortcut } from '../utils/clipSelection';
import { getClipViewPolicy } from '../utils/clipViewPolicy';

interface UseClipSelectionKeyboardOptions {
  currentTab: string;
  displayedClips: ClipItem[];
  selectedClip: ClipItem | null;
  setSelectedClip: Dispatch<SetStateAction<ClipItem | null>>;
  setSelectedClipIds: Dispatch<SetStateAction<Set<number>>>;
  setSelectedIndex: Dispatch<SetStateAction<number>>;
  setIsSidebarCollapsed: Dispatch<SetStateAction<boolean>>;
  copyClip: (clip: ClipItem) => unknown;
  deleteClip: (clipId: number) => unknown;
  purgeClipPermanently: (clipId: number) => unknown;
}

export function useClipSelectionKeyboard({
  currentTab,
  displayedClips,
  selectedClip,
  setSelectedClip,
  setSelectedClipIds,
  setSelectedIndex,
  setIsSidebarCollapsed,
  copyClip,
  deleteClip,
  purgeClipPermanently,
}: UseClipSelectionKeyboardOptions) {
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key === '\\') {
        event.preventDefault();
        setIsSidebarCollapsed((collapsed) => !collapsed);
        return;
      }
      if (event.defaultPrevented || !getClipCollection(currentTab)) return;
      const target = event.target;
      if (target instanceof HTMLElement && (
        ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)
        || target.isContentEditable
        || target.closest('[contenteditable]:not([contenteditable="false"])')
      )) return;
      if (displayedClips.length === 0) return;
      if (isSelectAllShortcut(event)) {
        event.preventDefault();
        setSelectedClipIds(clipIdsForSelectAll(displayedClips));
        if (!selectedClip) {
          setSelectedClip(displayedClips[0]);
          setSelectedIndex(0);
        }
        return;
      }
      if (event.key === 'Escape' && selectedClip) {
        event.preventDefault();
        setSelectedClip(null);
        setSelectedClipIds(new Set());
        setSelectedIndex(-1);
        return;
      }
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault();
        const direction = event.key === 'ArrowDown' ? 1 : -1;
        setSelectedIndex((previous) => {
          const next = Math.max(0, Math.min(previous + direction, displayedClips.length - 1));
          setSelectedClip(displayedClips[next]);
          setSelectedClipIds(new Set([displayedClips[next].id]));
          return next;
        });
      } else if (event.key === 'Enter' && selectedClip) {
        event.preventDefault();
        void copyClip(selectedClip);
      } else if ((event.key === 'Delete' || event.key === 'Backspace') && selectedClip) {
        event.preventDefault();
        if (getClipViewPolicy(currentTab, selectedClip).state === 'trash') void purgeClipPermanently(selectedClip.id);
        else void deleteClip(selectedClip.id);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [copyClip, currentTab, deleteClip, displayedClips, purgeClipPermanently, selectedClip, setIsSidebarCollapsed, setSelectedClip, setSelectedClipIds, setSelectedIndex]);
}
