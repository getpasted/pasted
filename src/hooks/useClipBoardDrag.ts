import { useCallback, useMemo, useState, type Dispatch, type SetStateAction } from 'react';
import type { Board, ClipItem } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { soundManager } from '../utils/sound';

interface ClipDragPreview {
  clipId: number;
  x: number;
  y: number;
}

interface ClipBoardDragInput {
  allClips: ClipItem[];
  setAllClips: Dispatch<SetStateAction<ClipItem[]>>;
  boards: Board[];
  setBoards: Dispatch<SetStateAction<Board[]>>;
  selectedClipIds: Set<number>;
  enableSounds: boolean;
  fetchBoards: () => Promise<void>;
  fetchClips: () => Promise<void>;
}

export function useClipBoardDrag({
  allClips,
  setAllClips,
  boards,
  setBoards,
  selectedClipIds,
  enableSounds,
  fetchBoards,
  fetchClips,
}: ClipBoardDragInput) {
  const [draggedClipId, setDraggedClipId] = useState<number | null>(null);
  const [pointerDropTargetBoardId, setPointerDropTargetBoardId] = useState<number | null>(null);
  const [clipDragPreview, setClipDragPreview] = useState<ClipDragPreview | null>(null);

  const disabledDropBoardId = useMemo(() => {
    if (draggedClipId === null) return null;
    const draggedIds = selectedClipIds.size > 1 && selectedClipIds.has(draggedClipId)
      ? Array.from(selectedClipIds)
      : [draggedClipId];
    const draggedClips = allClips.filter((clip) => draggedIds.includes(clip.id));
    if (draggedClips.length !== draggedIds.length) return null;
    const currentBoardId = draggedClips[0]?.board_id ?? null;
    if (currentBoardId === null || !draggedClips.every((clip) => clip.board_id === currentBoardId)) {
      return null;
    }
    return boards.find((board) => board.id === currentBoardId && board.board_type !== 'tag')
      ? currentBoardId
      : null;
  }, [allClips, boards, draggedClipId, selectedClipIds]);

  const getPointerDropTarget = useCallback((x: number, y: number) => {
    const target = document
      .elementFromPoint(x, y)
      ?.closest<HTMLElement>('[data-bin-drop-board-id]');
    if (!target) return null;
    const boardId = Number(target.dataset.binDropBoardId);
    return Number.isInteger(boardId) && boardId > 0 ? boardId : null;
  }, []);

  const assignClipToBoard = useCallback(async (clipId: number, boardId: number) => {
    const isBatch = selectedClipIds.size > 1 && selectedClipIds.has(clipId);
    const targetIds = isBatch ? Array.from(selectedClipIds) : [clipId];
    const targetClips = allClips.filter((clip) => targetIds.includes(clip.id));
    const categoryBoardIds = new Set(
      boards.filter((board) => board.board_type !== 'tag').map((board) => board.id)
    );

    setAllClips((previous) => previous.map((clip) => {
      if (!targetIds.includes(clip.id)) return clip;
      const tagIds = (clip.board_ids || []).filter((id) => !categoryBoardIds.has(id));
      return { ...clip, board_id: boardId, board_ids: [...tagIds, boardId] };
    }));

    setBoards((previous) => previous.map((board) => {
      if (board.board_type === 'tag') return board;
      let delta = 0;
      for (const clip of targetClips) {
        const oldBinIds = new Set([
          ...(clip.board_ids || []).filter((id) => categoryBoardIds.has(id)),
          ...(clip.board_id && categoryBoardIds.has(clip.board_id) ? [clip.board_id] : []),
        ]);
        if (oldBinIds.has(board.id) && board.id !== boardId) delta -= 1;
        if (board.id === boardId && !oldBinIds.has(boardId)) delta += 1;
      }
      return delta === 0
        ? board
        : { ...board, clip_count: Math.max(0, (board.clip_count || 0) + delta) };
    }));

    soundManager.playCopySound(enableSounds);

    try {
      if (isBatch) {
        await invoke('batch_assign_board_clips', { ids: targetIds, boardId });
      } else {
        await invoke('assign_clip_board', { clipId, boardId });
      }
      void fetchBoards();
      void fetchClips();
    } catch (error) {
      console.error('Failed to assign clip to board:', error);
      void fetchClips();
      void fetchBoards();
    }
  }, [allClips, boards, enableSounds, fetchBoards, fetchClips, selectedClipIds, setAllClips, setBoards]);

  const finishClipPointerDrag = useCallback(async (x: number, y: number, clipId: number) => {
    const boardId = getPointerDropTarget(x, y);
    setPointerDropTargetBoardId(null);
    setClipDragPreview(null);

    if (boardId !== null) {
      await assignClipToBoard(clipId, boardId);
      return;
    }

    const target = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-clip-id]');
    const targetId = Number(target?.dataset.clipId);
    if (!Number.isInteger(targetId) || targetId === clipId) return;

    const pinnedClips = allClips.filter((clip) => clip.is_pinned);
    const draggedIndex = pinnedClips.findIndex((clip) => clip.id === clipId);
    const targetIndex = pinnedClips.findIndex((clip) => clip.id === targetId);
    if (draggedIndex === -1 || targetIndex === -1) return;

    const reordered = [...pinnedClips];
    const [moved] = reordered.splice(draggedIndex, 1);
    reordered.splice(targetIndex, 0, moved);
    setAllClips([...reordered, ...allClips.filter((clip) => !clip.is_pinned)]);

    try {
      await invoke('reorder_pinned_clips', { ids: reordered.map((clip) => clip.id) });
    } catch (error) {
      console.error('Failed to save pin order:', error);
      void fetchClips();
    }
  }, [allClips, assignClipToBoard, fetchClips, getPointerDropTarget, setAllClips]);

  return {
    draggedClipId,
    setDraggedClipId,
    pointerDropTargetBoardId,
    setPointerDropTargetBoardId,
    clipDragPreview,
    setClipDragPreview,
    disabledDropBoardId,
    getPointerDropTarget,
    assignClipToBoard,
    finishClipPointerDrag,
  };
}
