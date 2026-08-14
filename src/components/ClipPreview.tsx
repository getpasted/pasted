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
import { ClipPreviewContent } from './ClipPreviewContent';
import { ClipTransformBar } from './ClipTransformBar';
import { ClipWorkflowMenu } from './ClipWorkflowMenu';
import { MenuSelect } from './MenuSelect';
import { ClipBinPicker } from './ClipBinPicker';
import { ClipNoteViewer } from './ClipNoteViewer';
import { NoteRowItem } from './ClipNoteRow';
import { OverflowText } from './OverflowText';
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
import { contentTypeLabel } from '../utils/contentTypes';

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
  onOpenConnections?: () => void;
  trashEnabled: boolean;
  filePreviewMode: AppSettings['filePreviewMode'];
  filePreviewMaxMb: number;
}

interface StructuralInspection {
  formatVersion: number;
  policy: 'capture' | 'background' | 'interactive' | 'rescan';
  through: 'inspect' | 'extract' | 'classify' | 'enrich';
  result: {
    origin: 'clipboard_content' | 'file_reference' | 'screenshot' | 'command_line';
    byteCount: number;
    text?: { characterCount: number; wordCount: number; lineCount: number };
    image?: { width: number; height: number };
    files?: { itemCount: number; extensions: string[] };
  };
  appliedClipId: number | null;
  liveFileObservations?: {
    availableCount: number;
    fileCount: number;
    directoryCount: number;
    totalSizeBytes: number;
  };
}

interface SmartActionEnrichment {
  formatVersion: number;
  policy: 'interactive';
  through: 'enrich';
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
  through: 'inspect' | 'extract' | 'classify' | 'enrich';
  result: {
    clipKind: string;
    structure?: StructuralInspection['result'];
    detectedType?: string;
    matchedDetectorRef?: string;
    searchableTextAvailable: boolean;
    recommendations?: SmartActionEnrichment['result'];
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
  outcome: 'produced' | 'no_output' | 'failed';
  output: string | null;
  failure: { code: string; message: string } | null;
  appliedClipId: number | null;
  ocrUpdated: boolean;
  classificationUpdated: boolean;
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
  onOpenConnections,
  trashEnabled,
  filePreviewMode,
  filePreviewMaxMb,
}) => {
  const features = useFeatures();
  const relativeTimeNow = useMinuteTick();
  const [copied, setCopied] = useState(false);
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
  const [smartActions, setSmartActions] = useState<SmartActionEnrichment | null>(null);
  const [filePreviews, setFilePreviews] = useState<FileClipPreview[]>([]);
  const [isFilePreviewLoading, setIsFilePreviewLoading] = useState(false);
  const [isHistoryLoading, setIsHistoryLoading] = useState(false);
  const [isLoadingOlderVersions, setIsLoadingOlderVersions] = useState(false);
  const [hasMoreVersions, setHasMoreVersions] = useState(false);

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
    const includeEnricher = features.transformations
      && viewPolicy.canRunPipelines
      && clip.content_type !== 'image'
      && clip.content_type !== 'file';
    setInspection(null);
    setSmartActions(null);
    invoke<AnalyzerPreview>('analyze_content', { ...input, includeEnricher })
      .then((result) => {
        if (cancelled) return;
        setInspection(result.result.structure ? {
          formatVersion: result.formatVersion,
          policy: result.policy,
          through: result.through,
          result: result.result.structure,
          appliedClipId: null,
          liveFileObservations: result.liveFileObservations,
        } : null);
        setSmartActions(result.result.recommendations ? {
          formatVersion: result.formatVersion,
          policy: 'interactive',
          through: 'enrich',
          result: result.result.recommendations,
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
        throw new Error(result.failure?.message ?? 'The Extractor failed.');
      }
      if (result.outcome === 'no_output') {
        throw new Error('No text recognized in image.');
      }
      if (features.revisions) {
        invoke<number>('get_clip_version_count', { clipId: clip.id })
          .then(setRevisionCount)
          .catch((error) => console.error('Failed to refresh clip revision count:', error));
      }
      soundManager.playCopySound();
      onUpdateClip();
    } catch (e) {
      console.error('OCR Extraction Failed:', e);
    } finally {
      setIsOcrLoading(false);
    }
  };

  if (!clip) {
    return (
      <div className="clip-preview-empty flex-1 col-preview h-screen flex flex-col items-center justify-center p-8 select-none">
        <div className="clip-preview-empty-icon theme-surface w-16 h-16 rounded-2xl border flex items-center justify-center mb-4 shadow-xl">
          <FileText className="w-8 h-8" />
        </div>
        <p className="theme-text-main text-sm font-medium">No Clip Selected</p>
        <p className="theme-text-muted text-xs mt-1 max-w-xs text-center">
          Select an item from history or right-click to copy, transform, add notes, or organize.
        </p>
      </div>
    );
  }

  const displayText = previewedVersion?.text_content ?? transformedText ?? clip.text_content ?? '';
  const colorData: ColorFormats | null =
    clip.content_type === 'color' || (displayText && displayText.length < 30)
      ? parseColor(displayText, clip.content_type === 'color')
      : null;
  const canTransformContent = clip.content_type !== 'image' && clip.content_type !== 'file';

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
      setPipelineError(e instanceof Error ? e.message : 'Advanced Transform failed to run.');
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
      setPipelineError(error instanceof Error ? error.message : 'Transform failed to run.');
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
      setPipelineError(`Could not ${destination} the Advanced Transform result.`);
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
          <span className="clip-type-badge theme-badge text-xs font-semibold px-2.5 py-1 rounded-md border capitalize titlebar-drag-handle">
            {clip.content_type === 'file' && getClipFilePaths(clip).length > 1 ? 'Files' : contentTypeLabel(clip.content_type)}
          </span>
          <OverflowText text={clip.source} className="theme-text-main min-w-0 max-w-[200px] truncate text-xs font-medium titlebar-drag-handle" />
          {isTransforming && (
            <LoaderCircle
              className="clip-transform-working h-4 w-4 shrink-0 animate-spin"
              aria-label="Applying Transform"
            />
          )}
          {features.transformations && !isTransforming && provenance && (
            <Workflow
              className="transform-accent pipelines h-4 w-4 shrink-0"
              aria-label={`Transformed with ${provenance.transformName}`}
            />
          )}
          {features.transformations && !isTransforming && provenance?.connectionId && (
            <Sparkles
              className="transform-accent pipelines h-3.5 w-3.5 shrink-0"
              aria-label="Transform used connected intelligence"
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
                title="Workflow"
                aria-label="Open clip workflow"
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
            aria-label={copied ? 'Clip copied' : 'Copy clip'}
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
                className={`clip-preview-action preview-protect-btn theme-focusable transition-colors ${clip.is_protected ? 'is-active' : ''}`}
                title={clip.is_protected ? UI_COPY.unprotect : UI_COPY.protect}
                aria-label={clip.is_protected ? UI_COPY.unprotect : UI_COPY.protect}
                aria-pressed={Boolean(clip.is_protected)}
              >
                {clip.is_protected ? <ShieldOff /> : <Shield />}
              </button>}
            </>
          )}

          {features.notes && viewPolicy.canEditNotes && (
            <button
              type="button"
              onClick={handleToggleAddNote}
              className={`clip-preview-action preview-note-btn theme-focusable transition-colors ${isAddingNote ? 'is-active' : ''}`}
              title={isAddingNote ? 'Cancel Note' : 'Add Note'}
              aria-label={isAddingNote ? 'Cancel Note' : 'Add Note'}
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
              ? 'Clip is Protected. Unprotect first to delete.'
              : clipDeleteLabel({ trashEnabled, permanent: viewPolicy.state === 'trash' })}
            aria-label={viewPolicy.state === 'trash' || !trashEnabled ? 'Delete Clip Permanently' : 'Move Clip to Trash'}
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

      </div>
      ) : (
        <div className="preview-bin-bar px-4 py-2 flex items-center justify-between text-xs border-b" role="note">
          <div className="preview-readonly-notice flex items-center space-x-2">
            <Trash2 className="w-3.5 h-3.5" />
            <span>Restore to organize or edit notes.</span>
          </div>
        </div>
      ))}

      {/* Multi-Note Container (Inline Input Row, Stable Animated Reordering, Non-Selectable) */}
      {features.notes && (notes.length > 0 || isAddingNote) && (
        <div className="px-4 py-2.5 border-b space-y-2 note-container select-none">
          <div className="note-header-text flex items-center space-x-1.5 text-[11px] font-semibold uppercase tracking-wider select-none">
            <StickyNote className="w-3.5 h-3.5" />
            <span>Notes ({notes.length})</span>
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
                <textarea
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
                    Cancel
                  </button>
                  <button
                    type="button"
                    onClick={handleCreateNote}
                    className="note-save-button px-3 py-1 rounded-md text-xs font-semibold shadow cursor-pointer"
                  >
                    Save
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
              <strong className="block">Transform failed.</strong>
              <span>The clip stayed in its Bin and its content was not replaced. </span>
              {transformError === 'Power on a provider and try again.' && onOpenConnections ? (
                <button
                  type="button"
                  className="cursor-pointer font-semibold underline underline-offset-2"
                  onClick={onOpenConnections}
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
              <span>
                {previewedVersion
                  ? 'Previewing revision'
                  : isPipelineRunning
                    ? transformRequestStatus.phase === 'queued'
                      ? `Queued${transformRequestStatus.connectionName ? ` for ${transformRequestStatus.connectionName}` : ''}`
                      : transformRequestStatus.phase === 'starting'
                        ? 'Starting'
                        : `Running${transformRequestStatus.connectionName ? ` with ${transformRequestStatus.connectionName}` : ''}${transformRequestStatus.didFallback ? ' · fallback' : ''}`
                    : 'Previewing'}:
                {' '}<strong>{previewedVersion
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
                Reset
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
          copiedFormat={copiedFormat}
          isOcrLoading={isOcrLoading}
          ocrEnabled={features.ocr}
          readOnly={!viewPolicy.canMutateContent}
          onColorChange={setTransformedText}
          onCopyFormat={(label, value) => void handleCopySpecificFormat(label, value)}
          onRunOCR={() => void handleRunOCR()}
        />

      </div>

      {/* Contextual Transform suggestions live beside the controls they affect. */}
      {features.transformations && viewPolicy.canRunPipelines && canTransformContent && smartActions && smartActions.result.actions.length > 0 && (
          <div className="smart-actions-bar px-4 py-2 flex items-center justify-between text-xs space-x-2 overflow-x-auto">
            <div className="smart-actions-heading flex items-center space-x-1.5 shrink-0 font-semibold text-[11px]">
              <Lightbulb className="w-3.5 h-3.5" />
              <span>Smart Actions ({smartActions.result.signalLabels.join(', ')})</span>
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
                    title={`Preview ${action.transformName}`}
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
              <span className="theme-text-main text-xs font-semibold">Advanced Transform</span>
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
                label="Choose Advanced Transform"
                className={`w-full ${activePipelineRef ? 'preview-filter-select-active' : 'form-field-valid'}`}
                searchable
                searchPlaceholder="Search manual Transforms…"
                options={[
                  { value: '', label: 'Original clip' },
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
                  title="Copy Result"
                >
                  {pipelineAction === 'copied' ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
                  <span>{pipelineAction === 'copied' ? 'Copied' : 'Copy'}</span>
                </button>
                <button
                  type="button"
                  onClick={() => void handlePipelineOutput('paste')}
                  disabled={isPipelineRunning || transformedText === null}
                  className="transform-workspace-action pipelines flex items-center gap-1.5 rounded-lg px-2.5 py-1 text-xs font-bold transition-colors disabled:cursor-not-allowed disabled:opacity-40"
                  title="Paste Result"
                >
                  <ClipboardPaste className="h-3.5 w-3.5" />
                  <span>{pipelineAction === 'pasted' ? 'Pasted' : 'Paste'}</span>
                </button>
                <button
                  onClick={handleResetTransform}
                  className="preview-filter-reset flex items-center space-x-1 px-2.5 py-1 rounded-lg border text-xs font-semibold transition-colors"
                  title="Reset"
                >
                  <span>Reset</span>
                </button>
              </div>
            )}
          </div>
          {pipelineError && (
            <div role="status" className="theme-status-error mt-2 flex items-center gap-2 rounded-lg border px-2.5 py-1.5 text-[11px]">
              <span className="min-w-0 flex-1">{pipelineError}</span>
              <button type="button" onClick={handleRetryTransform} className="playground-run-status-action inline-flex shrink-0 items-center gap-1 rounded-md px-2 py-1 font-semibold">
                <RotateCcw className="h-3 w-3" /> Retry
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
                <span>Items:</span>
                <strong>{inspection?.result.files?.itemCount ?? getClipFilePaths(clip).length}</strong>
              </span>
              <span className="clip-preview-footer-stat" title={inspection?.result.files?.extensions.join(', ') || 'No file extensions'}>
                <span>Types:</span>
                <strong>{inspection?.result.files ? (inspection.result.files.extensions.length > 2 ? `${inspection.result.files.extensions.slice(0, 2).join(', ')} +${inspection.result.files.extensions.length - 2}` : inspection.result.files.extensions.join(', ') || '—') : '…'}</strong>
              </span>
              <span className="clip-preview-footer-stat">
                <span>Size:</span>
                <strong>{inspection?.liveFileObservations ? (inspection.liveFileObservations.fileCount > 0 ? formatFileSize(inspection.liveFileObservations.totalSizeBytes) : '—') : '…'}</strong>
              </span>
              <span className="clip-preview-footer-stat">
                <span>Available:</span>
                <strong>{inspection?.liveFileObservations ? `${inspection.liveFileObservations.availableCount}/${inspection.result.files?.itemCount ?? 0}` : '…'}</strong>
              </span>
            </>
          ) : (
            <>
          <span className="clip-preview-footer-stat">
            <span>Chars:</span>
            <strong>{charCount}</strong>
          </span>
          <span className="clip-preview-footer-stat">
            <span>Words:</span>
            <strong>{wordCount}</strong>
          </span>
          <span className="clip-preview-footer-stat">
            <span>Lines:</span>
            <strong>{lineCount}</strong>
          </span>
          {features.revisions && <span className="clip-preview-footer-stat">
            <span>Revisions:</span>
            <button
              type="button"
              onClick={() => setShowHistory((prev) => !prev)}
              className={`clip-revision-count ${showHistory ? 'is-active' : ''}`}
              title={revisionCount === null ? 'Loading Revisions…' : 'View Revisions'}
              aria-label={revisionCount === null ? 'Loading clip revision count' : `View ${revisionCount} clip revisions`}
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
          <span>Captured:</span>
          <time dateTime={dateTimeAttribute(clip.created_at)} title={formatFullDateTime(clip.created_at)}>
            {formatRelativeTime(clip.created_at, relativeTimeNow)}
          </time>
        </div>
      </div>

      {viewingNote && (
        <ClipNoteViewer
          note={viewingNote}
          source={clip.source}
          onClose={() => setViewingNote(null)}
        />
      )}

    </div>
  );
};
