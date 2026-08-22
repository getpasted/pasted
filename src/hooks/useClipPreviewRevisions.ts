import { useCallback, useEffect, useState } from 'react';

import type { ClipItem, ClipVersion } from '../types';
import { soundManager } from '../utils/sound';
import { safeInvoke as invoke } from '../utils/tauri';

interface UseClipPreviewRevisionsInput {
  clip: ClipItem | null;
  enabled: boolean;
  canRestore: boolean;
  onBeforeRestore: () => void;
  onUpdateClip: (clip?: ClipItem) => void;
}

export function useClipPreviewRevisions({
  clip,
  enabled,
  canRestore,
  onBeforeRestore,
  onUpdateClip,
}: UseClipPreviewRevisionsInput) {
  const [isOpen, setIsOpen] = useState(false);
  const [versions, setVersions] = useState<ClipVersion[]>([]);
  const [previewedVersion, setPreviewedVersion] = useState<ClipVersion | null>(null);
  const [restoringVersionId, setRestoringVersionId] = useState<number | null>(null);
  const [count, setCount] = useState<number | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  const [hasMore, setHasMore] = useState(false);
  const supported = Boolean(enabled && clip && clip.content_type !== 'file');

  const refreshCount = useCallback(() => {
    if (!supported || !clip) return Promise.resolve();
    return invoke<number>('get_clip_version_count', { clipId: clip.id })
      .then((value) => setCount(Number.isFinite(value) ? value : 0))
      .catch((error) => console.error('Failed to refresh clip revision count:', error));
  }, [clip, supported]);

  useEffect(() => {
    setPreviewedVersion(null);
    setVersions([]);
    setIsOpen(false);
  }, [clip?.id]);

  useEffect(() => {
    let cancelled = false;
    if (!supported || !clip) {
      setCount(null);
      setIsOpen(false);
      return () => { cancelled = true; };
    }
    setCount(null);
    invoke<number>('get_clip_version_count', { clipId: clip.id })
      .then((value) => {
        if (!cancelled) setCount(Number.isFinite(value) ? value : 0);
      })
      .catch((error) => console.error('Failed to load clip revision count:', error));
    return () => { cancelled = true; };
  }, [clip?.id, clip?.is_transformed, clip?.text_content, supported]);

  useEffect(() => {
    let cancelled = false;
    if (!supported || !clip || !isOpen) {
      setIsLoading(false);
      setHasMore(false);
      return () => { cancelled = true; };
    }
    setVersions([]);
    setIsLoading(true);
    Promise.all([
      invoke<ClipVersion[]>('get_clip_versions', { clipId: clip.id, limit: 50, offset: 0 }),
      invoke<number>('get_clip_version_count', { clipId: clip.id }),
    ])
      .then(([result, total]) => {
        if (cancelled) return;
        const items = Array.isArray(result) ? result : [];
        setVersions(items);
        setCount(total);
        setHasMore(items.length < total);
      })
      .catch((error) => console.error('Failed to load clip versions:', error))
      .finally(() => { if (!cancelled) setIsLoading(false); });
    return () => { cancelled = true; };
  }, [clip?.id, clip?.is_transformed, clip?.text_content, isOpen, supported]);

  const loadMore = async () => {
    if (!clip || isLoadingMore || !hasMore) return;
    setIsLoadingMore(true);
    try {
      const result = await invoke<ClipVersion[]>('get_clip_versions', {
        clipId: clip.id,
        limit: 50,
        offset: versions.length,
      });
      const items = Array.isArray(result) ? result : [];
      setVersions((current) => [...current, ...items]);
      setHasMore(versions.length + items.length < (count ?? 0));
    } catch (error) {
      console.error('Failed to load older clip revisions:', error);
    } finally {
      setIsLoadingMore(false);
    }
  };

  const restore = async (version: ClipVersion) => {
    if (!clip || !canRestore || restoringVersionId !== null) return;
    setRestoringVersionId(version.id);
    try {
      const restoredClip = await invoke<ClipItem>('restore_clip_version', {
        clipId: clip.id,
        versionId: version.id,
      });
      void refreshCount();
      onBeforeRestore();
      setIsOpen(false);
      setPreviewedVersion(null);
      soundManager.playCopySound();
      onUpdateClip(restoredClip);
    } catch (error) {
      console.error('Failed to restore clip version:', error);
    } finally {
      setRestoringVersionId(null);
    }
  };

  return {
    supported,
    isOpen,
    setIsOpen,
    toggleOpen: () => setIsOpen((current) => !current),
    versions,
    previewedVersion,
    clearPreview: () => setPreviewedVersion(null),
    togglePreview: (version: ClipVersion) => setPreviewedVersion((current) => (
      current?.id === version.id ? null : version
    )),
    restoringVersionId,
    count,
    isLoading,
    isLoadingMore,
    hasMore,
    loadMore,
    restore,
    refreshCount,
    noteRevisionAdded: () => setCount((current) => (current ?? 0) + 1),
  };
}

export type ClipPreviewRevisionsController = ReturnType<typeof useClipPreviewRevisions>;
