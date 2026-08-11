import { useMemo } from 'react';
import type { Bin, ClipItem, SequentialStatus } from '../types';
import { getClipFilePaths, getClipOriginKind } from '../types';
import { sortClipsChronologically } from '../utils/clipOrder';
import { clipMatchesSearch, parseClipSearch } from '../utils/clipSearch';
import { getClipCollection, parseClipFacetRoute } from '../utils/clipCollections';
import type { FeatureId } from '../utils/features';

interface ClipViewsInput {
  allClips: ClipItem[];
  trashedClips: ClipItem[];
  bins: Bin[];
  currentTab: string;
  selectedBinId: number | null;
  searchQuery: string;
  sequentialStatus: SequentialStatus | null;
  features: Record<FeatureId, boolean>;
}

interface SmartCondition {
  type: string;
  operator?: 'is' | 'contains';
  value: string;
}

export function applyClipSearch(
  items: ClipItem[],
  rawQuery: string,
  features?: Pick<Record<FeatureId, boolean>, 'notes' | 'pinning' | 'protection'>,
) {
  const trimmed = rawQuery.trim();
  if (!trimmed) return items;
  const plan = parseClipSearch(trimmed);
  return items.filter((clip) => clipMatchesSearch(features ? {
    ...clip,
    note: features.notes ? clip.note : null,
    is_pinned: features.pinning && clip.is_pinned,
    is_protected: features.protection && clip.is_protected,
  } : clip, plan));
}

function matchesCondition(clip: ClipItem, condition: SmartCondition) {
  const expected = condition.value.toLowerCase().trim();
  if (!expected) return false;
  if (condition.type === 'file_extension') {
    const extension = expected.replace(/^\./, '');
    return Boolean(extension) && getClipFilePaths(clip).some((path) => path.toLowerCase().endsWith(`.${extension}`));
  }
  if (condition.type === 'file_path') {
    return getClipFilePaths(clip).some((path) => path.toLowerCase().includes(expected));
  }
  const actual = condition.type === 'source'
    ? clip.source
    : condition.type === 'content_type'
      ? clip.content_type
      : condition.type === 'origin_kind'
        ? getClipOriginKind(clip)
      : condition.type === 'contains'
        ? clip.text_content
        : null;
  const normalized = actual?.toLowerCase() ?? '';
  const exactMatch = condition.operator === 'is'
    || (condition.operator === undefined && (condition.type === 'content_type' || condition.type === 'origin_kind'));
  return exactMatch ? normalized === expected : normalized.includes(expected);
}

function filterByBin(clips: ClipItem[], bins: Bin[], binId: number) {
  const assigned = (clip: ClipItem) => clip.bin_id === binId || Boolean(clip.bin_ids?.includes(binId));
  const bin = bins.find((item) => item.id === binId);
  let matchingClips: ClipItem[];
  if (!bin?.smart_rule) matchingClips = clips.filter(assigned);
  else {
    try {
      const rule = JSON.parse(bin.smart_rule) as {
        match?: 'all' | 'any';
        conditions?: SmartCondition[];
        type?: string;
        operator?: 'is' | 'contains';
        value?: string;
      };
      const conditions = rule.conditions?.length
        ? rule.conditions
        : rule.type && rule.value !== undefined
          ? [{ type: rule.type, operator: rule.operator, value: rule.value }]
          : [];
      matchingClips = conditions.length === 0
        ? clips.filter(assigned)
        : clips.filter((clip) => assigned(clip) || (rule.match === 'all'
          ? conditions.every((condition) => matchesCondition(clip, condition))
          : conditions.some((condition) => matchesCondition(clip, condition))));
    } catch {
      matchingClips = clips.filter(assigned);
    }
  }

  if (!bin?.clip_order?.length) return matchingClips;
  const positionById = new Map(bin.clip_order.map((clipId, position) => [clipId, position]));
  return matchingClips
    .map((clip, fallbackPosition) => ({ clip, fallbackPosition }))
    .sort((left, right) => {
      const leftPosition = positionById.get(left.clip.id);
      const rightPosition = positionById.get(right.clip.id);
      if (leftPosition !== undefined && rightPosition !== undefined) return leftPosition - rightPosition;
      if (leftPosition !== undefined) return -1;
      if (rightPosition !== undefined) return 1;
      return left.fallbackPosition - right.fallbackPosition;
    })
    .map(({ clip }) => clip);
}

export function useClipViews({
  allClips,
  trashedClips,
  bins,
  currentTab,
  selectedBinId,
  searchQuery,
  sequentialStatus,
  features,
}: ClipViewsInput) {
  const displayedClips = useMemo(() => {
    const selectedBin = selectedBinId === null ? undefined : bins.find((bin) => bin.id === selectedBinId);
    const collection = getClipCollection(currentTab, selectedBin);

    if (collection?.membership === 'queue') {
      return (sequentialStatus?.queue ?? []).map((text, index): ClipItem => ({
        id: -(sequentialStatus?.item_ids[index] ?? index + 1),
        content_type: 'text',
        text_content: text,
        html_content: null,
        image_base64: null,
        content_hash: `queue_${sequentialStatus?.item_ids[index] ?? index}`,
        source: `Queue Position #${index + 1}`,
        bin_id: null,
        is_pinned: false,
        note: null,
        created_at: new Date().toISOString(),
      }));
    }

    if (collection?.membership === 'search') {
      return searchQuery.trim()
        ? applyClipSearch(sortClipsChronologically([...allClips, ...trashedClips]), searchQuery, features)
        : [];
    }

    let clips = collection?.membership === 'trash' ? trashedClips : allClips;
    if (collection?.membership === 'trash') return clips;
    const facet = parseClipFacetRoute(currentTab);
    if (facet?.kind === 'type') clips = clips.filter((clip) => clip.content_type === facet.value);
    if (facet?.kind === 'source') clips = clips.filter((clip) => clip.source === facet.value);
    if (collection?.membership === 'bin' && selectedBinId !== null) clips = filterByBin(clips, bins, selectedBinId);
    if (collection?.membership === 'pinned') clips = clips.filter((clip) => clip.is_pinned);
    if (collection?.membership === 'protected') clips = clips.filter((clip) => clip.is_protected);
    if (collection?.membership === 'noted') clips = clips.filter((clip) => Boolean(clip.note?.trim()));
    if (!features.pinning) clips = sortClipsChronologically(clips);
    return clips;
  }, [allClips, trashedClips, searchQuery, currentTab, selectedBinId, sequentialStatus, bins, features]);

  const counts = useMemo(() => allClips.reduce((result, clip) => ({
    pinnedCount: result.pinnedCount + Number(features.pinning && Boolean(clip.is_pinned)),
    protectedCount: result.protectedCount + Number(features.protection && Boolean(clip.is_protected)),
    notesCount: result.notesCount + Number(features.notes && Boolean(clip.note?.trim())),
  }), { pinnedCount: 0, protectedCount: 0, notesCount: 0 }), [allClips, features.notes, features.pinning, features.protection]);

  const queuedIndexMap = useMemo(() => {
    const indexes = new Map<string, number>();
    (sequentialStatus?.queue ?? []).forEach((text, index) => {
      if (!indexes.has(text)) indexes.set(text, index + 1);
    });
    return indexes;
  }, [sequentialStatus?.queue]);

  return { displayedClips, queuedIndexMap, ...counts };
}
