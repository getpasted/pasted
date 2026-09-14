import { startTransition, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { Bin, ClipItem, SequentialStatus } from '../types';
import { sortClipsChronologically } from '../utils/clipOrder';
import { clipMatchesSearch, parseClipSearch, type ClipSearchFeaturePolicy } from '../utils/clipSearch';
import { getClipCollection } from '../utils/clipCollections';
import type { FeatureId } from '../utils/features';
import { appendUniqueSearchPage, resolveSearchDisplayItems } from '../utils/searchPagination';
import { clipsApi } from '../api/clips';
import { searchHistoryApi } from '../api/searchHistory';
import { usePagedClipCollection } from './usePagedClipCollection';
import { clipListItemsAsClips } from '../utils/clipListItems';

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
  const pagedCollection = usePagedClipCollection({
    currentTab,
    selectedBinId,
    bins,
    features,
    activeClipsRevision: allClips,
    trashClipsRevision: trashedClips,
  });
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
    clipsApi.searchList({ query: normalizedSearchQuery, limit: SEARCH_PAGE_SIZE, offset: 0 }).then((result) => {
      if (active) {
        startTransition(() => {
          setSearchResult({
            query: normalizedSearchQuery,
            items: clipListItemsAsClips(result.items),
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
      const result = await clipsApi.searchList({
        query: normalizedSearchQuery,
        limit: SEARCH_PAGE_SIZE,
        offset: searchResult.items.length,
      });
      setSearchResult((current) => {
        if (current.query !== normalizedSearchQuery) return current;
        return {
          query: current.query,
          items: appendUniqueSearchPage(current.items, clipListItemsAsClips(result.items)),
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

    if (pagedCollection.active) return pagedCollection.items;

    const clips = collection?.membership === 'trash' ? trashedClips : allClips;
    if (collection?.membership === 'trash') return clips;
    return features.pinning ? clips : sortClipsChronologically(clips);
  }, [allClips, trashedClips, normalizedSearchQuery, currentTab, selectedBinId, sequentialStatus, bins, features, searchResult, pagedCollection.active, pagedCollection.items]);

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
    currentPageTotalCount: pagedCollection.active
      ? pagedCollection.totalCount
      : searchResult.query === normalizedSearchQuery
        ? searchResult.totalCount
        : searchResult.loading && searchResult.items.length > 0
          ? searchResult.totalCount
          : displayedClips.length,
    searchDisplayQuery: normalizedSearchQuery ? searchResult.query : '',
    isLoadingCurrentPage: pagedCollection.active
      ? pagedCollection.loading
      : searchResult.loading
        || Boolean(normalizedSearchQuery && searchResult.query !== normalizedSearchQuery),
    searchFailed: searchResult.query === normalizedSearchQuery && searchResult.failed,
    retrySearch: () => setSearchRevision((revision) => revision + 1),
    loadMoreCurrentPage: pagedCollection.active ? pagedCollection.loadMore : loadMoreSearchResults,
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
