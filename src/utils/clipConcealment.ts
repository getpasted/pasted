import type { Bin, ClipItem } from '../types';

export interface ConcealableContentType {
  id: string;
  concealClips: boolean;
}

export interface ClipConcealmentPolicy {
  effective: boolean;
  explicit: boolean;
  inheritedFromBin: boolean;
  inheritedFromContentType: boolean;
}

export function clipConcealmentPolicy(
  clip: ClipItem,
  bins: readonly Bin[],
  contentTypes: readonly ConcealableContentType[],
): ClipConcealmentPolicy {
  const assignedBinIds = new Set(clip.bin_ids ?? []);
  if (clip.bin_id !== null) assignedBinIds.add(clip.bin_id);
  const concealingContentTypeIds = new Set(
    contentTypes.filter(({ concealClips }) => concealClips).map(({ id }) => id),
  );
  const inheritedFromBin = Boolean(clip.concealing_bin_ids?.length)
    || bins.some(({ id, conceal_clips }) => Boolean(conceal_clips) && assignedBinIds.has(id));
  const inheritedFromContentType = (clip.content_types ?? [clip.content_type])
    .some((type) => concealingContentTypeIds.has(type))
    || Boolean(clip.concealing_content_types?.length);
  const explicit = Boolean(clip.is_explicitly_concealed ?? clip.is_concealed);
  const explicitlyRevealed = Boolean(clip.is_explicitly_revealed);
  return {
    effective: !explicitlyRevealed
      && (Boolean(clip.is_concealed) || explicit || inheritedFromBin || inheritedFromContentType),
    explicit,
    inheritedFromBin,
    inheritedFromContentType,
  };
}
