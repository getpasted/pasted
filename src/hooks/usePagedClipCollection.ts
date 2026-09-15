import { startTransition, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { Bin, ClipItem } from '../types';
import type { ClipCollectionPageRequest } from '../api/clipListTypes';
import { clipsApi } from '../api/clips';
import { getClipCollection, parseClipFacetRoute } from '../utils/clipCollections';
import type { FeatureId } from '../utils/features';
import { clipListItemsAsClips } from '../utils/clipListItems';
import {
  cacheRecentClipCollection,
  clearRecentClipCollections,
  getRecentClipCollection,
  peekRecentClipCollection,
} from '../utils/recentClipCollectionCache';

const COLLECTION_PAGE_SIZE = 100;

function requestForCollection(
  currentTab: string,
  selectedBinId: number | null,
  bins: Bin[],
  features: Record<FeatureId, boolean>,
): ClipCollectionPageRequest | null {
  const selectedBin = selectedBinId === null
    ? undefined
    : bins.find((bin) => bin.id === selectedBinId);
  const collection = getClipCollection(currentTab, selectedBin);
  if (collection?.membership === 'bin' && selectedBinId !== null) {
    return { collection: 'bin', binId: selectedBinId };
  }
  if (collection?.membership === 'pinned') return { collection: 'pinned' };
  if (collection?.membership === 'protected') return { collection: 'protected' };
  if (collection?.membership === 'concealed') return { collection: 'concealed' };
  if (collection?.membership === 'named') return { collection: 'named' };
  if (collection?.membership === 'noted') return { collection: 'noted' };
  const facet = parseClipFacetRoute(currentTab);
  if (!facet) return null;
  const feature = {
    clip_type: 'clipTypes',
    content_type: 'types',
    file_format: 'fileFormats',
    source: 'sources',
  } as const;
  if (!features[feature[facet.kind]]) return null;
  const collectionKind = {
    clip_type: 'clipType',
    content_type: 'contentType',
    file_format: 'fileFormat',
    source: 'source',
  } as const;
  return { collection: collectionKind[facet.kind], value: facet.value };
}

export function usePagedClipCollection({
  currentTab,
  selectedBinId,
  bins,
  features,
  activeClipsRevision,
  trashClipsRevision,
}: {
  currentTab: string;
  selectedBinId: number | null;
  bins: Bin[];
  features: Record<FeatureId, boolean>;
  activeClipsRevision: ClipItem[];
  trashClipsRevision: ClipItem[];
}) {
  const request = useMemo(
    () => requestForCollection(currentTab, selectedBinId, bins, features),
    [bins, currentTab, features, selectedBinId],
  );
  const requestKey = request ? JSON.stringify(request) : '';
  const [state, setState] = useState({
    requestKey: '',
    items: [] as ClipItem[],
    totalCount: 0,
    loading: false,
    failed: false,
  });
  const loadingRef = useRef(false);
  const revisionsRef = useRef({ activeClipsRevision, trashClipsRevision });
  if (revisionsRef.current.activeClipsRevision !== activeClipsRevision
    || revisionsRef.current.trashClipsRevision !== trashClipsRevision) {
    clearRecentClipCollections();
    revisionsRef.current = { activeClipsRevision, trashClipsRevision };
  }

  useEffect(() => {
    if (!request) {
      loadingRef.current = false;
      setState((current) => current.requestKey ? {
        requestKey: '', items: [], totalCount: 0, loading: false, failed: false,
      } : current);
      return;
    }
    let active = true;
    const cached = getRecentClipCollection(requestKey);
    loadingRef.current = true;
    setState(cached
      ? { requestKey, ...cached, loading: false, failed: false }
      : { requestKey, items: [], totalCount: 0, loading: true, failed: false });
    void clipsApi.collectionPage({ ...request, limit: COLLECTION_PAGE_SIZE, offset: 0 })
      .then((page) => {
        if (!active) return;
        const collection = {
          items: clipListItemsAsClips(page.items),
          totalCount: page.totalCount,
        };
        cacheRecentClipCollection(requestKey, collection);
        startTransition(() => setState({
          requestKey,
          ...collection,
          loading: false,
          failed: false,
        }));
      })
      .catch((error) => {
        console.error('Failed to load clip collection:', error);
        if (active) setState({ requestKey, items: [], totalCount: 0, loading: false, failed: true });
      })
      .finally(() => {
        if (active) loadingRef.current = false;
      });
    return () => {
      active = false;
    };
  }, [activeClipsRevision, requestKey, trashClipsRevision]);

  const loadMore = useCallback(async () => {
    if (!request || loadingRef.current || state.requestKey !== requestKey
      || state.items.length >= state.totalCount) return;
    loadingRef.current = true;
    setState((current) => ({ ...current, loading: true }));
    try {
      const page = await clipsApi.collectionPage({
        ...request,
        limit: COLLECTION_PAGE_SIZE,
        offset: state.items.length,
      });
      setState((current) => {
        if (current.requestKey !== requestKey) return current;
        const known = new Set(current.items.map((clip) => clip.id));
        return {
          ...current,
          items: [...current.items, ...clipListItemsAsClips(page.items).filter((clip) => !known.has(clip.id))],
          totalCount: page.totalCount,
          loading: false,
          failed: false,
        };
      });
    } catch (error) {
      console.error('Failed to load more clips in collection:', error);
      setState((current) => ({ ...current, loading: false, failed: true }));
    } finally {
      loadingRef.current = false;
    }
  }, [request, requestKey, state]);

  const cached = state.requestKey === requestKey ? undefined : peekRecentClipCollection(requestKey);
  return {
    active: Boolean(request),
    items: state.requestKey === requestKey ? state.items : cached?.items ?? [],
    totalCount: state.requestKey === requestKey ? state.totalCount : cached?.totalCount ?? 0,
    loading: state.requestKey === requestKey ? state.loading : Boolean(request && !cached),
    failed: state.requestKey === requestKey && state.failed,
    loadMore,
  };
}
