import { startTransition, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { Bin, ClipItem, SequentialStatus } from '../types';
import { getClipFilePaths, getClipOriginKind } from '../types';
import { sortClipsChronologically } from '../utils/clipOrder';
import { clipMatchesSearch, parseClipSearch, type ClipSearchFeaturePolicy } from '../utils/clipSearch';
import { getClipCollection, parseClipFacetRoute } from '../utils/clipCollections';
import type { FeatureId } from '../utils/features';
import { appendUniqueSearchPage, resolveSearchDisplayItems } from '../utils/searchPagination';
import { clipsApi } from '../api/clips';
import { searchHistoryApi } from '../api/searchHistory';
import {
  CLIP_PROPERTY_ASSOCIATIONS,
  getClipPropertyAssociation,
} from '../utils/clipPropertyAssociations';

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

interface AuthoritativeSearchResult {
  query: string;
  items: ClipItem[];
  totalCount: number;
  loading: boolean;
  failed: boolean;
}

const SEARCH_PAGE_SIZE = 100;

function clipWithFeaturePolicy(
  clip: ClipItem,
  features?: ClipSearchFeaturePolicy,
) {
  return features ? {
    ...clip,
    name: features.naming ? clip.name : null,
    note: features.notes ? clip.note : null,
    is_pinned: features.pinning && clip.is_pinned,
    is_protected: features.protection && clip.is_protected,
  } : clip;
}

export function applyClipSearch(
  items: ClipItem[],
  rawQuery: string,
  features?: ClipSearchFeaturePolicy,
) {
  const trimmed = rawQuery.trim();
  if (!trimmed) return items;
  const plan = parseClipSearch(trimmed);
  return items.filter((clip) => clipMatchesSearch(clipWithFeaturePolicy(clip, features), plan, features));
}

function matchesCondition(clip: ClipItem, condition: SmartCondition, features?: Record<FeatureId, boolean>) {
  const expected = condition.value.toLowerCase().trim();
  if (!expected) return false;
  if (condition.type === 'file_extension') {
    const extension = expected.replace(/^\./, '');
    return Boolean(extension) && getClipFilePaths(clip).some((path) => path.toLowerCase().endsWith(`.${extension}`));
  }
  if (condition.type === 'file_path') {
    return getClipFilePaths(clip).some((path) => path.toLowerCase().includes(expected));
  }
  if (condition.type === 'clip_type') {
    return Boolean(features?.clipTypes) && (condition.operator === 'contains'
      ? clip.content_type.toLowerCase().includes(expected)
      : clip.content_type.toLowerCase() === expected);
  }
  if (condition.type === 'file_format') {
    return Boolean(features?.fileFormats)
      && (clip.file_formats ?? []).some((fileFormat) => condition.operator === 'contains'
        ? fileFormat.toLowerCase().includes(expected)
        : fileFormat.toLowerCase() === expected);
  }
  if (condition.type === 'content_type') {
    return Boolean(features?.types) && (clip.content_types ?? []).some((contentType) => condition.operator === 'contains'
      ? contentType.toLowerCase().includes(expected)
      : contentType.toLowerCase() === expected);
  }
  if (condition.type === 'source' && !features?.sources) return false;
  const actual = condition.type === 'source'
    ? clip.source
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

function filterByBin(clips: ClipItem[], bins: Bin[], binId: number, features: Record<FeatureId, boolean>) {
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
          ? conditions.every((condition) => matchesCondition(clip, condition, features))
          : conditions.some((condition) => matchesCondition(clip, condition, features))));
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
  const normalizedSearchQuery = searchQuery.trim();
  const [searchResult, setSearchResult] = useState<AuthoritativeSearchResult>({
    query: '',
    items: [],
    totalCount: 0,
    loading: false,
    failed: false,
  });
  const [searchRevision, setSearchRevision] = useState(0);
  const searchLoadingRef = useRef(false);
  const recordedSearchRef = useRef<string | null>(null);

  useEffect(() => {
    if (currentTab !== 'search' || !normalizedSearchQuery) {
      searchLoadingRef.current = false;
      recordedSearchRef.current = null;
      setSearchResult((current) => (
        current.query === '' && current.items.length === 0 && current.totalCount === 0
          ? current
          : { query: '', items: [], totalCount: 0, loading: false, failed: false }
      ));
      return;
    }
    let active = true;
    setSearchResult((current) => ({ ...current, loading: true, failed: false }));
    searchLoadingRef.current = true;
    clipsApi.search({ query: normalizedSearchQuery, limit: SEARCH_PAGE_SIZE, offset: 0 }).then((result) => {
      if (active) {
        startTransition(() => {
          setSearchResult({
            query: normalizedSearchQuery,
            items: result.items,
            totalCount: result.totalCount,
            loading: false,
            failed: false,
          });
        });
        if (recordedSearchRef.current !== normalizedSearchQuery) {
          recordedSearchRef.current = normalizedSearchQuery;
          void searchHistoryApi.record({ query: normalizedSearchQuery }, result.totalCount).catch((error) => {
            console.error('Failed to record Search history:', error);
          });
        }
      }
    }).catch((error) => {
      console.error('Failed to search clips:', error);
      if (active) {
        setSearchResult({ query: normalizedSearchQuery, items: [], totalCount: 0, loading: false, failed: true });
      }
    }).finally(() => {
      if (active) searchLoadingRef.current = false;
    });
    return () => {
      active = false;
    };
  // Refresh after clip updates so newly persisted OCR or transcription joins an active search.
  }, [allClips, currentTab, features, normalizedSearchQuery, searchRevision, trashedClips]);

  const loadMoreSearchResults = useCallback(async () => {
    if (currentTab !== 'search'
      || !normalizedSearchQuery
      || searchLoadingRef.current
      || searchResult.query !== normalizedSearchQuery
      || searchResult.items.length >= searchResult.totalCount) return;
    searchLoadingRef.current = true;
    setSearchResult((current) => ({ ...current, loading: true }));
    try {
      const result = await clipsApi.search({
        query: normalizedSearchQuery,
        limit: SEARCH_PAGE_SIZE,
        offset: searchResult.items.length,
      });
      setSearchResult((current) => {
        if (current.query !== normalizedSearchQuery) return current;
        return {
          query: current.query,
          items: appendUniqueSearchPage(current.items, result.items),
          totalCount: result.totalCount,
          loading: false,
          failed: false,
        };
      });
    } catch (error) {
      console.error('Failed to load more Search results:', error);
      setSearchResult((current) => ({ ...current, loading: false, failed: true }));
    } finally {
      searchLoadingRef.current = false;
    }
  }, [currentTab, normalizedSearchQuery, searchResult]);

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
      return resolveSearchDisplayItems(
        normalizedSearchQuery,
        searchResult.query,
        searchResult.items,
      );
    }

    let clips = collection?.membership === 'trash' ? trashedClips : allClips;
    if (collection?.membership === 'trash') return clips;
    const facet = parseClipFacetRoute(currentTab);
    if (facet?.kind === 'clip_type') {
      clips = clips.filter((clip) => clip.content_type === facet.value);
    }
    if (facet?.kind === 'content_type') {
      clips = clips.filter((clip) => (clip.content_types ?? []).includes(facet.value as ClipItem['content_type']));
    }
    if (facet?.kind === 'file_format') {
      clips = clips.filter((clip) => (clip.file_formats ?? []).includes(facet.value));
    }
    if (facet?.kind === 'source') clips = clips.filter((clip) => clip.source === facet.value);
    if (collection?.membership === 'bin' && selectedBinId !== null) clips = filterByBin(clips, bins, selectedBinId, features);
    const propertyAssociation = getClipPropertyAssociation(collection?.association);
    if (propertyAssociation && features[propertyAssociation.feature]) {
      clips = clips.filter(propertyAssociation.isMember);
    }
    if (collection?.membership === 'noted') clips = clips.filter((clip) => Boolean(clip.note?.trim()));
    if (!features.pinning) clips = sortClipsChronologically(clips);
    return clips;
  }, [allClips, trashedClips, normalizedSearchQuery, currentTab, selectedBinId, sequentialStatus, bins, features, searchResult]);

  const counts = useMemo(() => {
    const propertyCounts = new Map(CLIP_PROPERTY_ASSOCIATIONS.map((association) => [
      association.id,
      features[association.feature]
        ? allClips.filter(association.isMember).length
        : 0,
    ]));
    return {
      pinnedCount: propertyCounts.get('pin') ?? 0,
      protectedCount: propertyCounts.get('protect') ?? 0,
      concealedCount: propertyCounts.get('conceal') ?? 0,
      namedCount: propertyCounts.get('name') ?? 0,
      notesCount: features.notes ? allClips.filter((clip) => Boolean(clip.note?.trim())).length : 0,
    };
  }, [allClips, features]);

  const queuedIndexMap = useMemo(() => {
    const indexes = new Map<string, number>();
    (sequentialStatus?.queue ?? []).forEach((text, index) => {
      if (!indexes.has(text)) indexes.set(text, index + 1);
    });
    return indexes;
  }, [sequentialStatus?.queue]);

  return {
    displayedClips,
    queuedIndexMap,
    searchTotalCount: searchResult.query === normalizedSearchQuery
      ? searchResult.totalCount
      : searchResult.loading && searchResult.items.length > 0
        ? searchResult.totalCount
        : displayedClips.length,
    searchDisplayQuery: normalizedSearchQuery ? searchResult.query : '',
    isSearching: searchResult.loading
      || Boolean(normalizedSearchQuery && searchResult.query !== normalizedSearchQuery),
    searchFailed: searchResult.query === normalizedSearchQuery && searchResult.failed,
    retrySearch: () => setSearchRevision((revision) => revision + 1),
    loadMoreSearchResults,
    ...counts,
  };
}

export function useLiveClipSnapshot(
  snapshot: ClipItem | null,
  allClips: ClipItem[],
  trashedClips: ClipItem[],
) {
  return useMemo(() => {
    if (!snapshot) return null;
    return allClips.find(({ id }) => id === snapshot.id)
      ?? trashedClips.find(({ id }) => id === snapshot.id)
      ?? snapshot;
  }, [allClips, snapshot, trashedClips]);
}
