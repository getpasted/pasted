import type { ClipItem } from '../types.ts';

export function selectionHasRestrictedClip(
  selectedClipIds: Set<number>,
  displayedClips: ClipItem[],
  isRestricted: (clip: ClipItem) => boolean,
) {
  return Array.from(selectedClipIds).some((id) => {
    const selected = displayedClips.find((clip) => clip.id === id);
    return selected ? isRestricted(selected) : false;
  });
}

export function findDraggedPreviewClip(
  preview: { clipId: number } | null,
  displayedClips: ClipItem[],
  allClips: ClipItem[],
) {
  if (!preview) return undefined;
  return displayedClips.find((clip) => clip.id === preview.clipId)
    ?? allClips.find((clip) => clip.id === preview.clipId);
}
