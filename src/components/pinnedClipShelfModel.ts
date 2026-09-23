import type { Bin, ClipItem } from '../types.ts';
import {
  clipConcealmentPolicy,
  type ConcealableContentType,
} from '../utils/clipConcealment.ts';
import { concealedClipMask } from '../utils/concealedClipMask.ts';

function idsChanged(previous?: readonly (number | string)[], next?: readonly (number | string)[]): boolean {
  return (previous ?? []).join('\0') !== (next ?? []).join('\0');
}

export function mergePinnedClipSnapshots(displayed: ClipItem[], stacked: ClipItem[]): ClipItem[] {
  const stackedById = new Map(stacked.map((clip) => [clip.id, clip]));
  const displayedIds = new Set(displayed.map((clip) => clip.id));
  return [
    ...displayed.map((clip) => {
      const latest = stackedById.get(clip.id);
      if (!latest) return clip;
      return idsChanged(clip.content_types, latest.content_types)
        || idsChanged(clip.bin_ids, latest.bin_ids)
        || idsChanged(clip.concealing_bin_ids, latest.concealing_bin_ids)
        || idsChanged(clip.concealing_content_types, latest.concealing_content_types)
        || latest.content_type !== clip.content_type
        || latest.text_content !== clip.text_content
        || latest.source !== clip.source
        || latest.is_concealed !== clip.is_concealed
        || latest.is_explicitly_concealed !== clip.is_explicitly_concealed
        || latest.is_explicitly_revealed !== clip.is_explicitly_revealed
        || latest.file_count !== clip.file_count
        ? latest
        : clip;
    }),
    ...stacked.filter((clip) => !displayedIds.has(clip.id)),
  ];
}

export function pinnedClipIsConcealed(
  clip: ClipItem,
  bins: readonly Bin[],
  contentTypes: readonly ConcealableContentType[],
): boolean {
  return clipConcealmentPolicy(clip, bins, contentTypes).effective;
}

export function pinnedClipSummary(
  clip: ClipItem,
  concealed: boolean,
  visibleSummary: (item: ClipItem) => string,
): string {
  if (concealed) return concealedClipMask(clip);
  return visibleSummary(clip);
}
