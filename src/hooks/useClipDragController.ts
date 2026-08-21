import { useCallback, useEffect, useRef, useState, type Dispatch, type SetStateAction } from 'react';
import type { Bin, ClipItem } from '../types';
import type { ClipDropAction } from '../utils/clipCollections';
import { getClipPropertyAssociationForDropAction } from '../utils/clipPropertyAssociations';
import { useClipBinDrag } from './useClipBinDrag';

interface UseClipDragControllerOptions {
  isQueueCollection: boolean;
  allClips: ClipItem[];
  setAllClips: Dispatch<SetStateAction<ClipItem[]>>;
  bins: Bin[];
  selectedClipIds: Set<number>;
  fetchClips: () => Promise<void>;
  binsEnabled: boolean;
  queueEnabled: boolean;
  pinningEnabled: boolean;
  protectionEnabled: boolean;
  concealmentEnabled: boolean;
  assignClipToBin: (
    clipId: number,
    binId: number | null,
    options?: { includeSelection?: boolean; playSound?: boolean },
  ) => Promise<void>;
  addToQueue: (clip: ClipItem) => unknown;
  setPinned: (clipId: number, pinned: boolean) => unknown;
  setProtected: (clipId: number, protectedState: boolean) => unknown;
  setConcealed: (clipId: number, concealedState: boolean) => unknown;
  deleteClip: (clipId: number) => unknown;
}

export function useClipDragController({
  isQueueCollection,
  allClips,
  setAllClips,
  bins,
  selectedClipIds,
  fetchClips,
  binsEnabled,
  queueEnabled,
  pinningEnabled,
  protectionEnabled,
  concealmentEnabled,
  assignClipToBin,
  addToQueue,
  setPinned,
  setProtected,
  setConcealed,
  deleteClip,
}: UseClipDragControllerOptions) {
  const [hoveredClipId, setHoveredClipId] = useState<number | null>(null);
  const assignDraggedClipToBin = useCallback(async (clipId: number, binId: number) => {
    if (!binsEnabled) return;
    await assignClipToBin(clipId, binId, { includeSelection: true, playSound: true });
  }, [assignClipToBin, binsEnabled]);
  const assignClipToBinRef = useRef(assignDraggedClipToBin);
  assignClipToBinRef.current = assignDraggedClipToBin;
  const assignSidebarDropToBin = useCallback((clipId: number, binId: number) => (
    assignClipToBinRef.current(clipId, binId)
  ), []);

  const applyClipDropAction = useCallback((clipId: number, action: ClipDropAction) => {
    if (action === 'queue') {
      if (!queueEnabled) return;
      const clip = allClips.find((item) => item.id === clipId);
      if (clip) void addToQueue(clip);
    } else {
      const association = getClipPropertyAssociationForDropAction(action);
      if (association) {
        const handlers = {
          pin: { enabled: pinningEnabled, set: setPinned },
          protect: { enabled: protectionEnabled, set: setProtected },
          conceal: { enabled: concealmentEnabled, set: setConcealed },
        } as const;
        const handler = handlers[association.dropAction!];
        if (handler.enabled) void handler.set(clipId, true);
      } else {
        void deleteClip(clipId);
      }
    }
  }, [addToQueue, allClips, concealmentEnabled, deleteClip, pinningEnabled, protectionEnabled, queueEnabled, setConcealed, setPinned, setProtected]);

  const drag = useClipBinDrag({
    isQueueMode: isQueueCollection,
    allClips,
    setAllClips,
    bins,
    selectedClipIds,
    fetchClips,
    assignClipToBin: assignDraggedClipToBin,
    applyClipDropAction,
  });

  useEffect(() => {
    if (drag.draggedClipId !== null) setHoveredClipId(null);
    const updateHoveredClip = (event: PointerEvent) => {
      if (drag.draggedClipId !== null) {
        setHoveredClipId((current) => current === null ? current : null);
        return;
      }
      const card = document.elementFromPoint(event.clientX, event.clientY)
        ?.closest<HTMLElement>('[data-clip-id]');
      const candidateId = Number(card?.dataset.clipId);
      const nextId = Number.isInteger(candidateId) && candidateId > 0 ? candidateId : null;
      setHoveredClipId((current) => current === nextId ? current : nextId);
    };
    const clearHoveredClipOutsideWindow = (event: PointerEvent) => {
      if (!event.relatedTarget) setHoveredClipId(null);
    };
    window.addEventListener('pointermove', updateHoveredClip, { passive: true });
    window.addEventListener('pointerout', clearHoveredClipOutsideWindow);
    return () => {
      window.removeEventListener('pointermove', updateHoveredClip);
      window.removeEventListener('pointerout', clearHoveredClipOutsideWindow);
    };
  }, [drag.draggedClipId]);

  return { ...drag, hoveredClipId, setHoveredClipId, assignSidebarDropToBin };
}
