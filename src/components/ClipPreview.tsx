import React, { useState, useEffect, useRef } from 'react';
import { dateTimeAttribute, formatFullDateTime, formatRelativeTime } from '../utils/date';
import { useMinuteTick } from '../hooks/useMinuteTick';
import { ClipItem, Bin, ManualTransform, getClipFilePaths } from '../types';
import type { AppSettings } from '../types';
import type { ClipTransformationProvenance, TransformationExecutionOutcome, SavedTransform } from '../types';
import { parseColor, ColorFormats } from '../utils/color';
import { soundManager } from '../utils/sound';
import { handleWindowDragDoubleClick, startWindowDrag } from '../utils/windowDrag';
import { ClipRevisionHistory } from './ClipRevisionHistory';
import { ClipPreviewFooter } from './ClipPreviewFooter';
import { ClipPreviewContent } from './ClipPreviewContent';
import { ClipTransformBar } from './ClipTransformBar';
import { ClipWorkflowMenu } from './ClipWorkflowMenu';
import { MenuSelect } from './MenuSelect';
import { ClipBinPicker } from './ClipBinPicker';
import { ClipNoteViewer } from './ClipNoteViewer';
import { ClipPreviewNotesPanel } from './ClipPreviewNotesPanel';
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
  Eye,
  EyeOff,
  Sparkles,
  LoaderCircle,
  Workflow,
  Lightbulb,
  AlertTriangle,
  RotateCcw,
  X,
  FilePenLine,
} from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { useClipPreviewNotes } from '../hooks/useClipPreviewNotes';
import { useClipPreviewRevisions } from '../hooks/useClipPreviewRevisions';
import { useClipPreviewAnalysis } from '../hooks/useClipPreviewAnalysis';
import type { ClipViewPolicy } from '../utils/clipViewPolicy';
import { clipDeleteLabel, UI_COPY } from '../utils/uiCopy';
import { startTransformation, type TransformationExecutionHandle } from '../utils/transformExecution';
import { useIntelligenceRequestStatus } from '../hooks/useIntelligenceRequestStatus';
import { useFeatures } from '../hooks/useFeatures';
import { contentTypeLabel, structuralClipType } from '../utils/contentTypes';
import { useToast } from './ToastProvider';
import { formatTransformRequestPhase, translate } from '../localization/runtime';
import { localizedSourceName } from '../localization/presentation';
import { useContentTypes } from './ContentTypeProvider';
import { clipConcealmentPolicy } from '../utils/clipConcealment';
import { contentMatchTitle } from './clipPreviewModel';

interface ClipPreviewProps {
  clip: ClipItem | null;
  viewPolicy: ClipViewPolicy;
  bins: Bin[];
  viewedBinId?: number | null;
  manualTransforms: ManualTransform[];
  onUpdateClip: (updatedClip?: ClipItem) => void;
  onAssignBin: (clipId: number, binId: number | null) => void | Promise<void>;
  onRemoveBin: (clipId: number, binId: number) => void | Promise<void>;
  onTogglePin: (clipId: number) => void;
  onToggleProtected: (clipId: number) => void;
  onToggleConcealed: (clipId: number) => void;
  onName: (clip: ClipItem) => void;
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

export const ClipPreview: React.FC<ClipPreviewProps> = ({
  clip,
  viewPolicy,
  bins,
  viewedBinId,
  manualTransforms,
  onUpdateClip,
  onAssignBin,
  onRemoveBin,
  onTogglePin,
  onToggleProtected,
  onToggleConcealed,
  onName,
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
  const { definitions: contentTypes } = useContentTypes();
  const { showToast } = useToast();
  const relativeTimeNow = useMinuteTick();
  const [copied, setCopied] = useState(false);
  const [copiedFormat, setCopiedFormat] = useState<string | null>(null);
  const [transformedText, setTransformedText] = useState<string | null>(null);
  const [activeManualTransformRef, setActiveManualTransformRef] = useState<string | null>(null);
  const [activeManualTransformName, setActiveManualTransformName] = useState<string | null>(null);
  const [transforms, setTransforms] = useState<SavedTransform[]>([]);
  const [activeTransformRef, setActiveTransformRef] = useState<string | null>(null);
  const [activeTransformName, setActiveTransformName] = useState<string | null>(null);
  const [isWorkflowMenuOpen, setIsWorkflowMenuOpen] = useState(false);
  const [transformPreviewOutcome, setTransformPreviewOutcome] = useState<TransformationExecutionOutcome | null>(null);
  const [provenance, setProvenance] = useState<ClipTransformationProvenance | null>(null);
  const [isManualTransformRunning, setIsManualTransformRunning] = useState(false);
  const [pipelineAction, setManualTransformAction] = useState<'copied' | 'pasted' | null>(null);
  const [pipelineError, setManualTransformError] = useState<string | null>(null);
  const notesController = useClipPreviewNotes({
    clip,
    canEdit: viewPolicy.canEditNotes,
    onUpdateClipNote,
  });
  const {
    isAdding: isAddingNote,
    viewingNote,
    setViewingNote,
    toggleAdding: handleToggleAddNote,
  } = notesController;
  const workflowTriggerRef = useRef<HTMLButtonElement>(null);
  const copiedTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const copiedFormatTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pipelineRequestIdRef = useRef(0);
  const activeTransformExecutionRef = useRef<TransformationExecutionHandle | null>(null);
  const [transformClientRequestId, setTransformClientRequestId] = useState<string | null>(null);
  const transformRequestStatus = useIntelligenceRequestStatus(transformClientRequestId);

  const revisions = useClipPreviewRevisions({
    clip,
    enabled: features.revisions,
    canRestore: viewPolicy.canMutateContent,
    onBeforeRestore: () => {
      setTransformedText(null);
      setActiveManualTransformRef(null);
      setActiveManualTransformName(null);
      setManualTransformAction(null);
      setManualTransformError(null);
    },
    onUpdateClip,
  });
  const {
    isOpen: showHistory,
    previewedVersion,
    count: revisionCount,
  } = revisions;
  const analysis = useClipPreviewAnalysis({
    clip,
    transformedText,
    typesEnabled: features.types,
    transformationsEnabled: features.transformations,
    transcriptionsEnabled: features.transcriptions,
    canRunTransforms: viewPolicy.canRunManualTransforms,
    canMutateContent: viewPolicy.canMutateContent,
    filePreviewMode,
    filePreviewMaxMb,
    onRevisionAdded: revisions.noteRevisionAdded,
    onUpdateClip: () => onUpdateClip(),
    onError: (message) => showToast({ tone: 'error', message }),
  });
  const {
    contentMatches,
    inspection,
    smartActions,
    fileSearchableText,
    extractionResults,
    extractionHistory,
    extractionHistoryHasMore,
    isExtractionHistoryLoading,
    isFileExtractionLoading,
    filePreviews,
    isFilePreviewLoading,
    isOcrLoading,
    resolvedImage,
    loadExtractionHistory,
    runOcr: handleRunOCR,
    runFileExtraction: handleRunFileExtraction,
  } = analysis;

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
    void activeTransformExecutionRef.current?.cancel();
    activeTransformExecutionRef.current = null;
    setTransformClientRequestId(null);
    pipelineRequestIdRef.current += 1;
    setTransformedText(null);
    setActiveManualTransformRef(null);
    setActiveManualTransformName(null);
    setActiveTransformRef(null);
    setActiveTransformName(null);
    setIsWorkflowMenuOpen(false);
    setTransformPreviewOutcome(null);
    setIsManualTransformRunning(false);
    setManualTransformAction(null);
    setManualTransformError(null);
    setCopied(false);
    setCopiedFormat(null);
    if (copiedTimerRef.current) clearTimeout(copiedTimerRef.current);
    if (copiedFormatTimerRef.current) clearTimeout(copiedFormatTimerRef.current);
  }, [clip]);

  useEffect(() => {
    if (viewPolicy.canRunManualTransforms) return;
    setTransformedText(null);
    setActiveManualTransformRef(null);
    setActiveManualTransformName(null);
    setManualTransformAction(null);
    setManualTransformError(null);
  }, [viewPolicy.canRunManualTransforms]);

  useEffect(() => () => {
    if (copiedTimerRef.current) clearTimeout(copiedTimerRef.current);
    if (copiedFormatTimerRef.current) clearTimeout(copiedFormatTimerRef.current);
  }, []);

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
  const protectedByBin = Boolean(clip.protecting_bin_ids?.length);
  const protectionToggleDisabled = Boolean(clip.hotkey) || protectedByBin;
  const concealment = clipConcealmentPolicy(clip, bins, contentTypes);

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

  const handlePreviewManualTransform = async (manualTransform: ManualTransform) => {
    if (!clip.text_content) return;
    revisions.clearPreview();
    const requestId = ++pipelineRequestIdRef.current;
    setActiveManualTransformRef(manualTransform.stableRef);
    setActiveManualTransformName(manualTransform.name);
    setActiveTransformRef(null);
    setActiveTransformName(null);
    setTransformPreviewOutcome(null);
    setTransformedText(null);
    setIsManualTransformRunning(true);
    setManualTransformAction(null);
    setManualTransformError(null);
    try {
      const execution = startTransformation(
        clip.text_content,
        { kind: 'manual_transform', transformRef: manualTransform.stableRef },
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
      setManualTransformError(e instanceof Error ? e.message : translate('component.clipPreview.advancedTransformFailedToRun'));
    } finally {
      if (requestId === pipelineRequestIdRef.current) {
        activeTransformExecutionRef.current = null;
        setTransformClientRequestId(null);
        setIsManualTransformRunning(false);
      }
    }
  };

  const handlePreviewTransform = async (transform: SavedTransform) => {
    if (!clip.text_content) return;
    revisions.clearPreview();
    const requestId = ++pipelineRequestIdRef.current;
    setActiveTransformRef(transform.stableRef);
    setActiveTransformName(transform.name);
    setActiveManualTransformRef(null);
    setActiveManualTransformName(null);
    setTransformedText(null);
    setTransformPreviewOutcome(null);
    setIsManualTransformRunning(true);
    setManualTransformAction(null);
    setManualTransformError(null);
    revisions.clearPreview();
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
      setManualTransformError(error instanceof Error ? error.message : translate('component.clipPreview.transformFailedToRun'));
    } finally {
      if (requestId === pipelineRequestIdRef.current) {
        activeTransformExecutionRef.current = null;
        setTransformClientRequestId(null);
        setIsManualTransformRunning(false);
      }
    }
  };

  const handleApplyTransform = async () => {
    if (!activeTransformRef || !transformPreviewOutcome || transformedText === null || !clip.text_content) return;
    setIsManualTransformRunning(true);
    setManualTransformError(null);
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
      revisions.noteRevisionAdded();
      soundManager.playCopySound();
      handleResetTransform();
      onUpdateClip();
    } catch (error) {
      setManualTransformError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsManualTransformRunning(false);
    }
  };

  const handleResetTransform = () => {
    void activeTransformExecutionRef.current?.cancel();
    activeTransformExecutionRef.current = null;
    setTransformClientRequestId(null);
    pipelineRequestIdRef.current += 1;
    setTransformedText(null);
    setActiveManualTransformRef(null);
    setActiveManualTransformName(null);
    setActiveTransformRef(null);
    setActiveTransformName(null);
    setTransformPreviewOutcome(null);
    setManualTransformAction(null);
    setManualTransformError(null);
    revisions.clearPreview();
  };

  const handleRetryTransform = () => {
    if (activeTransformRef) {
      const transform = transforms.find((candidate) => candidate.stableRef === activeTransformRef);
      if (transform) void handlePreviewTransform(transform);
      return;
    }
    if (activeManualTransformRef) {
      const manualTransform = manualTransforms.find((candidate) => candidate.stableRef === activeManualTransformRef);
      if (manualTransform) void handlePreviewManualTransform(manualTransform);
    }
  };

  const handleManualTransformOutput = async (destination: 'copy' | 'paste') => {
    if (transformedText === null) return;
    try {
      if (destination === 'copy') {
        await invoke('copy_clip_to_system', { text: transformedText, imageBase64: null });
        setManualTransformAction('copied');
        soundManager.playCopySound();
      } else {
        await invoke('paste_text_to_frontmost', { text: transformedText });
        setManualTransformAction('pasted');
        soundManager.playPasteSound();
      }
      setManualTransformError(null);
    } catch (error) {
      console.error(`Failed to ${destination} Advanced Transform output:`, error);
      setManualTransformError(translate('component.clipPreview.couldNotDestinationTheAdvancedTransformResult', { destination: destination }));
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

  const inspectedText = transformedText === null ? inspection?.result.text : undefined;
  const charCount = inspectedText?.characterCount ?? displayText.length;
  const wordCount = inspectedText?.wordCount ?? (displayText.trim() ? displayText.trim().split(/\s+/).length : 0);
  const lineCount = inspectedText?.lineCount ?? (displayText ? displayText.split('\n').length : 0);

  return (
    <div className="flex-1 col-preview h-screen flex flex-col overflow-hidden">
      {/* Finder Top Header Bar */}
      <div
        onMouseDown={startWindowDrag}
        onDoubleClick={handleWindowDragDoubleClick}
        className="col-preview-header h-[60px] px-4 flex items-center justify-between cursor-default titlebar-drag-handle shrink-0"
      >
        <div className="me-3 flex min-w-0 flex-1 flex-nowrap items-center gap-3 overflow-hidden whitespace-nowrap titlebar-drag-handle">
          {features.clipTypes && <span className="clip-type-badge theme-badge shrink-0 whitespace-nowrap text-xs font-semibold px-2.5 py-1 rounded-md border capitalize titlebar-drag-handle">
            {clip.content_type === 'file' && getClipFilePaths(clip).length > 1
              ? translate('component.clipPreview.files')
              : contentTypeLabel(structuralClipType(clip.content_type))}
          </span>}
          {features.types && visibleContentTypes.map((contentType) => (
            <span
              key={contentType}
              title={contentMatchTitle(contentType, contentMatches)}
              className="clip-type-badge theme-badge shrink-0 whitespace-nowrap text-xs font-semibold px-2.5 py-1 rounded-md border titlebar-drag-handle"
            >
              {contentTypeLabel(contentType)}
            </span>
          ))}
          {features.types && hiddenContentTypes.length > 0 && (
            <span
              title={hiddenContentTypes.map(contentTypeLabel).join(', ')}
              className="clip-type-badge theme-badge shrink-0 whitespace-nowrap text-xs font-semibold px-2.5 py-1 rounded-md border titlebar-drag-handle"
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
              className="transform-accent manual-transforms h-4 w-4 shrink-0"
              aria-label={translate('component.clipPreview.transformedWithTransformname', { transformName: provenance.transformName })}
            />
          )}
          {features.transformations && !isTransforming && provenance?.connectionId && (
            <Sparkles
              className="transform-accent manual-transforms h-3.5 w-3.5 shrink-0"
              aria-label={translate('component.clipPreview.transformUsedConnectedIntelligence')}
            />
          )}
        </div>

        <div className="clip-preview-actions relative flex shrink-0 items-center titlebar-no-drag">
          {features.transformations && viewPolicy.canRunManualTransforms && canTransformContent && (
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
                {isManualTransformRunning && activeTransformRef
                  ? <LoaderCircle className="h-4 w-4 animate-spin" />
                  : <Workflow className="h-4 w-4" />}
              </button>
              {isWorkflowMenuOpen && (
                <ClipWorkflowMenu
                  transforms={transforms}
                  activeTransformRef={activeTransformRef}
                  isRunning={isManualTransformRunning}
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
          {features.concealment && viewPolicy.canOrganize && <button
            type="button"
            onClick={() => onToggleConcealed(clip.id)}
            className={`clip-preview-action preview-conceal-btn theme-focusable transition-colors ${concealment.effective ? 'is-active' : ''}`}
            title={concealment.effective
              ? translate('component.clipCard.revealSensitiveText')
              : translate('action.conceal')}
            aria-label={concealment.effective
              ? translate('component.clipCard.revealSensitiveText')
              : translate('action.conceal')}
            aria-pressed={concealment.effective}
          >
            {concealment.effective ? <Eye /> : <EyeOff />}
          </button>}

          {features.naming && viewPolicy.canOrganize && <button
            type="button"
            onClick={() => onName(clip)}
            className={`clip-preview-action preview-name-btn theme-focusable transition-colors ${clip.name ? 'is-active' : ''}`}
            title={clip.name ? translate('action.editName') : translate('action.nameClip')}
            aria-label={clip.name ? translate('action.editName') : translate('action.nameClip')}
          >
            <FilePenLine />
          </button>}

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
                  ? translate('component.clipCard.protectedByHotkey')
                  : protectedByBin
                    ? translate('component.clipPreview.protectedByBin')
                    : clip.is_protected ? UI_COPY.unprotect : UI_COPY.protect}
                aria-label={clip.hotkey
                  ? translate('component.clipCard.protectedByHotkey')
                  : protectedByBin
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

      {features.naming && clip.name && (
        <button
          type="button"
          onClick={() => onName(clip)}
          disabled={!viewPolicy.canOrganize}
          className="preview-name-row theme-named-text flex w-full items-center gap-2.5 border-b px-4 py-3 text-start disabled:cursor-default"
          title={viewPolicy.canOrganize ? translate('action.editName') : clip.name}
          aria-label={viewPolicy.canOrganize ? translate('action.editName') : clip.name}
        >
          <FilePenLine className="h-5 w-5 shrink-0" />
          <OverflowText text={clip.name} className="min-w-0 truncate text-lg font-semibold" />
        </button>
      )}

      {features.notes && (
        <ClipPreviewNotesPanel
          controller={notesController}
          readOnly={!viewPolicy.canEditNotes}
        />
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
        {(activeManualTransformName || previewedVersion) && (
          <div className="active-filter-banner flex items-center justify-between px-3 py-2 border rounded-lg">
            <div className="flex items-center space-x-2">
              <Sliders className="w-4 h-4" />
              <span className="flex items-center gap-1.5">
                <span>
                {previewedVersion
                  ? translate('component.clipPreview.previewingRevision')
                  : isManualTransformRunning
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
                  : activeManualTransformName}</strong>
              </span>
            </div>
            {(activeManualTransformName || previewedVersion) && (
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
      {features.transformations && viewPolicy.canRunManualTransforms && canTransformContent && smartActions && smartActions.result.actions.length > 0 && (
          <div className="smart-actions-bar px-4 py-2 flex items-center justify-between text-xs space-x-2 overflow-x-auto">
            <div className="smart-actions-heading flex items-center space-x-1.5 shrink-0 font-semibold text-[11px]">
              <Lightbulb className="w-3.5 h-3.5" />
              <span>{translate('component.clipPreview.smartActionsSignals', { signals: smartActions.result.signalLabels.join(', ') })}</span>
            </div>
            <div className="flex items-center space-x-1.5 overflow-x-auto scrollbar-none py-0.5">
              {smartActions.result.actions.map((action) => {
                const transform = transforms.find((candidate) => candidate.stableRef === action.transformRef);
                const manualTransform = manualTransforms.find((candidate) => candidate.stableRef === action.transformRef);
                if (!transform && !manualTransform) return null;
                return (
                  <button
                    key={action.transformRef}
                    onClick={() => transform ? handlePreviewTransform(transform) : void handlePreviewManualTransform(manualTransform!)}
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

      {viewPolicy.canRunManualTransforms && canTransformContent && activeTransformRef && activeTransformName && (
        <ClipTransformBar
          activeTransformName={activeTransformName}
          isRunning={isManualTransformRunning}
          hasPreview={transformedText !== null && transformedText !== (clip.text_content || '') && Boolean(transformPreviewOutcome)}
          error={pipelineError}
          onApply={() => void handleApplyTransform()}
          onRetry={handleRetryTransform}
          onReset={handleResetTransform}
          requestStatus={transformClientRequestId ? transformRequestStatus : undefined}
        />
      )}

      {/* Advanced Transform selector */}
      {viewPolicy.canRunManualTransforms && canTransformContent && manualTransforms.length > 0 && (
        <div className="preview-filter-bar px-4 py-2.5 border-t select-none">
          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center space-x-2 shrink-0">
              <Sliders className="preview-filter-accent w-4 h-4" />
              <span className="theme-text-main text-xs font-semibold">{translate('component.clipPreview.advancedTransform')}</span>
            </div>

            <div className="max-w-xs flex-1">
              <MenuSelect
                value={activeManualTransformRef || ''}
                onChange={(selectedRef) => {
                  if (!selectedRef) {
                    handleResetTransform();
                  } else {
                    const found = manualTransforms.find((manualTransform) => manualTransform.stableRef === selectedRef);
                    if (found) void handlePreviewManualTransform(found);
                  }
                }}
                label={translate('component.clipPreview.chooseAdvancedTransform')}
                className={`w-full ${activeManualTransformRef ? 'preview-filter-select-active' : 'form-field-valid'}`}
                searchable
                searchPlaceholder={translate('component.clipPreview.searchManualTransforms')}
                options={[
                  { value: '', get label() { return translate('component.clipPreview.originalClip'); } },
                  ...manualTransforms.map((manualTransform) => ({ value: manualTransform.stableRef, label: manualTransform.name })),
                ]}
              />
            </div>

            {/* Reset Action */}
            {activeManualTransformRef && (
              <div className="flex items-center gap-1.5 shrink-0">
                <button
                  type="button"
                  onClick={() => void handleManualTransformOutput('copy')}
                  disabled={isManualTransformRunning || transformedText === null}
                  className="theme-secondary-button flex items-center gap-1.5 rounded-lg border px-2.5 py-1 text-xs font-semibold transition-colors disabled:cursor-not-allowed disabled:opacity-40"
                  title={translate('component.clipPreview.copyResult')}
                >
                  {pipelineAction === 'copied' ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
                  <span>{pipelineAction === 'copied' ? translate('action.copied') : translate('action.copy')}</span>
                </button>
                <button
                  type="button"
                  onClick={() => void handleManualTransformOutput('paste')}
                  disabled={isManualTransformRunning || transformedText === null}
                  className="transform-workspace-action manual-transforms flex items-center gap-1.5 rounded-lg px-2.5 py-1 text-xs font-bold transition-colors disabled:cursor-not-allowed disabled:opacity-40"
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
          versions={revisions.versions}
          isLoading={revisions.isLoading}
          readOnly={!viewPolicy.canMutateContent}
          onClose={() => revisions.setIsOpen(false)}
          previewedVersionId={previewedVersion?.id ?? null}
          restoringVersionId={revisions.restoringVersionId}
          hasMore={revisions.hasMore}
          isLoadingMore={revisions.isLoadingMore}
          onLoadMore={() => void revisions.loadMore()}
          onPreview={revisions.togglePreview}
          onRestore={(version) => void revisions.restore(version)}
        />
      )}

      <ClipPreviewFooter
        clip={clip}
        inspection={inspection}
        fileFormatsEnabled={features.fileFormats}
        revisionsEnabled={features.revisions}
        characterCount={charCount}
        wordCount={wordCount}
        lineCount={lineCount}
        revisionCount={revisionCount}
        showHistory={showHistory}
        relativeTimeNow={relativeTimeNow}
        onToggleHistory={revisions.toggleOpen}
      />

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
