import { useCallback, useEffect, useState } from 'react';
import type { ClearHistoryMode } from '../components/ClearHistoryDialog';
import type { Bin, ClipItem } from '../types';
import { useClipMetadataPrompts } from './useClipMetadataPrompts';

export interface ClipContextMenuState {
  x: number;
  y: number;
  clip: ClipItem;
}

export interface BinContextMenuState {
  x: number;
  y: number;
  bin: Bin;
}

interface UseAppOverlaysOptions {
  binsEnabled: boolean;
  notesEnabled: boolean;
  namingEnabled: boolean;
}

export function useAppOverlays({ binsEnabled, notesEnabled, namingEnabled }: UseAppOverlaysOptions) {
  const [contextMenu, setContextMenu] = useState<ClipContextMenuState | null>(null);
  const [binContextMenu, setBinContextMenu] = useState<BinContextMenuState | null>(null);
  const [isBinModalOpen, setIsBinModalOpen] = useState(false);
  const [editingBin, setEditingBin] = useState<Bin | null>(null);
  const [binToDelete, setBinToDelete] = useState<Bin | null>(null);
  const metadataPrompts = useClipMetadataPrompts(notesEnabled, namingEnabled);
  const { notePromptClip, setNotePromptClip, namePromptClip, setNamePromptClip } = metadataPrompts;
  const [clearHistoryMode, setClearHistoryMode] = useState<ClearHistoryMode | null>(null);

  useEffect(() => {
    if (!binsEnabled) {
      setIsBinModalOpen(false);
      setEditingBin(null);
      setBinToDelete(null);
      setBinContextMenu(null);
    }
  }, [binsEnabled]);

  useEffect(() => {
    const handleGlobalKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      const closeTopmostOverlay = namePromptClip
        ? () => setNamePromptClip(null)
        : notePromptClip
        ? () => setNotePromptClip(null)
        : binToDelete
          ? () => setBinToDelete(null)
          : clearHistoryMode
            ? () => setClearHistoryMode(null)
            : binContextMenu
              ? () => setBinContextMenu(null)
              : contextMenu
                ? () => setContextMenu(null)
                : isBinModalOpen
                  ? () => {
                      setIsBinModalOpen(false);
                      setEditingBin(null);
                    }
                  : null;
      if (!closeTopmostOverlay) return;
      event.preventDefault();
      event.stopPropagation();
      closeTopmostOverlay();
    };
    window.addEventListener('keydown', handleGlobalKeyDown, true);
    return () => window.removeEventListener('keydown', handleGlobalKeyDown, true);
  }, [binContextMenu, binToDelete, clearHistoryMode, contextMenu, isBinModalOpen, namePromptClip, notePromptClip]);

  useEffect(() => {
    const preventNativeContextMenu = (event: MouseEvent) => event.preventDefault();
    window.addEventListener('contextmenu', preventNativeContextMenu);
    return () => window.removeEventListener('contextmenu', preventNativeContextMenu);
  }, []);

  const openNewBinModal = useCallback(() => {
    setEditingBin(null);
    setIsBinModalOpen(true);
  }, []);
  const editBin = useCallback((bin: Bin) => {
    setEditingBin(bin);
    setIsBinModalOpen(true);
  }, []);
  const closeBinModal = useCallback(() => {
    setIsBinModalOpen(false);
    setEditingBin(null);
  }, []);
  const openBinContextMenu = useCallback((x: number, y: number, bin: Bin) => {
    setBinContextMenu({ x, y, bin });
  }, []);
  return {
    ...metadataPrompts,
    contextMenu,
    setContextMenu,
    binContextMenu,
    setBinContextMenu,
    isBinModalOpen,
    setIsBinModalOpen,
    editingBin,
    setEditingBin,
    binToDelete,
    setBinToDelete,
    clearHistoryMode,
    setClearHistoryMode,
    openNewBinModal,
    editBin,
    closeBinModal,
    openBinContextMenu,
  };
}
