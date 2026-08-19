import React, { useState, useEffect, useRef } from 'react';
import { dateTimeAttribute, formatFullDateTime, formatRelativeTime } from '../utils/date';
import { useMinuteTick } from '../hooks/useMinuteTick';
import { ClipItem, Bin, Pipeline, ClipNote, parseClipNotes, serializeClipNotes, ClipVersion, getClipFilePaths } from '../types';
import type { AppSettings } from '../types';
import type { ClipTransformationProvenance, TransformationExecutionOutcome, SavedTransform } from '../types';
import { parseColor, ColorFormats } from '../utils/color';
import { soundManager } from '../utils/sound';
import { handleWindowDragDoubleClick, startWindowDrag } from '../utils/windowDrag';
import { ClipRevisionHistory } from './ClipRevisionHistory';
import { ClipPreviewContent, type ExtractionAttempt, type ExtractionResult } from './ClipPreviewContent';
import { ClipTransformBar } from './ClipTransformBar';
import { ClipWorkflowMenu } from './ClipWorkflowMenu';
import { MenuSelect } from './MenuSelect';
import { ClipBinPicker } from './ClipBinPicker';
import { ClipNoteViewer } from './ClipNoteViewer';
import { NoteRowItem } from './ClipNoteRow';
import { OverflowText } from './OverflowText';
import { HotkeyRecorder } from './HotkeyRecorder';
import {
  Copy,
  ClipboardPaste,
  Check,
  Trash2,
  Sliders,
  FileText,
  StickyNote,
  Pin,
  Shield,
  ShieldOff,
  Sparkles,
  LoaderCircle,
  Workflow,
  Lightbulb,
  AlertTriangle,
  RotateCcw,
  X,
} from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { useStableVerticalReorder } from '../hooks/useStableVerticalReorder';
import type { ClipViewPolicy } from '../utils/clipViewPolicy';
import { clipDeleteLabel, UI_COPY } from '../utils/uiCopy';
import { startTransformation, type TransformationExecutionHandle } from '../utils/transformExecution';
import { useIntelligenceRequestStatus } from '../hooks/useIntelligenceRequestStatus';
import { useFeatures } from '../hooks/useFeatures';
import { contentTypeLabel, structuralClipType } from '../utils/contentTypes';
import { useToast } from './ToastProvider';
import { formatTransformRequestPhase, translate } from '../localization/runtime';
import { localizedSourceName } from '../localization/presentation';

interface ClipPreviewProps {
  clip: ClipItem | null;
  viewPolicy: ClipViewPolicy;
  bins: Bin[];
  viewedBinId?: number | null;
  pipelines: Pipeline[];
  onUpdateClip: (updatedClip?: ClipItem) => void;
  onAssignBin: (clipId: number, binId: number | null) => void | Promise<void>;
  onRemoveBin: (clipId: number, binId: number) => void | Promise<void>;
  onTogglePin: (clipId: number) => void;
  onToggleProtected: (clipId: number) => void;
  onDeleteClip: (id: number) => void;
  onUpdateClipNote?: (clipId: number, noteContent: string | null) => void;
  isTransforming?: boolean;
  transformError?: string;
  onOpenTransformations?: () => void;
  onOpenIntelligence?: () => void;
  trashEnabled: boolean;
  filePreviewMode: AppSettings['filePreviewMode'];
  filePreviewMaxMb: number;
}

interface StructuralInspection {
  formatVersion: number;
  policy: 'capture' | 'background' | 'interactive' | 'rescan';
  through: 'inspect' | 'extract' | 'classify' | 'suggest';
  result: {
    origin: 'clipboard_content' | 'file_reference' | 'screenshot' | 'command_line';
    byteCount: number;
    text?: { characterCount: number; wordCount: number; lineCount: number };
    image?: { width: number; height: number };
    files?: { itemCount: number; extensions: string[] };
  };
  appliedClipId: number | null;
  mediaMetadata?: {
    examinedFileCount: number;
    mediaFileCount: number;
    audioStreamCount: number;
    videoStreamCount: number;
    totalDurationMs: number;
    containers: string[];
    codecs: string[];
  };
  fileFormats?: {
    formats: Array<{ format: string; mimeType: string; count: number }>;
    inspectedCount: number;
    unknownCount: number;
    unavailableCount: number;
  };
  liveFileObservations?: {
    availableCount: number;
    fileCount: number;
    directoryCount: number;
    totalSizeBytes: number;
  };
}

interface SmartActionSuggestion {
  formatVersion: number;
  policy: 'interactive';
  through: 'suggest';
  result: {
    signals: Array<'url' | 'json' | 'html' | 'markdown' | 'multi_line' | 'email' | 'phone'>;
    signalLabels: string[];
    actions: Array<{
      transformRef: string;
      transformName: string;
      transformRevision: number;
      reasons: string[];
    }>;
  };
  appliedClipId: null;
}

interface AnalyzerPreview {
  formatVersion: number;
  policy: 'capture' | 'background' | 'interactive' | 'rescan';
  through: 'inspect' | 'extract' | 'classify' | 'suggest';
  result: {
    clipKind: string;
    structure?: StructuralInspection['result'];
    mediaMetadata?: StructuralInspection['mediaMetadata'];
    classificationMatches?: Array<{
      classifierRef: string;
      classifierName: string;
      contentType: string;
      priority: number;
      startOffset: number;
      endOffset: number;
    }>;
    searchableTextAvailable: boolean;
    suggestions?: SmartActionSuggestion['result'];
  };
  appliedClipId: null;
  liveFileObservations?: StructuralInspection['liveFileObservations'];
}

interface FileClipPreview {
  index: number;
  dataUrl: string | null;
  textContent: string | null;
  width: number | null;
  height: number | null;
}

interface ExtractionApplicationResult {
  formatVersion: number;
  policy: 'capture' | 'background' | 'interactive' | 'rescan';
  through: 'inspect' | 'extract' | 'classify' | 'suggest';
  outcome: 'produced' | 'no_output' | 'failed';
  output: string | null;
  classificationMatches: AnalyzerPreview['result']['classificationMatches'];
  failure: { code: string; message: string } | null;
  appliedClipId: number | null;
  ocrUpdated: boolean;
  searchableTextUpdated: boolean;
  classificationUpdated: boolean;
}

interface ClipSearchableText {
  clipId: number;
  extractorRef: string;
  extractorName: string;
  engine: string;
  inputHash: string;
  searchableText: string;
  updatedAt: string;
}

interface ClipContentMatch {
  id: number;
  clipId: number;
  contentType: string;
  classifierRef: string;
  classifierName: string;
  priority: number;
  sourceRepresentation: 'original_text' | 'searchable_text';
  inputHash: string;
  startOffset: number | null;
  endOffset: number | null;
  updatedAt: string;
}

const filePreviewResultCache = new Map<string, FileClipPreview[]>();
const filePreviewRequestCache = new Map<string, Promise<FileClipPreview[]>>();

function loadFilePreviews(
  cacheKey: string,
  request: { clipId: number; mode: AppSettings['filePreviewMode']; maxSizeMb: number },
): Promise<FileClipPreview[]> {
  const cached = filePreviewResultCache.get(cacheKey);
  if (cached) return Promise.resolve(cached);
  const pending = filePreviewRequestCache.get(cacheKey);
  if (pending) return pending;
  const next = invoke<FileClipPreview[]>('get_file_clip_previews', request)
    .then((items) => {
      const previews = Array.isArray(items) ? items : [];
      filePreviewResultCache.set(cacheKey, previews);
      return previews;
    })
    .finally(() => filePreviewRequestCache.delete(cacheKey));
  filePreviewRequestCache.set(cacheKey, next);
  return next;
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = bytes / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && value >= 1024; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value >= 10 ? value.toFixed(0) : value.toFixed(1)} ${unit}`;
}

function formatMediaDuration(milliseconds: number): string {
  const totalSeconds = Math.floor(milliseconds / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
    : `${minutes}:${String(seconds).padStart(2, '0')}`;
}

function contentMatchTitle(contentType: string, matches: ClipContentMatch[]): string | undefined {
  const counts = matches
    .filter((match) => match.contentType === contentType)
    .reduce((result, match) => {
      result.set(match.classifierName, (result.get(match.classifierName) ?? 0) + 1);
      return result;
    }, new Map<string, number>());
  if (counts.size === 0) return undefined;
  return [...counts].map(([name, count]) => count > 1 ? `${name} ×${count}` : name).join(', ');
}

const CLEVER_PLACEHOLDERS = [
  "Add a note before future-you forgets why you copied this...",
  "Jot down your secret brilliance...",
  "What's the tea on this snippet?...",
  "Note to self: Don't lose this thought...",
  "Drop some wisdom, context, or grocery items...",
];

export const ClipPreview: React.FC<ClipPreviewProps> = ({
  clip,
  viewPolicy,
  bins,
  viewedBinId,
  pipelines,
  onUpdateClip,
  onAssignBin,
  onRemoveBin,
  onTogglePin,
  onToggleProtected,
  onDeleteClip,
  onUpdateClipNote,
  isTransforming = false,
  transformError,
  onOpenTransformations,
  onOpenIntelligence,
  trashEnabled,
  filePreviewMode,
  filePreviewMaxMb,
}) => {
  const features = useFeatures();
  const { showToast } = useToast();
  const relativeTimeNow = useMinuteTick();
  const [copied, setCopied] = useState(false);
  const [contentMatches, setContentMatches] = useState<ClipContentMatch[]>([]);
  const [copiedFormat, setCopiedFormat] = useState<string | null>(null);
  const [transformedText, setTransformedText] = useState<string | null>(null);
  const [activePipelineRef, setActivePipelineRef] = useState<string | null>(null);
  const [activePipelineName, setActivePipelineName] = useState<string | null>(null);
  const [transforms, setTransforms] = useState<SavedTransform[]>([]);
  const [activeTransformRef, setActiveTransformRef] = useState<string | null>(null);
  const [activeTransformName, setActiveTransformName] = useState<string | null>(null);
  const [isWorkflowMenuOpen, setIsWorkflowMenuOpen] = useState(false);
  const [transformPreviewOutcome, setTransformPreviewOutcome] = useState<TransformationExecutionOutcome | null>(null);
  const [provenance, setProvenance] = useState<ClipTransformationProvenance | null>(null);
  const [isPipelineRunning, setIsPipelineRunning] = useState(false);
  const [pipelineAction, setPipelineAction] = useState<'copied' | 'pasted' | null>(null);
  const [pipelineError, setPipelineError] = useState<string | null>(null);
  const [notes, setNotes] = useState<ClipNote[]>(() => parseClipNotes(clip?.note));
  const notesRef = useRef(notes);
  const noteBoxRef = useRef<HTMLDivElement>(null);
  const workflowTriggerRef = useRef<HTMLButtonElement>(null);
  const copiedTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const copiedFormatTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pipelineRequestIdRef = useRef(0);
  const fileExtractionRequestIdRef = useRef(0);
  const extractionHistoryRequestIdRef = useRef(0);
  const activeTransformExecutionRef = useRef<TransformationExecutionHandle | null>(null);
  const [transformClientRequestId, setTransformClientRequestId] = useState<string | null>(null);
  const transformRequestStatus = useIntelligenceRequestStatus(transformClientRequestId);

  useEffect(() => {
    notesRef.current = notes;
  }, [notes]);

  const [isAddingNote, setIsAddingNote] = useState<boolean>(false);
  const [newNoteText, setNewNoteText] = useState<string>('');
  const [placeholderText, setPlaceholderText] = useState<string>(CLEVER_PLACEHOLDERS[0]);
  const [editingNoteId, setEditingNoteId] = useState<string | null>(null);
  const [editingNoteText, setEditingNoteText] = useState<string>('');
  const [viewingNote, setViewingNote] = useState<ClipNote | null>(null);
  const [isOcrLoading, setIsOcrLoading] = useState<boolean>(false);
  const [resolvedImage, setResolvedImage] = useState<{ clipId: number; base64: string } | null>(null);
  const [showHistory, setShowHistory] = useState<boolean>(false);
  const [versions, setVersions] = useState<ClipVersion[]>([]);
  const [previewedVersion, setPreviewedVersion] = useState<ClipVersion | null>(null);
  const [restoringVersionId, setRestoringVersionId] = useState<number | null>(null);
  const [revisionCount, setRevisionCount] = useState<number | null>(null);
  const [inspection, setInspection] = useState<StructuralInspection | null>(null);
  const [smartActions, setSmartActions] = useState<SmartActionSuggestion | null>(null);
  const [fileSearchableText, setFileSearchableText] = useState<ClipSearchableText | null>(null);
  const [extractionResults, setExtractionResults] = useState<ExtractionResult[]>([]);
  const [extractionHistory, setExtractionHistory] = useState<ExtractionAttempt[]>([]);
  const [extractionHistoryHasMore, setExtractionHistoryHasMore] = useState(false);
  const [isExtractionHistoryLoading, setIsExtractionHistoryLoading] = useState(false);
  const [isFileExtractionLoading, setIsFileExtractionLoading] = useState(false);
  const [filePreviews, setFilePreviews] = useState<FileClipPreview[]>([]);
  const [isFilePreviewLoading, setIsFilePreviewLoading] = useState(false);
  const [isHistoryLoading, setIsHistoryLoading] = useState(false);
  const [isLoadingOlderVersions, setIsLoadingOlderVersions] = useState(false);
  const [hasMoreVersions, setHasMoreVersions] = useState(false);

  useEffect(() => {
    let cancelled = false;
    if (!clip || !features.types) {
      setContentMatches([]);
      return () => { cancelled = true; };
    }
    invoke<ClipContentMatch[]>('get_clip_content_matches', { clipId: clip.id })
      .then((matches) => {
        if (!cancelled) setContentMatches(Array.isArray(matches) ? matches : []);
      })
      .catch(() => {
        if (!cancelled) setContentMatches([]);
      });
    return () => { cancelled = true; };
  }, [clip?.content_hash, clip?.id, features.types]);

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
    const includeSuggestions = features.transformations
      && viewPolicy.canRunPipelines
      && clip.content_type !== 'image'
      && clip.content_type !== 'file';
    setInspection(null);
    setSmartActions(null);
    invoke<AnalyzerPreview>('analyze_content', {
      request: {
        ...input,
        includeClassifiers: includeSuggestions,
        includeSuggestions,
      },
    })
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
  }, [clip?.content_hash, clip?.content_type, clip?.id, clip?.source, clip?.text_content, features.transformations, transformedText, viewPolicy.canRunPipelines]);

  useEffect(() => {
    let cancelled = false;
    fileExtractionRequestIdRef.current += 1;
    setIsFileExtractionLoading(false);
    if (!features.transcriptions || !clip || clip.content_type !== 'file') {
      setFileSearchableText(null);
      return () => { cancelled = true; };
    }
    setFileSearchableText(null);
    invoke<ClipSearchableText | null>('get_clip_searchable_text', { clipId: clip.id })
      .then((result) => {
        if (!cancelled) setFileSearchableText(result);
      })
      .catch((error) => {
        if (!cancelled) console.error('Failed to load extracted file text:', error);
      });
    return () => { cancelled = true; };
  }, [clip?.content_hash, clip?.content_type, clip?.id, features.transcriptions]);

  const loadExtractionResults = React.useCallback(async (clipId: number) => {
    const results = await invoke<ExtractionResult[]>('get_clip_extraction_results', { clipId });
    setExtractionResults(Array.isArray(results) ? results : []);
    const requestId = ++extractionHistoryRequestIdRef.current;
    setIsExtractionHistoryLoading(true);
    try {
      const attempts = await invoke<ExtractionAttempt[]>('get_clip_extraction_history', {
        clipId,
        limit: 101,
        offset: 0,
      });
      if (requestId !== extractionHistoryRequestIdRef.current) return;
      const page = Array.isArray(attempts) ? attempts : [];
      setExtractionHistory(page.slice(0, 100));
      setExtractionHistoryHasMore(page.length > 100);
    } catch (error) {
      console.error('Failed to refresh Extractor history:', error);
    } finally {
      if (requestId === extractionHistoryRequestIdRef.current) setIsExtractionHistoryLoading(false);
    }
  }, []);

  const loadExtractionHistory = React.useCallback(async (reset: boolean) => {
    if (!clip || (clip.content_type !== 'image' && clip.content_type !== 'file')) return;
    const requestedClipId = clip.id;
    const offset = reset ? 0 : extractionHistory.length;
    const requestId = ++extractionHistoryRequestIdRef.current;
    setIsExtractionHistoryLoading(true);
    try {
      const attempts = await invoke<ExtractionAttempt[]>('get_clip_extraction_history', {
        clipId: requestedClipId,
        limit: 101,
        offset,
      });
      if (requestId !== extractionHistoryRequestIdRef.current) return;
      const page = Array.isArray(attempts) ? attempts : [];
      setExtractionHistory((current) => reset ? page.slice(0, 100) : [...current, ...page.slice(0, 100)]);
      setExtractionHistoryHasMore(page.length > 100);
    } catch (error) {
      console.error('Failed to load Extractor history:', error);
    } finally {
      if (requestId === extractionHistoryRequestIdRef.current) setIsExtractionHistoryLoading(false);
    }
  }, [clip, extractionHistory.length]);

  useEffect(() => {
    let cancelled = false;
    if (!clip || (clip.content_type !== 'image' && clip.content_type !== 'file')) {
      extractionHistoryRequestIdRef.current += 1;
      setExtractionResults([]);
      setExtractionHistory([]);
      setExtractionHistoryHasMore(false);
      setIsExtractionHistoryLoading(false);
      return () => { cancelled = true; };
    }
    setExtractionResults([]);
    extractionHistoryRequestIdRef.current += 1;
    setExtractionHistory([]);
    setExtractionHistoryHasMore(false);
    setIsExtractionHistoryLoading(false);
    invoke<ExtractionResult[]>('get_clip_extraction_results', { clipId: clip.id })
      .then((results) => {
        if (!cancelled) setExtractionResults(Array.isArray(results) ? results : []);
      })
      .catch((error) => {
        if (!cancelled) console.error('Failed to load Extractor results:', error);
      });
    return () => { cancelled = true; };
  }, [clip?.content_hash, clip?.content_type, clip?.id, clip?.ocr_extractor_ref, clip?.text_content]);

  useEffect(() => {
    let cancelled = false;
    if (!clip || clip.content_type !== 'file' || filePreviewMode === 'off') {
      setFilePreviews([]);
      setIsFilePreviewLoading(false);
      return () => { cancelled = true; };
    }
    const cacheKey = `${clip.id}:${clip.content_hash}:${filePreviewMode}:${filePreviewMaxMb}`;
    const cached = filePreviewResultCache.get(cacheKey);
    if (cached) {
      setFilePreviews(cached);
      setIsFilePreviewLoading(false);
      return () => { cancelled = true; };
    }
    setFilePreviews([]);
    setIsFilePreviewLoading(true);
    loadFilePreviews(cacheKey, { clipId: clip.id, mode: filePreviewMode, maxSizeMb: filePreviewMaxMb })
      .then((items) => {
        if (!cancelled) setFilePreviews(items);
      })
      .catch((error) => {
        if (!cancelled) console.error('Failed to load file previews:', error);
      })
      .finally(() => {
        if (!cancelled) setIsFilePreviewLoading(false);
      });
    return () => { cancelled = true; };
  }, [clip?.content_type, clip?.id, filePreviewMode, filePreviewMaxMb]);

  useEffect(() => {
    invoke<SavedTransform[]>('get_intent_transforms')
      .then((items) => setTransforms(Array.isArray(items) ? items : []))
      .catch((error) => console.error('Failed to load Transforms:', error));
  }, []);

  useEffect(() => {
    let cancelled = false;
    if (!clip) {
      setProvenance(null);
      return () => { cancelled = true; };
    }
    invoke<ClipTransformationProvenance | null>('get_clip_transformation_provenance', { clipId: clip.id })
      .then((value) => { if (!cancelled) setProvenance(value); })
      .catch((error) => console.error('Failed to load transformation provenance:', error));
    return () => { cancelled = true; };
  }, [clip?.id, clip?.is_transformed, clip?.text_content]);

  useEffect(() => {
    let cancelled = false;
    if (!features.revisions || !clip || clip.content_type === 'file') {
      setRevisionCount(null);
      setShowHistory(false);
      return () => {
        cancelled = true;
      };
    }

    setRevisionCount(null);
    invoke<number>('get_clip_version_count', { clipId: clip.id })
      .then((count) => {
        if (!cancelled) setRevisionCount(Number.isFinite(count) ? count : 0);
      })
      .catch((error) => console.error('Failed to load clip revision count:', error));

    return () => {
      cancelled = true;
    };
  }, [clip?.id, clip?.is_transformed, clip?.text_content, features.revisions]);

  useEffect(() => {
    let cancelled = false;
    if (features.revisions && clip && clip.content_type !== 'file' && showHistory) {
      setVersions([]);
      setIsHistoryLoading(true);
      Promise.all([
        invoke<ClipVersion[]>('get_clip_versions', { clipId: clip.id, limit: 50, offset: 0 }),
        invoke<number>('get_clip_version_count', { clipId: clip.id }),
      ])
        .then(([res, count]) => {
          if (cancelled) return;
          const items = Array.isArray(res) ? res : [];
          setVersions(items);
          setRevisionCount(count);
          setHasMoreVersions(items.length < count);
        })
        .catch((e) => console.error('Failed to load clip versions:', e))
        .finally(() => {
          if (!cancelled) setIsHistoryLoading(false);
        });
    } else {
      setIsHistoryLoading(false);
      setHasMoreVersions(false);
    }
    return () => {
      cancelled = true;
    };
  }, [clip?.id, clip?.is_transformed, clip?.text_content, features.revisions, showHistory]);

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
          .then((base64) => {
            if (!cancelled && base64) setResolvedImage({ clipId, base64 });
          })
          .catch(console.error);
      });
    }
    return () => {
      cancelled = true;
      if (frame) cancelAnimationFrame(frame);
    };
  }, [clip?.id, clip?.image_base64, clip?.content_type]);

  useEffect(() => {
    void activeTransformExecutionRef.current?.cancel();
    activeTransformExecutionRef.current = null;
    setTransformClientRequestId(null);
    pipelineRequestIdRef.current += 1;
    setTransformedText(null);
    setActivePipelineRef(null);
    setActivePipelineName(null);
    setActiveTransformRef(null);
    setActiveTransformName(null);
    setIsWorkflowMenuOpen(false);
    setTransformPreviewOutcome(null);
    setIsPipelineRunning(false);
    setPipelineAction(null);
    setPipelineError(null);
    setShowHistory(false);
    const parsed = parseClipNotes(clip?.note);
    setNotes(parsed);
    notesRef.current = parsed;
    setIsAddingNote(false);
    setNewNoteText('');
    setEditingNoteId(null);
    setEditingNoteText('');
    setViewingNote(null);
    setIsOcrLoading(false);
    setCopied(false);
    setCopiedFormat(null);
    setVersions([]);
    setPreviewedVersion(null);
    setRestoringVersionId(null);
    setIsHistoryLoading(false);
    setIsLoadingOlderVersions(false);
    setHasMoreVersions(false);
    if (copiedTimerRef.current) clearTimeout(copiedTimerRef.current);
    if (copiedFormatTimerRef.current) clearTimeout(copiedFormatTimerRef.current);
  }, [clip]);

  useEffect(() => {
    if (viewPolicy.canEditNotes) return;
    setIsAddingNote(false);
    setNewNoteText('');
    setEditingNoteId(null);
    setEditingNoteText('');
  }, [viewPolicy.canEditNotes]);

  useEffect(() => {
    if (viewPolicy.canRunPipelines) return;
    setTransformedText(null);
    setActivePipelineRef(null);
    setActivePipelineName(null);
    setPipelineAction(null);
    setPipelineError(null);
  }, [viewPolicy.canRunPipelines]);

  useEffect(() => () => {
    if (copiedTimerRef.current) clearTimeout(copiedTimerRef.current);
    if (copiedFormatTimerRef.current) clearTimeout(copiedFormatTimerRef.current);
  }, []);

  const saveNotes = (updatedNotes: ClipNote[]) => {
    if (!clip || !viewPolicy.canEditNotes) return;
    setNotes(updatedNotes);
    notesRef.current = updatedNotes;
    const serialized = serializeClipNotes(updatedNotes);
    if (onUpdateClipNote) {
      onUpdateClipNote(clip.id, serialized);
    }
    invoke('update_clip_note', {
      clipId: clip.id,
      note: serialized,
    }).catch((e) => console.error('Failed to update clip note:', e));
  };

  const {
    activeId: activeNoteId,
    offsets: noteReorderOffsets,
    isSettling: isNoteReorderSettling,
    startPointerReorder: startNotePointerReorder,
  } = useStableVerticalReorder({
    itemIds: notes.map((note) => note.id),
    containerRef: noteBoxRef,
    disabled: !viewPolicy.canEditNotes || notes.length < 2 || editingNoteId !== null,
    onCommit: (orderedIds) => {
      const byId = new Map(notesRef.current.map((note) => [note.id, note]));
      const reordered = orderedIds
        .map((id) => byId.get(id))
        .filter((note): note is ClipNote => Boolean(note));
      saveNotes(reordered);
    },
  });

  const handleCreateNote = () => {
    if (!viewPolicy.canEditNotes || !newNoteText.trim()) return;
    const newNote: ClipNote = {
      id: `note-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
      text: newNoteText.trim(),
      created_at: new Date().toISOString(),
    };
    const updated = [...notes, newNote];
    setNewNoteText('');
    setIsAddingNote(false);
    saveNotes(updated);
  };

  const handleUpdateNoteItem = (id: string, text: string) => {
    if (!viewPolicy.canEditNotes) return;
    const updated = notes
      .map((n) => (n.id === id ? { ...n, text: text.trim() } : n))
      .filter((n) => n.text.length > 0);
    setEditingNoteId(null);
    saveNotes(updated);
  };

  const handleDeleteNoteItem = (id: string) => {
    if (!viewPolicy.canEditNotes) return;
    const updated = notes.filter((n) => n.id !== id);
    saveNotes(updated);
  };

  const handleRunOCR = async () => {
    if (!clip || !viewPolicy.canMutateContent) return;
    setIsOcrLoading(true);
    try {
      const result = await invoke<ExtractionApplicationResult>('extract_ocr_from_clip', { clipId: clip.id });
      if (result.outcome === 'failed') {
        await loadExtractionResults(clip.id);
        throw new Error(result.failure?.message ?? 'The Extractor failed.');
      }
      if (result.outcome === 'no_output') {
        await loadExtractionResults(clip.id);
        return;
      }
      if (features.revisions) {
        invoke<number>('get_clip_version_count', { clipId: clip.id })
          .then(setRevisionCount)
          .catch((error) => console.error('Failed to refresh clip revision count:', error));
      }
      soundManager.playCopySound();
      await loadExtractionResults(clip.id);
      onUpdateClip();
    } catch (e) {
      console.error('OCR Extraction Failed:', e);
    } finally {
      setIsOcrLoading(false);
    }
  };

  const handleRunFileExtraction = async () => {
    if (!features.transcriptions || !clip || clip.content_type !== 'file' || !viewPolicy.canMutateContent) return;
    const requestedClipId = clip.id;
    const requestId = ++fileExtractionRequestIdRef.current;
    setIsFileExtractionLoading(true);
    try {
      const result = await invoke<ExtractionApplicationResult>('extract_text_from_file_clip', { clipId: requestedClipId });
      if (requestId !== fileExtractionRequestIdRef.current) return;
      if (result.outcome === 'failed') {
        await loadExtractionResults(requestedClipId);
        throw new Error(result.failure?.message ?? 'The Extractor failed.');
      }
      if (result.outcome === 'no_output') {
        await loadExtractionResults(requestedClipId);
        return;
      }
      const stored = await invoke<ClipSearchableText | null>('get_clip_searchable_text', { clipId: requestedClipId });
      if (requestId !== fileExtractionRequestIdRef.current) return;
      setFileSearchableText(stored);
      await loadExtractionResults(requestedClipId);
      soundManager.playCopySound();
      onUpdateClip();
    } catch (error) {
      if (requestId === fileExtractionRequestIdRef.current) {
        showToast({ tone: 'error', message: String(error) });
      }
    } finally {
      if (requestId === fileExtractionRequestIdRef.current) {
        setIsFileExtractionLoading(false);
      }
    }
  };

  if (!clip) {
    return (
      <div className="clip-preview-empty flex-1 col-preview h-screen flex flex-col items-center justify-center p-8 select-none">
        <div className="clip-preview-empty-icon theme-surface w-16 h-16 rounded-2xl border flex items-center justify-center mb-4 shadow-xl">
          <FileText className="w-8 h-8" />
        </div>
        <p className="theme-text-main text-sm font-medium">{translate('component.clipPreview.noClipSelected')}</p>
        <p className="theme-text-muted text-xs mt-1 max-w-xs text-center">
          {translate('component.clipPreview.selectAnItemFromHistoryOrRightClickToCopyTransformAdd')}
        </p>
      </div>
    );
  }

  const displayText = previewedVersion?.text_content ?? transformedText ?? clip.text_content ?? '';
  const detectedContentTypes = clip.content_types ?? [];
  const visibleContentTypes = detectedContentTypes.slice(0, 3);
  const hiddenContentTypes = detectedContentTypes.slice(3);
  const isColorContent = (clip.content_types ?? [clip.content_type]).includes('color');
  const colorData: ColorFormats | null =
    isColorContent || (displayText && displayText.length < 30)
      ? parseColor(displayText, isColorContent)
      : null;
  const canTransformContent = clip.content_type !== 'image' && clip.content_type !== 'file';
  const isExplicitlyProtected = clip.is_explicitly_protected ?? clip.is_protected ?? false;
  const protectionIsInheritedOnly = Boolean(clip.is_protected) && !isExplicitlyProtected;
  const protectionToggleDisabled = Boolean(clip.hotkey) || protectionIsInheritedOnly;

  const handleHotkeyChange = async (hotkey: string | null) => {
    try {
      const updated = await invoke<ClipItem>('update_clip_hotkey', {
        clipId: clip.id,
        hotkey,
      });
      onUpdateClip(updated);
      showToast({
        tone: 'success',
        message: hotkey
          ? translate('component.clipPreview.hotkeyAssignedAndClipProtected')
          : translate('component.clipPreview.hotkeyRemovedProtectionKept'),
      });
    } catch (error) {
      console.error('Failed to update clip hotkey:', error);
      showToast({
        tone: 'error',
        message: translate('component.clipPreview.clipHotkeyCouldNotBeRegistered'),
      });
    }
  };

  const handleCopy = async () => {
    try {
      if (clip.content_type === 'image' || clip.content_type === 'file') {
        await invoke('copy_clip_by_id', { clipId: clip.id });
      } else {
        await invoke('copy_clip_to_system', {
          text: displayText,
          imageBase64: null,
          filePaths: null,
        });
      }
      setCopied(true);
      if (copiedTimerRef.current) clearTimeout(copiedTimerRef.current);
      copiedTimerRef.current = setTimeout(() => setCopied(false), 2000);
    } catch (e) {
      console.error(e);
    }
  };

  const handleCopySpecificFormat = async (label: string, value: string) => {
    try {
      await invoke('copy_clip_to_system', { text: value, imageBase64: null });
      setCopiedFormat(label);
      soundManager.playCopySound();
      if (copiedFormatTimerRef.current) clearTimeout(copiedFormatTimerRef.current);
      copiedFormatTimerRef.current = setTimeout(() => setCopiedFormat(null), 2000);
    } catch (e) {
      console.error(e);
    }
  };

  const handlePreviewPipeline = async (pipeline: Pipeline) => {
    if (!clip.text_content) return;
    setPreviewedVersion(null);
    const requestId = ++pipelineRequestIdRef.current;
    setActivePipelineRef(pipeline.stableRef);
    setActivePipelineName(pipeline.name);
    setActiveTransformRef(null);
    setActiveTransformName(null);
    setTransformPreviewOutcome(null);
    setTransformedText(null);
    setIsPipelineRunning(true);
    setPipelineAction(null);
    setPipelineError(null);
    try {
      const execution = startTransformation(
        clip.text_content,
        { kind: 'pipeline', pipelineRef: pipeline.stableRef },
        { sourceClipId: clip.id },
      );
      activeTransformExecutionRef.current = execution;
      setTransformClientRequestId(execution.clientRequestId);
      const res = await execution.promise;
      if (requestId !== pipelineRequestIdRef.current) return;
      setTransformedText(res.output);
    } catch (e) {
      if (requestId !== pipelineRequestIdRef.current) return;
      console.error(e);
      setPipelineError(e instanceof Error ? e.message : translate('component.clipPreview.advancedTransformFailedToRun'));
    } finally {
      if (requestId === pipelineRequestIdRef.current) {
        activeTransformExecutionRef.current = null;
        setTransformClientRequestId(null);
        setIsPipelineRunning(false);
      }
    }
  };

  const handlePreviewTransform = async (transform: SavedTransform) => {
    if (!clip.text_content) return;
    setPreviewedVersion(null);
    const requestId = ++pipelineRequestIdRef.current;
    setActiveTransformRef(transform.stableRef);
    setActiveTransformName(transform.name);
    setActivePipelineRef(null);
    setActivePipelineName(null);
    setTransformedText(null);
    setTransformPreviewOutcome(null);
    setIsPipelineRunning(true);
    setPipelineAction(null);
    setPipelineError(null);
    setPreviewedVersion(null);
    try {
      const execution = startTransformation(
        clip.text_content,
        { kind: 'transform', transformRef: transform.stableRef },
        { sourceClipId: clip.id },
      );
      activeTransformExecutionRef.current = execution;
      setTransformClientRequestId(execution.clientRequestId);
      const result = await execution.promise;
      if (requestId !== pipelineRequestIdRef.current) return;
      setTransformPreviewOutcome(result);
      setTransformedText(result.output);
    } catch (error) {
      if (requestId !== pipelineRequestIdRef.current) return;
      setPipelineError(error instanceof Error ? error.message : translate('component.clipPreview.transformFailedToRun'));
    } finally {
      if (requestId === pipelineRequestIdRef.current) {
        activeTransformExecutionRef.current = null;
        setTransformClientRequestId(null);
        setIsPipelineRunning(false);
      }
    }
  };

  const handleApplyTransform = async () => {
    if (!activeTransformRef || !transformPreviewOutcome || transformedText === null || !clip.text_content) return;
    setIsPipelineRunning(true);
    setPipelineError(null);
    try {
      const saved = await invoke<ClipTransformationProvenance>('apply_transform_preview_to_clip', {
        clipId: clip.id,
        transformRef: activeTransformRef,
        expectedInput: clip.text_content,
        output: transformedText,
        connectionId: transformPreviewOutcome.connectionId,
        durationMs: transformPreviewOutcome.durationMs,
      });
      setProvenance(saved);
      setRevisionCount((count) => (count ?? 0) + 1);
      soundManager.playCopySound();
      handleResetTransform();
      onUpdateClip();
    } catch (error) {
      setPipelineError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsPipelineRunning(false);
    }
  };

  const handleResetTransform = () => {
    void activeTransformExecutionRef.current?.cancel();
    activeTransformExecutionRef.current = null;
    setTransformClientRequestId(null);
    pipelineRequestIdRef.current += 1;
    setTransformedText(null);
    setActivePipelineRef(null);
    setActivePipelineName(null);
    setActiveTransformRef(null);
    setActiveTransformName(null);
    setTransformPreviewOutcome(null);
    setPipelineAction(null);
    setPipelineError(null);
    setPreviewedVersion(null);
  };

  const handleRetryTransform = () => {
    if (activeTransformRef) {
      const transform = transforms.find((candidate) => candidate.stableRef === activeTransformRef);
      if (transform) void handlePreviewTransform(transform);
      return;
    }
    if (activePipelineRef) {
      const pipeline = pipelines.find((candidate) => candidate.stableRef === activePipelineRef);
      if (pipeline) void handlePreviewPipeline(pipeline);
    }
  };

  const handlePipelineOutput = async (destination: 'copy' | 'paste') => {
    if (transformedText === null) return;
    try {
      if (destination === 'copy') {
        await invoke('copy_clip_to_system', { text: transformedText, imageBase64: null });
        setPipelineAction('copied');
        soundManager.playCopySound();
      } else {
        await invoke('paste_text_to_frontmost', { text: transformedText });
        setPipelineAction('pasted');
        soundManager.playPasteSound();
      }
      setPipelineError(null);
    } catch (error) {
      console.error(`Failed to ${destination} Advanced Transform output:`, error);
      setPipelineError(translate('component.clipPreview.couldNotDestinationTheAdvancedTransformResult', { destination: destination }));
    }
  };

  const handleAssignBin = async (binId: number | null) => {
    if (!viewPolicy.canAssignBins) return;
    try {
      await onAssignBin(clip.id, binId);
    } catch (e) {
      console.error(e);
    }
  };

  const handleRestoreVersion = async (version: ClipVersion) => {
    if (!viewPolicy.canMutateContent || restoringVersionId !== null) return;
    setRestoringVersionId(version.id);
    try {
      const restoredClip = await invoke<ClipItem>('restore_clip_version', {
        clipId: clip.id,
        versionId: version.id,
      });
      invoke<number>('get_clip_version_count', { clipId: clip.id })
        .then(setRevisionCount)
        .catch((error) => console.error('Failed to refresh clip revision count:', error));
      setTransformedText(null);
      setActivePipelineRef(null);
      setActivePipelineName(null);
      setPipelineAction(null);
      setPipelineError(null);
      setShowHistory(false);
      setPreviewedVersion(null);
      soundManager.playCopySound();
      onUpdateClip(restoredClip);
    } catch (error) {
      console.error('Failed to restore clip version:', error);
    } finally {
      setRestoringVersionId(null);
    }
  };

  const handleLoadOlderVersions = async () => {
    if (!clip || isLoadingOlderVersions || !hasMoreVersions) return;
    setIsLoadingOlderVersions(true);
    try {
      const older = await invoke<ClipVersion[]>('get_clip_versions', {
        clipId: clip.id,
        limit: 50,
        offset: versions.length,
      });
      const items = Array.isArray(older) ? older : [];
      setVersions((current) => [...current, ...items]);
      setHasMoreVersions(versions.length + items.length < (revisionCount ?? 0));
    } catch (error) {
      console.error('Failed to load older clip revisions:', error);
    } finally {
      setIsLoadingOlderVersions(false);
    }
  };

  const inspectedText = transformedText === null ? inspection?.result.text : undefined;
  const charCount = inspectedText?.characterCount ?? displayText.length;
  const wordCount = inspectedText?.wordCount ?? (displayText.trim() ? displayText.trim().split(/\s+/).length : 0);
  const lineCount = inspectedText?.lineCount ?? (displayText ? displayText.split('\n').length : 0);

  const handleToggleAddNote = () => {
    if (!isAddingNote) {
      const nextIdx = Math.floor(Math.random() * CLEVER_PLACEHOLDERS.length);
      setPlaceholderText(CLEVER_PLACEHOLDERS[nextIdx]);
    }
    setIsAddingNote((current) => !current);
  };

  return (
    <div className="flex-1 col-preview h-screen flex flex-col overflow-hidden">
      {/* Finder Top Header Bar */}
      <div
        onMouseDown={startWindowDrag}
        onDoubleClick={handleWindowDragDoubleClick}
        className="col-preview-header h-[60px] px-4 flex items-center justify-between cursor-default titlebar-drag-handle shrink-0"
      >
        <div className="flex min-w-0 items-center space-x-3 titlebar-drag-handle">
          {features.clipTypes && <span className="clip-type-badge theme-badge text-xs font-semibold px-2.5 py-1 rounded-md border capitalize titlebar-drag-handle">
            {clip.content_type === 'file' && getClipFilePaths(clip).length > 1
              ? translate('component.clipPreview.files')
              : contentTypeLabel(structuralClipType(clip.content_type))}
          </span>}
          {features.types && visibleContentTypes.map((contentType) => (
            <span
              key={contentType}
              title={contentMatchTitle(contentType, contentMatches)}
              className="clip-type-badge theme-badge text-xs font-semibold px-2.5 py-1 rounded-md border titlebar-drag-handle"
            >
              {contentTypeLabel(contentType)}
            </span>
          ))}
          {features.types && hiddenContentTypes.length > 0 && (
            <span
              title={hiddenContentTypes.map(contentTypeLabel).join(', ')}
              className="clip-type-badge theme-badge text-xs font-semibold px-2.5 py-1 rounded-md border titlebar-drag-handle"
            >
              +{hiddenContentTypes.length}
            </span>
          )}
          {features.sources && <OverflowText text={localizedSourceName(clip.source)} className="theme-text-main min-w-0 max-w-[200px] truncate text-xs font-medium titlebar-drag-handle" />}
          {isTransforming && (
            <LoaderCircle
              className="clip-transform-working h-4 w-4 shrink-0 animate-spin"
              aria-label={translate('component.clipPreview.applyingTransform')}
            />
          )}
          {features.transformations && !isTransforming && provenance && (
            <Workflow
              className="transform-accent pipelines h-4 w-4 shrink-0"
              aria-label={translate('component.clipPreview.transformedWithTransformname', { transformName: provenance.transformName })}
            />
          )}
          {features.transformations && !isTransforming && provenance?.connectionId && (
            <Sparkles
              className="transform-accent pipelines h-3.5 w-3.5 shrink-0"
              aria-label={translate('component.clipPreview.transformUsedConnectedIntelligence')}
            />
          )}
        </div>

        <div className="clip-preview-actions relative flex shrink-0 items-center titlebar-no-drag">
          {features.transformations && viewPolicy.canRunPipelines && canTransformContent && (
            <div className="clip-workflow-shell relative">
              <button
                ref={workflowTriggerRef}
                type="button"
                onClick={() => setIsWorkflowMenuOpen((current) => !current)}
                className={`clip-preview-action clip-workflow-trigger theme-focusable transition-colors ${isWorkflowMenuOpen || activeTransformRef ? 'is-active' : ''}`}
                title={translate('component.clipPreview.workflow')}
                aria-label={translate('component.clipPreview.openClipWorkflow')}
                aria-haspopup="menu"
                aria-expanded={isWorkflowMenuOpen}
              >
                {isPipelineRunning && activeTransformRef
                  ? <LoaderCircle className="h-4 w-4 animate-spin" />
                  : <Workflow className="h-4 w-4" />}
              </button>
              {isWorkflowMenuOpen && (
                <ClipWorkflowMenu
                  transforms={transforms}
                  activeTransformRef={activeTransformRef}
                  isRunning={isPipelineRunning}
                  anchorRef={workflowTriggerRef}
                  onClose={() => setIsWorkflowMenuOpen(false)}
                  onPreview={(transform) => void handlePreviewTransform(transform)}
                  onManageTransforms={() => onOpenTransformations?.()}
                />
              )}
            </div>
          )}
          <button
            type="button"
            onClick={handleCopy}
            className={`clip-preview-action copy-clip-main-btn theme-focusable active:scale-95 transition-[background-color,color,transform] ${copied ? 'is-copied' : ''}`}
            title={copied ? UI_COPY.copied : UI_COPY.copy}
            aria-label={copied ? translate('component.clipPreview.clipCopied') : translate('component.clipPreview.copyClip')}
          >
            {copied ? <Check /> : <Copy />}
          </button>

          {viewPolicy.canOrganize && (features.pinning || features.protection) && (
            <>
              {features.pinning && <button
                type="button"
                onClick={() => onTogglePin(clip.id)}
                className={`clip-preview-action preview-pin-btn theme-focusable transition-colors ${clip.is_pinned ? 'is-active' : ''}`}
                title={clip.is_pinned ? UI_COPY.unpin : UI_COPY.pin}
                aria-label={clip.is_pinned ? UI_COPY.unpin : UI_COPY.pin}
                aria-pressed={Boolean(clip.is_pinned)}
              >
                <Pin className={clip.is_pinned ? 'pin-icon' : ''} />
              </button>}
              {features.protection && <button
                type="button"
                onClick={() => onToggleProtected(clip.id)}
                disabled={protectionToggleDisabled}
                className={`clip-preview-action preview-protect-btn theme-focusable transition-colors ${clip.is_protected ? 'is-active' : ''}`}
                title={clip.hotkey
                  ? translate('component.clipPreview.removeHotkeyBeforeUnprotecting')
                  : protectionIsInheritedOnly
                    ? translate('component.clipPreview.protectedByBin')
                    : clip.is_protected ? UI_COPY.unprotect : UI_COPY.protect}
                aria-label={clip.hotkey
                  ? translate('component.clipPreview.removeHotkeyBeforeUnprotecting')
                  : protectionIsInheritedOnly
                    ? translate('component.clipPreview.protectedByBin')
                    : clip.is_protected ? UI_COPY.unprotect : UI_COPY.protect}
                aria-pressed={Boolean(clip.is_protected)}
              >
                {clip.is_protected && !protectionToggleDisabled ? <ShieldOff /> : <Shield />}
              </button>}
            </>
          )}

          {features.notes && viewPolicy.canEditNotes && (
            <button
              type="button"
              onClick={handleToggleAddNote}
              className={`clip-preview-action preview-note-btn theme-focusable transition-colors ${isAddingNote ? 'is-active' : ''}`}
              title={isAddingNote ? translate('component.clipPreview.cancelNote') : translate('action.addNote')}
              aria-label={isAddingNote ? translate('component.clipPreview.cancelNote') : translate('action.addNote')}
              aria-pressed={isAddingNote}
            >
              <StickyNote />
            </button>
          )}

          <button
            type="button"
            onClick={() => onDeleteClip(clip.id)}
            disabled={Boolean(clip.is_protected) && viewPolicy.state !== 'trash'}
            className={`clip-preview-action preview-delete-btn theme-danger-text theme-focusable active:scale-95 transition-[background-color,color,opacity,transform] ${clip.is_protected && viewPolicy.state !== 'trash' ? 'cursor-not-allowed opacity-45' : ''}`}
            title={clip.is_protected && viewPolicy.state !== 'trash'
              ? translate('component.clipPreview.clipIsProtectedUnprotectFirstToDelete')
              : clipDeleteLabel({ trashEnabled, permanent: viewPolicy.state === 'trash' })}
            aria-label={viewPolicy.state === 'trash' || !trashEnabled ? translate('component.clipPreview.deleteClipPermanently') : translate('component.clipPreview.moveClipToTrash')}
          >
            {viewPolicy.state === 'trash' || !trashEnabled ? <X /> : <Trash2 />}
          </button>
        </div>
      </div>

      {/* Quick Bin Assignment & Note Section */}
      {features.bins && (viewPolicy.canOrganize ? (
      <div className="preview-bin-bar px-4 py-2 flex items-center text-xs border-b">
        <div className="flex min-w-0 items-center">
          <ClipBinPicker
            bins={bins}
            selectedBinIds={clip.bin_ids || []}
            viewedBinId={viewedBinId}
            onClear={() => handleAssignBin(null)}
            onToggle={(binId, selected) => {
              if (selected) void onAssignBin(clip.id, binId);
              else void onRemoveBin(clip.id, binId);
            }}
          />
        </div>

        {features.protection && features.hotkeys && (
          <div className="ms-auto flex items-center ps-3">
            <HotkeyRecorder
              value={clip.hotkey ?? null}
              onChange={(hotkey) => void handleHotkeyChange(hotkey)}
            />
          </div>
        )}

      </div>
      ) : (
        <div className="preview-bin-bar px-4 py-2 flex items-center justify-between text-xs border-b" role="note">
          <div className="preview-readonly-notice flex items-center space-x-2">
            <Trash2 className="w-3.5 h-3.5" />
            <span>{translate('component.clipPreview.restoreToOrganizeOrEditNotes')}</span>
          </div>
        </div>
      ))}

      {features.protection && features.hotkeys && !features.bins && viewPolicy.canOrganize && (
        <div className="preview-bin-bar flex items-center justify-end border-b px-4 py-2 text-xs">
          <HotkeyRecorder
            value={clip.hotkey ?? null}
            onChange={(hotkey) => void handleHotkeyChange(hotkey)}
          />
        </div>
      )}

      {/* Multi-Note Container (Inline Input Row, Stable Animated Reordering, Non-Selectable) */}
      {features.notes && (notes.length > 0 || isAddingNote) && (
        <div className="px-4 py-2.5 border-b space-y-2 note-container select-none">
          <div className="note-header-text flex items-center space-x-1.5 text-[11px] font-semibold uppercase tracking-wider select-none">
            <StickyNote className="w-3.5 h-3.5" />
            <span>{translate('component.clipPreview.noteCount', { count: notes.length })}</span>
          </div>

          <div
            ref={noteBoxRef}
            className={`note-row-stack relative space-y-2 ${isNoteReorderSettling ? 'is-settling-stable-reorder' : ''}`}
          >
                {notes.map((noteItem) => (
                  <NoteRowItem
                    key={noteItem.id}
                    noteItem={noteItem}
                    totalNotes={notes.length}
                    editingNoteId={editingNoteId}
                    editingNoteText={editingNoteText}
                    setEditingNoteId={setEditingNoteId}
                    setEditingNoteText={setEditingNoteText}
                    handleUpdateNoteItem={handleUpdateNoteItem}
                    handleDeleteNoteItem={handleDeleteNoteItem}
                    setViewingNote={setViewingNote}
                    readOnly={!viewPolicy.canEditNotes}
                    isDragging={activeNoteId === noteItem.id}
                    reorderOffsetY={noteReorderOffsets[noteItem.id] ?? 0}
                    onReorderPointerDown={(event) => startNotePointerReorder(noteItem.id, event)}
                  />
                ))}

            {/* Inline New Note Card */}
            {isAddingNote && (
              <div className="note-input-row p-3 rounded-lg border flex flex-col space-y-2 animate-in fade-in duration-100">
                <textarea dir="auto"
                  rows={3}
                  placeholder={placeholderText}
                  value={newNoteText}
                  onChange={(e) => setNewNoteText(e.target.value)}
                  className="w-full bg-transparent border-none outline-none focus:outline-none focus:ring-0 text-xs resize-y min-h-[60px] note-input font-sans leading-relaxed"
                  autoFocus
                  onKeyDown={(e) => {
                    if (e.key === 'Escape') {
                      setIsAddingNote(false);
                      setNewNoteText('');
                    }
                  }}
                />
                <div className="flex items-center justify-end space-x-2 pt-1">
                  <button
                    type="button"
                    onClick={() => {
                      setIsAddingNote(false);
                      setNewNoteText('');
                    }}
                    className="note-cancel-button px-3 py-1 rounded-md text-xs font-medium transition-colors cursor-pointer"
                  >
                    {translate('common.cancel')}
                  </button>
                  <button
                    type="button"
                    onClick={handleCreateNote}
                    className="note-save-button px-3 py-1 rounded-md text-xs font-semibold shadow cursor-pointer"
                  >
                    {translate('common.save')}
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Main Preview Workspace */}
      <div className="clip-preview-workspace overlay-scroll-region flex-1 overflow-y-auto p-4 space-y-4 font-mono text-xs">
        {transformError && !isTransforming && (
          <div className="theme-status-warning flex items-start gap-2 rounded-lg border px-3 py-2" role="status">
            <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
            <div className="min-w-0">
              <strong className="block">{translate('component.clipPreview.transformFailed')}</strong>
              <span>{translate('component.clipPreview.theClipStayedInItsBinAndItsContentWasNotReplaced')} </span>
              {transformError === 'Power on a provider and try again.' && onOpenIntelligence ? (
                <button
                  type="button"
                  className="cursor-pointer font-semibold underline underline-offset-2"
                  onClick={onOpenIntelligence}
                >
                  {transformError}
                </button>
              ) : (
                <span>{transformError}</span>
              )}
            </div>
          </div>
        )}
        {(activePipelineName || previewedVersion) && (
          <div className="active-filter-banner flex items-center justify-between px-3 py-2 border rounded-lg">
            <div className="flex items-center space-x-2">
              <Sliders className="w-4 h-4" />
              <span className="flex items-center gap-1.5">
                <span>
                {previewedVersion
                  ? translate('component.clipPreview.previewingRevision')
                  : isPipelineRunning
                    ? formatTransformRequestPhase(transformRequestStatus)
                    : translate('component.clipPreview.previewing')}
                </span>
                <strong>{previewedVersion
                  ? (
                    <time
                      dateTime={dateTimeAttribute(previewedVersion.created_at)}
                      title={formatFullDateTime(previewedVersion.created_at)}
                    >
                      {formatRelativeTime(previewedVersion.created_at, relativeTimeNow)}
                    </time>
                  )
                  : activePipelineName}</strong>
              </span>
            </div>
            {(activePipelineName || previewedVersion) && (
              <button
                onClick={handleResetTransform}
                className="active-filter-reset text-xs underline"
              >
                {translate('common.reset')}
              </button>
            )}
          </div>
        )}

        <ClipPreviewContent
          clip={clip}
          displayText={displayText}
          colorData={colorData}
          resolvedImageBase64={resolvedImage?.clipId === clip.id ? resolvedImage.base64 : null}
          filePreviews={filePreviews}
          isFilePreviewLoading={isFilePreviewLoading}
          fileSearchableText={fileSearchableText}
          extractionResults={extractionResults}
          extractionHistory={extractionHistory}
          extractionHistoryHasMore={extractionHistoryHasMore}
          isExtractionHistoryLoading={isExtractionHistoryLoading}
          isFileExtractionLoading={isFileExtractionLoading}
          copiedFormat={copiedFormat}
          isOcrLoading={isOcrLoading}
          ocrEnabled={features.ocr}
          transcriptionsEnabled={features.transcriptions}
          readOnly={!viewPolicy.canMutateContent}
          onColorChange={setTransformedText}
          onCopyFormat={(label, value) => void handleCopySpecificFormat(label, value)}
          onRunOCR={() => void handleRunOCR()}
          onRunFileExtraction={() => void handleRunFileExtraction()}
          onLoadExtractionHistory={(reset) => void loadExtractionHistory(reset)}
        />

      </div>

      {/* Contextual Transform suggestions live beside the controls they affect. */}
      {features.transformations && viewPolicy.canRunPipelines && canTransformContent && smartActions && smartActions.result.actions.length > 0 && (
          <div className="smart-actions-bar px-4 py-2 flex items-center justify-between text-xs space-x-2 overflow-x-auto">
            <div className="smart-actions-heading flex items-center space-x-1.5 shrink-0 font-semibold text-[11px]">
              <Lightbulb className="w-3.5 h-3.5" />
              <span>{translate('component.clipPreview.smartActionsSignals', { signals: smartActions.result.signalLabels.join(', ') })}</span>
            </div>
            <div className="flex items-center space-x-1.5 overflow-x-auto scrollbar-none py-0.5">
              {smartActions.result.actions.map((action) => {
                const transform = transforms.find((candidate) => candidate.stableRef === action.transformRef);
                const pipeline = pipelines.find((candidate) => candidate.stableRef === action.transformRef);
                if (!transform && !pipeline) return null;
                return (
                  <button
                    key={action.transformRef}
                    onClick={() => transform ? handlePreviewTransform(transform) : void handlePreviewPipeline(pipeline!)}
                    className="smart-action-button px-2 py-0.5 rounded-md border text-[11px] font-medium flex items-center space-x-1 whitespace-nowrap shadow-sm"
                    title={translate('component.clipPreview.previewTransformname', { transformName: action.transformName })}
                  >
                    <span>{action.transformName}</span>
                  </button>
                );
              })}
            </div>
          </div>
      )}

      {viewPolicy.canRunPipelines && canTransformContent && activeTransformRef && activeTransformName && (
        <ClipTransformBar
          activeTransformName={activeTransformName}
          isRunning={isPipelineRunning}
          hasPreview={transformedText !== null && transformedText !== (clip.text_content || '') && Boolean(transformPreviewOutcome)}
          error={pipelineError}
          onApply={() => void handleApplyTransform()}
          onRetry={handleRetryTransform}
          onReset={handleResetTransform}
          requestStatus={transformClientRequestId ? transformRequestStatus : undefined}
        />
      )}

      {/* Advanced Transform selector */}
      {viewPolicy.canRunPipelines && canTransformContent && pipelines.length > 0 && (
        <div className="preview-filter-bar px-4 py-2.5 border-t select-none">
          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center space-x-2 shrink-0">
              <Sliders className="preview-filter-accent w-4 h-4" />
              <span className="theme-text-main text-xs font-semibold">{translate('component.clipPreview.advancedTransform')}</span>
            </div>

            <div className="max-w-xs flex-1">
              <MenuSelect
                value={activePipelineRef || ''}
                onChange={(selectedRef) => {
                  if (!selectedRef) {
                    handleResetTransform();
                  } else {
                    const found = pipelines.find((pipeline) => pipeline.stableRef === selectedRef);
                    if (found) void handlePreviewPipeline(found);
                  }
                }}
                label={translate('component.clipPreview.chooseAdvancedTransform')}
                className={`w-full ${activePipelineRef ? 'preview-filter-select-active' : 'form-field-valid'}`}
                searchable
                searchPlaceholder={translate('component.clipPreview.searchManualTransforms')}
                options={[
                  { value: '', get label() { return translate('component.clipPreview.originalClip'); } },
                  ...pipelines.map((pipeline) => ({ value: pipeline.stableRef, label: pipeline.name })),
                ]}
              />
            </div>

            {/* Reset Action */}
            {activePipelineRef && (
              <div className="flex items-center gap-1.5 shrink-0">
                <button
                  type="button"
                  onClick={() => void handlePipelineOutput('copy')}
                  disabled={isPipelineRunning || transformedText === null}
                  className="theme-secondary-button flex items-center gap-1.5 rounded-lg border px-2.5 py-1 text-xs font-semibold transition-colors disabled:cursor-not-allowed disabled:opacity-40"
                  title={translate('component.clipPreview.copyResult')}
                >
                  {pipelineAction === 'copied' ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
                  <span>{pipelineAction === 'copied' ? translate('action.copied') : translate('action.copy')}</span>
                </button>
                <button
                  type="button"
                  onClick={() => void handlePipelineOutput('paste')}
                  disabled={isPipelineRunning || transformedText === null}
                  className="transform-workspace-action pipelines flex items-center gap-1.5 rounded-lg px-2.5 py-1 text-xs font-bold transition-colors disabled:cursor-not-allowed disabled:opacity-40"
                  title={translate('component.clipPreview.pasteResult')}
                >
                  <ClipboardPaste className="h-3.5 w-3.5" />
                  <span>{pipelineAction === 'pasted' ? translate('component.clipPreview.pasted') : translate('component.clipPreview.paste')}</span>
                </button>
                <button
                  onClick={handleResetTransform}
                  className="preview-filter-reset flex items-center space-x-1 px-2.5 py-1 rounded-lg border text-xs font-semibold transition-colors"
                  title={translate('common.reset')}
                >
                  <span>{translate('common.reset')}</span>
                </button>
              </div>
            )}
          </div>
          {pipelineError && (
            <div role="status" className="theme-status-error mt-2 flex items-center gap-2 rounded-lg border px-2.5 py-1.5 text-[11px]">
              <span className="min-w-0 flex-1">{pipelineError}</span>
              <button type="button" onClick={handleRetryTransform} className="playground-run-status-action inline-flex shrink-0 items-center gap-1 rounded-md px-2 py-1 font-semibold">
                <RotateCcw className="h-3 w-3" /> {translate('common.retry')}
              </button>
            </div>
          )}
        </div>
      )}

      {features.revisions && showHistory && clip.content_type !== 'file' && (
        <ClipRevisionHistory
          versions={versions}
          isLoading={isHistoryLoading}
          readOnly={!viewPolicy.canMutateContent}
          onClose={() => setShowHistory(false)}
          previewedVersionId={previewedVersion?.id ?? null}
          restoringVersionId={restoringVersionId}
          hasMore={hasMoreVersions}
          isLoadingMore={isLoadingOlderVersions}
          onLoadMore={() => void handleLoadOlderVersions()}
          onPreview={(version) => setPreviewedVersion((current) => (
            current?.id === version.id ? null : version
          ))}
          onRestore={(version) => void handleRestoreVersion(version)}
        />
      )}

      {/* Stats Footer */}
      <div className="clip-preview-footer px-4 py-2.5 border-t flex text-[11px]">
        <div className="clip-preview-footer-stats">
          {clip.content_type === 'file' ? (
            <>
              <span className="clip-preview-footer-stat">
                <span>{translate('component.clipPreview.items')}</span>
                <strong>{inspection?.result.files?.itemCount ?? getClipFilePaths(clip).length}</strong>
              </span>
              <span className="clip-preview-footer-stat" title={inspection?.result.files?.extensions.join(', ') || translate('component.clipPreview.noFileExtensions')}>
                <span>{translate('component.clipPreview.fileExtensions')}</span>
                <strong>{inspection?.result.files ? (inspection.result.files.extensions.length > 2 ? translate('component.clipPreview.valueValue2', { value: inspection.result.files.extensions.slice(0, 2).join(', '), value2: inspection.result.files.extensions.length - 2 }) : inspection.result.files.extensions.join(', ') || '—') : '…'}</strong>
              </span>
              {features.fileFormats && <span className="clip-preview-footer-stat" title={inspection?.fileFormats?.formats.map(({ mimeType }) => mimeType).join(', ')}>
                <span>{translate('component.clipPreview.fileFormats')}</span>
                <strong>{inspection?.fileFormats
                  ? inspection.fileFormats.formats.map(({ format }) => format.toUpperCase()).join(', ') || '—'
                  : '…'}</strong>
              </span>}
              <span className="clip-preview-footer-stat">
                <span>{translate('component.clipPreview.size')}</span>
                <strong>{inspection?.liveFileObservations ? (inspection.liveFileObservations.fileCount > 0 ? formatFileSize(inspection.liveFileObservations.totalSizeBytes) : '—') : '…'}</strong>
              </span>
              <span className="clip-preview-footer-stat">
                <span>{translate('component.clipPreview.available')}</span>
                <strong>{inspection?.liveFileObservations ? `${inspection.liveFileObservations.availableCount}/${inspection.result.files?.itemCount ?? 0}` : '…'}</strong>
              </span>
              {inspection?.mediaMetadata && <>
                <span className="clip-preview-footer-stat" title={inspection.mediaMetadata.containers.join(', ')}>
                  <span>{translate('component.clipPreview.media')}</span>
                  <strong>{inspection.mediaMetadata.mediaFileCount}</strong>
                </span>
                <span className="clip-preview-footer-stat" title={inspection.mediaMetadata.codecs.join(', ')}>
                  <span>{translate('component.clipPreview.codecs')}</span>
                  <strong>{inspection.mediaMetadata.codecs.slice(0, 2).join(', ') || '—'}</strong>
                </span>
                <span className="clip-preview-footer-stat">
                  <span>{translate('component.clipPreview.duration')}</span>
                  <strong>{formatMediaDuration(inspection.mediaMetadata.totalDurationMs)}</strong>
                </span>
              </>}
            </>
          ) : (
            <>
          <span className="clip-preview-footer-stat">
            <span>{translate('component.clipPreview.chars')}</span>
            <strong>{charCount}</strong>
          </span>
          <span className="clip-preview-footer-stat">
            <span>{translate('component.clipPreview.words')}</span>
            <strong>{wordCount}</strong>
          </span>
          <span className="clip-preview-footer-stat">
            <span>{translate('component.clipPreview.lines')}</span>
            <strong>{lineCount}</strong>
          </span>
          {features.revisions && <span className="clip-preview-footer-stat">
            <span>{translate('component.clipPreview.revisions')}</span>
            <button
              type="button"
              onClick={() => setShowHistory((prev) => !prev)}
              className={`clip-revision-count ${showHistory ? 'is-active' : ''}`}
              title={revisionCount === null ? translate('component.clipPreview.loadingRevisions') : translate('component.clipPreview.viewRevisions')}
              aria-label={revisionCount === null ? translate('component.clipPreview.loadingClipRevisionCount') : translate('component.clipPreview.viewCountClipRevisions', { count: revisionCount })}
              aria-expanded={showHistory}
              aria-controls="clip-revision-history-panel"
            >
              {revisionCount ?? '…'}
            </button>
          </span>}
            </>
          )}
        </div>
        <div className="clip-preview-footer-captured">
          <span>{translate('component.clipPreview.captured')}</span>
          <time dateTime={dateTimeAttribute(clip.created_at)} title={formatFullDateTime(clip.created_at)}>
            {formatRelativeTime(clip.created_at, relativeTimeNow)}
          </time>
        </div>
      </div>

      {viewingNote && (
        <ClipNoteViewer
          note={viewingNote}
          source={features.sources ? localizedSourceName(clip.source) : null}
          onClose={() => setViewingNote(null)}
        />
      )}

    </div>
  );
};
