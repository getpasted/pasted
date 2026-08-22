import { useCallback, type Dispatch, type SetStateAction } from 'react';
import type { ClipItem } from '../types';
import { clipsApi } from '../api/clips';
import { sortClipsForTimeline } from '../utils/clipOrder';
import { safeInvoke as invoke } from '../utils/tauri';
import type { ClipPropertyAssociationId } from '../utils/clipPropertyAssociations';

interface ClipPropertyActionsInput {
  allClips: ClipItem[];
  setAllClips: Dispatch<SetStateAction<ClipItem[]>>;
  setSelectedClip: Dispatch<SetStateAction<ClipItem | null>>;
  selectedClipIds: Set<number>;
  fetchClips: () => Promise<void>;
  onCollectionChanged: () => Promise<void>;
  onClipsRepositioned?: (ids: number[]) => void;
  onClipPropertyRemoved?: (association: ClipPropertyAssociationId, ids: number[]) => void;
}

export function useClipPropertyActions({
  allClips,
  setAllClips,
  setSelectedClip,
  selectedClipIds,
  fetchClips,
  onCollectionChanged,
  onClipsRepositioned,
  onClipPropertyRemoved,
}: ClipPropertyActionsInput) {
  const togglePin = useCallback((id: number) => {
    const isBatch = selectedClipIds.size > 1 && selectedClipIds.has(id);
    const targetIds = isBatch ? Array.from(selectedClipIds) : [id];
    const nextPinState = !(allClips.find((clip) => clip.id === id)?.is_pinned ?? false);

    const targetIdSet = new Set(targetIds);
    const idsToChange = allClips
      .filter((clip) => targetIdSet.has(clip.id) && Boolean(clip.is_pinned) !== nextPinState)
      .map((clip) => clip.id);
    onClipsRepositioned?.(idsToChange);
    if (!nextPinState) onClipPropertyRemoved?.('pin', idsToChange);
    setAllClips((previous) => {
      const updated = previous.map((clip) => (
        targetIdSet.has(clip.id) ? { ...clip, is_pinned: nextPinState } : clip
      ));
      if (nextPinState) {
        const newlyPinned = updated
          .filter((clip) => targetIdSet.has(clip.id))
          .map((clip, index) => ({ ...clip, pin_order: index }));
        const existingPinned = updated
          .filter((clip) => clip.is_pinned && !targetIdSet.has(clip.id))
          .map((clip) => ({ ...clip, pin_order: (clip.pin_order ?? 0) + newlyPinned.length }));
        return sortClipsForTimeline([
          ...newlyPinned,
          ...existingPinned,
          ...updated.filter((clip) => !clip.is_pinned),
        ]);
      }
      return sortClipsForTimeline(updated);
    });
    setSelectedClip((previous) => previous && targetIds.includes(previous.id)
      ? { ...previous, is_pinned: nextPinState, pin_order: nextPinState ? 0 : previous.pin_order }
      : previous);

    const request = isBatch
      ? clipsApi.setPinned(targetIds, nextPinState)
      : clipsApi.togglePin(id);
    void request
      .then(onCollectionChanged)
      .catch((error) => {
        console.error('Failed to update pinned state:', error);
        void fetchClips();
      });
  }, [allClips, fetchClips, onClipPropertyRemoved, onClipsRepositioned, onCollectionChanged, selectedClipIds, setAllClips, setSelectedClip]);

  const toggleProtected = useCallback((id: number) => {
    const current = allClips.find((clip) => clip.id === id);
    const explicit = current?.is_explicitly_protected ?? current?.is_protected ?? false;
    if (!current || current.hotkey || current.protecting_bin_ids?.length) return;
    const nextExplicit = !explicit;
    if (!nextExplicit) onClipPropertyRemoved?.('protect', [id]);
    const update = (clip: ClipItem) => clip.id === id ? {
      ...clip,
      is_explicitly_protected: nextExplicit,
      is_protected: nextExplicit || Boolean(clip.protecting_bin_ids?.length),
    } : clip;
    setAllClips((previous) => previous.map((clip) => (
      update(clip)
    )));
    setSelectedClip((previous) => previous?.id === id
      ? update(previous)
      : previous);

    void invoke('toggle_clip_protected', { clipId: id })
      .then(onCollectionChanged)
      .catch((error) => {
        console.error('Failed to toggle protected state:', error);
        void fetchClips();
      });
  }, [allClips, fetchClips, onClipPropertyRemoved, onCollectionChanged, setAllClips, setSelectedClip]);

  const toggleConcealed = useCallback((id: number) => {
    const current = allClips.find((clip) => clip.id === id);
    if (!current || current.is_trashed) return;
    const nextConcealed = !Boolean(current.is_concealed);
    if (!nextConcealed) onClipPropertyRemoved?.('conceal', [id]);
    const update = (clip: ClipItem) => clip.id === id ? {
      ...clip,
      is_explicitly_concealed: nextConcealed,
      is_explicitly_revealed: !nextConcealed,
      is_concealed: nextConcealed,
    } : clip;
    setAllClips((previous) => previous.map(update));
    setSelectedClip((previous) => previous?.id === id ? update(previous) : previous);

    void invoke('toggle_clip_concealed', { clipId: id })
      .then(onCollectionChanged)
      .catch((error) => {
        console.error('Failed to toggle concealed state:', error);
        void fetchClips();
      });
  }, [allClips, fetchClips, onClipPropertyRemoved, onCollectionChanged, setAllClips, setSelectedClip]);

  const setPinned = useCallback((id: number, pinState: boolean) => {
    const targetIds = selectedClipIds.size > 1 && selectedClipIds.has(id)
      ? Array.from(selectedClipIds)
      : [id];
    const targetIdSet = new Set(targetIds);
    const idsToChange = allClips
      .filter((clip) => targetIdSet.has(clip.id) && Boolean(clip.is_pinned) !== pinState)
      .map((clip) => clip.id);
    if (idsToChange.length === 0) return;
    const changedIdSet = new Set(idsToChange);

    onClipsRepositioned?.(idsToChange);
    if (!pinState) onClipPropertyRemoved?.('pin', idsToChange);
    setAllClips((previous) => {
      const updated = previous.map((clip) => (
        changedIdSet.has(clip.id) ? { ...clip, is_pinned: pinState } : clip
      ));
      if (pinState) {
        const newlyPinned = updated
          .filter((clip) => changedIdSet.has(clip.id))
          .map((clip, index) => ({ ...clip, pin_order: index }));
        const existingPinned = updated
          .filter((clip) => clip.is_pinned && !changedIdSet.has(clip.id))
          .map((clip) => ({ ...clip, pin_order: (clip.pin_order ?? 0) + newlyPinned.length }));
        return sortClipsForTimeline([
          ...newlyPinned,
          ...existingPinned,
          ...updated.filter((clip) => !clip.is_pinned),
        ]);
      }
      return sortClipsForTimeline(updated);
    });
    setSelectedClip((previous) => previous && changedIdSet.has(previous.id)
      ? { ...previous, is_pinned: pinState, pin_order: pinState ? 0 : previous.pin_order }
      : previous);

    void clipsApi.setPinned(idsToChange, pinState)
      .then(onCollectionChanged)
      .catch((error) => {
        console.error('Failed to set pinned state:', error);
        void fetchClips();
      });
  }, [allClips, fetchClips, onClipPropertyRemoved, onClipsRepositioned, onCollectionChanged, selectedClipIds, setAllClips, setSelectedClip]);

  const setProtected = useCallback((id: number, protectedState: boolean) => {
    const targetIds = selectedClipIds.size > 1 && selectedClipIds.has(id)
      ? Array.from(selectedClipIds)
      : [id];
    const idsToChange = allClips
      .filter((clip) => targetIds.includes(clip.id)
        && !clip.hotkey
        && (protectedState || !clip.protecting_bin_ids?.length)
        && Boolean(clip.is_explicitly_protected ?? clip.is_protected) !== protectedState)
      .map((clip) => clip.id);
    if (idsToChange.length === 0) return;
    const changedIdSet = new Set(idsToChange);
    if (!protectedState) onClipPropertyRemoved?.('protect', idsToChange);

    setAllClips((previous) => previous.map((clip) => (
      changedIdSet.has(clip.id) ? {
        ...clip,
        is_explicitly_protected: protectedState,
        is_protected: protectedState || Boolean(clip.protecting_bin_ids?.length),
      } : clip
    )));
    setSelectedClip((previous) => previous && changedIdSet.has(previous.id)
      ? {
        ...previous,
        is_explicitly_protected: protectedState,
        is_protected: protectedState || Boolean(previous.protecting_bin_ids?.length),
      }
      : previous);

    void clipsApi.setProtected(idsToChange, protectedState)
      .then(onCollectionChanged)
      .catch((error) => {
        console.error('Failed to set protected state:', error);
        void fetchClips();
      });
  }, [allClips, fetchClips, onClipPropertyRemoved, onCollectionChanged, selectedClipIds, setAllClips, setSelectedClip]);

  const setConcealed = useCallback((id: number, concealedState: boolean) => {
    const targetIds = selectedClipIds.size > 1 && selectedClipIds.has(id)
      ? Array.from(selectedClipIds)
      : [id];
    const idsToChange = allClips
      .filter((clip) => targetIds.includes(clip.id)
        && !clip.is_trashed
        && Boolean(clip.is_concealed) !== concealedState)
      .map((clip) => clip.id);
    if (idsToChange.length === 0) return;
    const changedIdSet = new Set(idsToChange);
    if (!concealedState) onClipPropertyRemoved?.('conceal', idsToChange);
    const update = (clip: ClipItem) => changedIdSet.has(clip.id) ? {
      ...clip,
      is_explicitly_concealed: concealedState,
      is_explicitly_revealed: !concealedState,
      is_concealed: concealedState,
    } : clip;
    setAllClips((previous) => previous.map(update));
    setSelectedClip((previous) => previous ? update(previous) : previous);

    void clipsApi.setConcealed(idsToChange, concealedState)
      .then(onCollectionChanged)
      .catch((error) => {
        console.error('Failed to set concealed state:', error);
        void fetchClips();
      });
  }, [allClips, fetchClips, onClipPropertyRemoved, onCollectionChanged, selectedClipIds, setAllClips, setSelectedClip]);

  return {
    togglePin,
    toggleProtected,
    toggleConcealed,
    setPinned,
    setProtected,
    setConcealed,
  };
}
