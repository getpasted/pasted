import { useEffect, useRef, useState } from 'react';

import { analysisApi } from '../api/analysis';
import type { ClipItem } from '../types';
import type {
  AnalyzerPreview,
  ClipContentMatch,
  ClipSearchableText,
  ExtractionApplicationResult,
  SmartActionSuggestion,
  StructuralInspection,
} from '../components/clipPreviewModel';
import { useFileClipPreviews } from './useFileClipPreviews';
import { soundManager } from '../utils/sound';
import { safeInvoke as invoke } from '../utils/tauri';
import { useClipVisualLabels } from './useClipVisualLabels';
import { useClipExtractionRecords } from './useClipExtractionRecords';

interface UseClipPreviewAnalysisInput {
  clip: ClipItem | null;
  transformedText: string | null;
  typesEnabled: boolean;
  transformationsEnabled: boolean;
  transcriptionsEnabled: boolean;
  canRunTransforms: boolean;
  canMutateContent: boolean;
  filePreviewMode: 'off' | 'safe' | 'all';
  filePreviewMaxMb: number;
  onRefreshRevisionCount: () => void | Promise<void>;
  onUpdateClip: () => void;
  onError: (message: string) => void;
}

export function useClipPreviewAnalysis({
  clip,
  transformedText,
  typesEnabled,
  transformationsEnabled,
  transcriptionsEnabled,
  canRunTransforms,
  canMutateContent,
  filePreviewMode,
  filePreviewMaxMb,
  onRefreshRevisionCount,
  onUpdateClip,
  onError,
}: UseClipPreviewAnalysisInput) {
  const [contentMatches, setContentMatches] = useState<ClipContentMatch[]>([]);
  const [inspection, setInspection] = useState<StructuralInspection | null>(null);
  const [smartActions, setSmartActions] = useState<SmartActionSuggestion | null>(null);
  const [fileSearchableText, setFileSearchableText] = useState<ClipSearchableText | null>(null);
  const [isFileExtractionLoading, setIsFileExtractionLoading] = useState(false);
  const filePreview = useFileClipPreviews({
    clip,
    mode: filePreviewMode,
    maxSizeMb: filePreviewMaxMb,
  });
  const [isOcrLoading, setIsOcrLoading] = useState(false);
  const [resolvedImage, setResolvedImage] = useState<{ clipId: number; base64: string } | null>(null);
  const fileExtractionRequestIdRef = useRef(0);
  const visualLabelController = useClipVisualLabels({
    clip,
    canMutate: canMutateContent,
    onUpdate: onUpdateClip,
    onError,
  });
  const extractionRecords = useClipExtractionRecords(clip, visualLabelController.refresh);

  useEffect(() => {
    let cancelled = false;
    if (!clip || !typesEnabled) {
      setContentMatches([]);
      return () => { cancelled = true; };
    }
    invoke<ClipContentMatch[]>('get_clip_content_matches', { clipId: clip.id })
      .then((matches) => { if (!cancelled) setContentMatches(Array.isArray(matches) ? matches : []); })
      .catch(() => { if (!cancelled) setContentMatches([]); });
    return () => { cancelled = true; };
  }, [clip?.content_hash, clip?.id, typesEnabled]);

  useEffect(() => {
    let cancelled = false;
    if (!clip) {
      setInspection(null);
      setSmartActions(null);
      return () => { cancelled = true; };
    }
    const text = transformedText ?? clip.text_content ?? '';
    if (transformedText !== null && !text) {
      setInspection(null);
      setSmartActions(null);
      return () => { cancelled = true; };
    }
    const input = transformedText === null
      ? { clipId: clip.id, includeExtractor: false }
      : { text, source: clip.source, includeExtractor: false };
    const includeSuggestions = transformationsEnabled && canRunTransforms
      && clip.content_type !== 'image' && clip.content_type !== 'file';
    setInspection(null);
    setSmartActions(null);
    analysisApi.analyze<AnalyzerPreview>({ ...input, includeClassifiers: includeSuggestions, includeSuggestions })
      .then((result) => {
        if (cancelled) return;
        setInspection(result.result.structure ? {
          formatVersion: result.formatVersion,
          policy: result.policy,
          through: result.through,
          result: result.result.structure,
          mediaMetadata: result.result.mediaMetadata,
          appliedClipId: null,
          liveFileObservations: result.liveFileObservations,
        } : null);
        setSmartActions(result.result.suggestions ? {
          formatVersion: result.formatVersion,
          policy: 'interactive',
          through: 'suggest',
          result: result.result.suggestions,
          appliedClipId: null,
        } : null);
      })
      .catch((error) => {
        if (!cancelled) {
          setInspection(null);
          setSmartActions(null);
          console.error('Failed to analyze clip:', error);
        }
      });
    return () => { cancelled = true; };
  }, [canRunTransforms, clip?.content_hash, clip?.content_type, clip?.id, clip?.source, clip?.text_content, transformationsEnabled, transformedText]);

  useEffect(() => {
    let cancelled = false;
    fileExtractionRequestIdRef.current += 1;
    setIsFileExtractionLoading(false);
    if (!transcriptionsEnabled || !clip || clip.content_type !== 'file') {
      setFileSearchableText(null);
      return () => { cancelled = true; };
    }
    setFileSearchableText(null);
    invoke<ClipSearchableText | null>('get_clip_searchable_text', { clipId: clip.id })
      .then((result) => { if (!cancelled) setFileSearchableText(result); })
      .catch((error) => { if (!cancelled) console.error('Failed to load extracted file text:', error); });
    return () => { cancelled = true; };
  }, [clip?.content_hash, clip?.content_type, clip?.id, transcriptionsEnabled]);

  useEffect(() => {
    let cancelled = false;
    let frame = 0;
    setResolvedImage(null);
    if (clip?.content_type === 'image') {
      const clipId = clip.id;
      const embeddedImage = clip.image_base64;
      frame = requestAnimationFrame(() => {
        if (cancelled) return;
        if (embeddedImage) {
          setResolvedImage({ clipId, base64: embeddedImage });
          return;
        }
        invoke<string | null>('get_clip_image', { id: clipId })
          .then((base64) => { if (!cancelled && base64) setResolvedImage({ clipId, base64 }); })
          .catch(console.error);
      });
    }
    return () => {
      cancelled = true;
      if (frame) cancelAnimationFrame(frame);
    };
  }, [clip?.content_type, clip?.id, clip?.image_base64]);

  useEffect(() => { setIsOcrLoading(false); }, [clip]);

  const runOcr = async () => {
    if (!clip || !canMutateContent) return;
    setIsOcrLoading(true);
    try {
      const result = await invoke<ExtractionApplicationResult>('extract_ocr_from_clip', { clipId: clip.id });
      if (result.outcome === 'failed') {
        await extractionRecords.refresh(clip.id);
        throw new Error(result.failure?.message ?? 'The Extractor failed.');
      }
      if (result.outcome === 'no_output') {
        await extractionRecords.refresh(clip.id);
        return;
      }
      await onRefreshRevisionCount();
      soundManager.playCopySound();
      await extractionRecords.refresh(clip.id);
      onUpdateClip();
    } catch (error) {
      console.error('OCR Extraction Failed:', error);
    } finally {
      setIsOcrLoading(false);
    }
  };

  const runFileExtraction = async () => {
    if (!transcriptionsEnabled || !clip || clip.content_type !== 'file' || !canMutateContent) return;
    const requestedClipId = clip.id;
    const requestId = ++fileExtractionRequestIdRef.current;
    setIsFileExtractionLoading(true);
    try {
      const result = await invoke<ExtractionApplicationResult>('extract_text_from_file_clip', { clipId: requestedClipId });
      if (requestId !== fileExtractionRequestIdRef.current) return;
      if (result.outcome === 'failed') {
        await extractionRecords.refresh(requestedClipId);
        throw new Error(result.failure?.message ?? 'The Extractor failed.');
      }
      if (result.outcome === 'no_output') {
        await extractionRecords.refresh(requestedClipId);
        return;
      }
      const stored = await invoke<ClipSearchableText | null>('get_clip_searchable_text', { clipId: requestedClipId });
      if (requestId !== fileExtractionRequestIdRef.current) return;
      setFileSearchableText(stored);
      await extractionRecords.refresh(requestedClipId);
      soundManager.playCopySound();
      onUpdateClip();
    } catch (error) {
      if (requestId === fileExtractionRequestIdRef.current) onError(String(error));
    } finally {
      if (requestId === fileExtractionRequestIdRef.current) setIsFileExtractionLoading(false);
    }
  };

  return {
    contentMatches,
    inspection,
    smartActions,
    fileSearchableText,
    ...extractionRecords,
    isFileExtractionLoading,
    filePreviews: filePreview.filePreviews,
    isFilePreviewLoading: filePreview.isFilePreviewLoading,
    isOcrLoading,
    resolvedImage,
    runOcr,
    runFileExtraction,
    previewContentAnalysisProps: {
      ...visualLabelController.contentProps,
      onRecheckFileReference: filePreview.recheckFileReference,
    },
  };
}
