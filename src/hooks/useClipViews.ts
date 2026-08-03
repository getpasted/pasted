import { useMemo } from 'react';
import type { Bin, ClipItem, SequentialStatus } from '../types';
import { getClipNoteSummary } from '../types';

interface ClipViewsInput {
  allClips: ClipItem[];
  trashedClips: ClipItem[];
  bins: Bin[];
  currentTab: string;
  selectedBinId: number | null;
  searchQuery: string;
  sequentialStatus: SequentialStatus | null;
}

interface SmartCondition {
  type: string;
  operator?: 'is' | 'contains';
  value: string;
}

export function applyClipSearch(items: ClipItem[], rawQuery: string) {
  const trimmed = rawQuery.trim();
  if (!trimmed) return items;
  const lower = trimmed.toLowerCase();

  if (lower.startsWith('regex:')) {
    const pattern = trimmed.slice(6);
    try {
      const expression = new RegExp(pattern, 'i');
      return items.filter((clip) => [clip.text_content, clip.source_app, clip.note].some((value) => value && expression.test(value)));
    } catch {
      const literal = pattern.toLowerCase();
      return items.filter((clip) => clip.text_content?.toLowerCase().includes(literal));
    }
  }
  if (lower.startsWith('app:')) {
    const value = lower.slice(4).trim();
    return items.filter((clip) => clip.source_app?.toLowerCase().includes(value));
  }
  if (lower.startsWith('type:')) {
    const value = lower.slice(5).trim();
    return items.filter((clip) => clip.content_type?.toLowerCase().includes(value));
  }
  if (lower === 'has:note') return items.filter((clip) => Boolean(clip.note?.trim()));
  if (lower === 'is:pinned') return items.filter((clip) => clip.is_pinned);
  if (lower === 'is:protected') return items.filter((clip) => clip.is_protected);

  return items.filter((clip) =>
    clip.text_content?.toLowerCase().includes(lower)
    || clip.source_app?.toLowerCase().includes(lower)
    || getClipNoteSummary(clip.note).toLowerCase().includes(lower)
    || clip.content_type?.toLowerCase().includes(lower));
}

function matchesCondition(clip: ClipItem, condition: SmartCondition) {
  const expected = condition.value.toLowerCase().trim();
  if (!expected) return false;
  const actual = condition.type === 'source_app'
    ? clip.source_app
    : condition.type === 'content_type'
      ? clip.content_type
      : condition.type === 'contains'
        ? clip.text_content
        : null;
  const normalized = actual?.toLowerCase() ?? '';
  const exactMatch = condition.operator === 'is'
    || (condition.operator === undefined && condition.type === 'content_type');
  return exactMatch ? normalized === expected : normalized.includes(expected);
}

function filterByBin(clips: ClipItem[], bins: Bin[], binId: number) {
  const assigned = (clip: ClipItem) => clip.bin_id === binId || Boolean(clip.bin_ids?.includes(binId));
  const bin = bins.find((item) => item.id === binId);
  if (!bin?.smart_rule) return clips.filter(assigned);

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
    if (conditions.length === 0) return clips.filter(assigned);
    return clips.filter((clip) => assigned(clip) || (rule.match === 'all'
      ? conditions.every((condition) => matchesCondition(clip, condition))
      : conditions.some((condition) => matchesCondition(clip, condition))));
  } catch {
    return clips.filter(assigned);
  }
}

export function useClipViews({
  allClips,
  trashedClips,
  bins,
  currentTab,
  selectedBinId,
  searchQuery,
  sequentialStatus,
}: ClipViewsInput) {
  const displayedClips = useMemo(() => {
    if (currentTab === 'sequential') {
      return (sequentialStatus?.queue ?? []).map((text, index): ClipItem => ({
        id: 999000 + index,
        content_type: 'text',
        text_content: text,
        html_content: null,
        image_base64: null,
        content_hash: `queue_${index}`,
        source_app: `Queue Position #${index + 1}`,
        bin_id: null,
        is_pinned: false,
        note: null,
        created_at: new Date().toISOString(),
      }));
    }

    const hasSearch = searchQuery.trim().length > 0;
    const searchPool = currentTab === 'trash'
      ? trashedClips
      : hasSearch
        ? [...allClips, ...trashedClips]
        : allClips;
    let clips = applyClipSearch(searchPool, searchQuery);
    if (currentTab === 'trash') return clips;
    if (currentTab === 'bin' && selectedBinId !== null) clips = filterByBin(clips, bins, selectedBinId);
    if (currentTab === 'pinned') clips = clips.filter((clip) => clip.is_pinned);
    if (currentTab === 'protected') clips = clips.filter((clip) => clip.is_protected);
    if (currentTab === 'notes') clips = clips.filter((clip) => Boolean(clip.note?.trim()));
    return clips;
  }, [allClips, trashedClips, searchQuery, currentTab, selectedBinId, sequentialStatus, bins]);

  const counts = useMemo(() => allClips.reduce((result, clip) => ({
    pinnedCount: result.pinnedCount + Number(Boolean(clip.is_pinned)),
    protectedCount: result.protectedCount + Number(Boolean(clip.is_protected)),
    notesCount: result.notesCount + Number(Boolean(clip.note?.trim())),
  }), { pinnedCount: 0, protectedCount: 0, notesCount: 0 }), [allClips]);

  const queuedIndexMap = useMemo(() => {
    const indexes = new Map<string, number>();
    (sequentialStatus?.queue ?? []).forEach((text, index) => {
      if (!indexes.has(text)) indexes.set(text, index + 1);
    });
    return indexes;
  }, [sequentialStatus?.queue]);

  return { displayedClips, queuedIndexMap, ...counts };
}
