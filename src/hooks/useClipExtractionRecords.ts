import { useCallback, useEffect, useRef, useState } from 'react';

import type { ClipItem } from '../types';
import type { ExtractionAttempt, ExtractionResult } from '../components/clipPreviewModel';
import { safeInvoke as invoke } from '../utils/tauri';

export function useClipExtractionRecords(
  clip: ClipItem | null,
  refreshVisualLabels: (clipId: number) => Promise<void>,
) {
  const [extractionResults, setExtractionResults] = useState<ExtractionResult[]>([]);
  const [extractionHistory, setExtractionHistory] = useState<ExtractionAttempt[]>([]);
  const [extractionHistoryHasMore, setExtractionHistoryHasMore] = useState(false);
  const [isExtractionHistoryLoading, setIsExtractionHistoryLoading] = useState(false);
  const historyRequestIdRef = useRef(0);

  const refresh = useCallback(async (clipId: number) => {
    const [results] = await Promise.all([
      invoke<ExtractionResult[]>('get_clip_extraction_results', { clipId }),
      refreshVisualLabels(clipId),
    ]);
    setExtractionResults(Array.isArray(results) ? results : []);
    const requestId = ++historyRequestIdRef.current;
    setIsExtractionHistoryLoading(true);
    try {
      const attempts = await invoke<ExtractionAttempt[]>('get_clip_extraction_history', { clipId, limit: 101, offset: 0 });
      if (requestId !== historyRequestIdRef.current) return;
      const page = Array.isArray(attempts) ? attempts : [];
      setExtractionHistory(page.slice(0, 100));
      setExtractionHistoryHasMore(page.length > 100);
    } catch (error) {
      console.error('Failed to refresh Extractor history:', error);
    } finally {
      if (requestId === historyRequestIdRef.current) setIsExtractionHistoryLoading(false);
    }
  }, [refreshVisualLabels]);

  const loadExtractionHistory = useCallback(async (reset: boolean) => {
    if (!clip || (clip.content_type !== 'image' && clip.content_type !== 'file')) return;
    const offset = reset ? 0 : extractionHistory.length;
    const requestId = ++historyRequestIdRef.current;
    setIsExtractionHistoryLoading(true);
    try {
      const attempts = await invoke<ExtractionAttempt[]>('get_clip_extraction_history', { clipId: clip.id, limit: 101, offset });
      if (requestId !== historyRequestIdRef.current) return;
      const page = Array.isArray(attempts) ? attempts : [];
      setExtractionHistory((current) => reset ? page.slice(0, 100) : [...current, ...page.slice(0, 100)]);
      setExtractionHistoryHasMore(page.length > 100);
    } catch (error) {
      console.error('Failed to load Extractor history:', error);
    } finally {
      if (requestId === historyRequestIdRef.current) setIsExtractionHistoryLoading(false);
    }
  }, [clip, extractionHistory.length]);

  useEffect(() => {
    let cancelled = false;
    if (!clip || (clip.content_type !== 'image' && clip.content_type !== 'file')) {
      historyRequestIdRef.current += 1;
      setExtractionResults([]);
      setExtractionHistory([]);
      setExtractionHistoryHasMore(false);
      setIsExtractionHistoryLoading(false);
      return () => { cancelled = true; };
    }
    setExtractionResults([]);
    historyRequestIdRef.current += 1;
    setExtractionHistory([]);
    setExtractionHistoryHasMore(false);
    setIsExtractionHistoryLoading(false);
    invoke<ExtractionResult[]>('get_clip_extraction_results', { clipId: clip.id })
      .then((results) => { if (!cancelled) setExtractionResults(Array.isArray(results) ? results : []); })
      .catch((error) => { if (!cancelled) console.error('Failed to load Extractor results:', error); });
    return () => { cancelled = true; };
  }, [clip]);

  return {
    extractionResults,
    extractionHistory,
    extractionHistoryHasMore,
    isExtractionHistoryLoading,
    loadExtractionHistory,
    refresh,
  };
}
