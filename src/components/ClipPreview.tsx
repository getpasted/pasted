import React, { useState, useEffect, useRef } from 'react';
import { ClipItem, Bin, ManualTransform } from '../types';
import type { AppSettings } from '../types';
import type { ClipTransformationProvenance, TransformationExecutionOutcome, SavedTransform } from '../types';
import { parseColor, ColorFormats } from '../utils/color';
import { soundManager } from '../utils/sound';
import { ClipRevisionHistory } from './ClipRevisionHistory';
import { ClipPreviewFooter } from './ClipPreviewFooter';
import { ClipNoteViewer } from './ClipNoteViewer';
import { FileText } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { useClipPreviewNotes } from '../hooks/useClipPreviewNotes';
import { useClipPreviewRevisions } from '../hooks/useClipPreviewRevisions';
import { useClipPreviewAnalysis } from '../hooks/useClipPreviewAnalysis';
import type { ClipViewPolicy } from '../utils/clipViewPolicy';
import { startTransformation, type TransformationExecutionHandle } from '../utils/transformExecution';
import { useIntelligenceRequestStatus } from '../hooks/useIntelligenceRequestStatus';
import { useFeatures } from '../hooks/useFeatures';
import { useToast } from './ToastProvider';
import { translate } from '../localization/runtime';
import { localizedSourceName } from '../localization/presentation';
import { useContentTypes } from './ContentTypeProvider';
import { clipConcealmentPolicy } from '../utils/clipConcealment';
import { ClipPreviewHeader } from './ClipPreviewHeader';
import { ClipPreviewOrganization } from './ClipPreviewOrganization';
import { ClipPreviewTransformControls } from './ClipPreviewTransformControls';
import { ClipPreviewWorkspace } from './ClipPreviewWorkspace';

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
      <ClipPreviewHeader
        activeTransformRef={activeTransformRef}
        canTransformContent={canTransformContent}
        clip={clip}
        concealmentEffective={concealment.effective}
        contentMatches={contentMatches}
        copied={copied}
        features={features}
        hiddenContentTypes={hiddenContentTypes}
        isAddingNote={isAddingNote}
        isManualTransformRunning={isManualTransformRunning}
        isTransforming={isTransforming}
        isWorkflowMenuOpen={isWorkflowMenuOpen}
        onCopy={() => void handleCopy()}
        onCloseWorkflowMenu={() => setIsWorkflowMenuOpen(false)}
        onDelete={() => onDeleteClip(clip.id)}
        onManageTransforms={onOpenTransformations}
        onName={() => onName(clip)}
        onPreviewTransform={(transform) => void handlePreviewTransform(transform)}
        onToggleConcealed={() => onToggleConcealed(clip.id)}
        onToggleNote={handleToggleAddNote}
        onTogglePin={() => onTogglePin(clip.id)}
        onToggleProtected={() => onToggleProtected(clip.id)}
        onToggleWorkflowMenu={() => setIsWorkflowMenuOpen((current) => !current)}
        protectedByBin={protectedByBin}
        protectionToggleDisabled={protectionToggleDisabled}
        provenance={provenance}
        transforms={transforms}
        trashEnabled={trashEnabled}
        viewPolicy={viewPolicy}
        visibleContentTypes={visibleContentTypes}
        workflowTriggerRef={workflowTriggerRef}
      />

      <ClipPreviewOrganization
        bins={bins}
        clip={clip}
        features={features}
        notesController={notesController}
        onAssignBin={(binId) => void handleAssignBin(binId)}
        onHotkeyChange={(hotkey) => void handleHotkeyChange(hotkey)}
        onName={() => onName(clip)}
        onRemoveBin={(binId) => void onRemoveBin(clip.id, binId)}
        viewedBinId={viewedBinId}
        viewPolicy={viewPolicy}
      />

      <ClipPreviewWorkspace
        activeManualTransformName={activeManualTransformName}
        contentProps={{
          clip,
          displayText,
          colorData,
          resolvedImageBase64: resolvedImage?.clipId === clip.id ? resolvedImage.base64 : null,
          filePreviews,
          isFilePreviewLoading,
          fileSearchableText,
          extractionResults,
          extractionHistory,
          extractionHistoryHasMore,
          isExtractionHistoryLoading,
          isFileExtractionLoading,
          copiedFormat,
          isOcrLoading,
          ocrEnabled: features.ocr,
          transcriptionsEnabled: features.transcriptions,
          readOnly: !viewPolicy.canMutateContent,
          onColorChange: setTransformedText,
          onCopyFormat: (label, value) => void handleCopySpecificFormat(label, value),
          onRunOCR: () => void handleRunOCR(),
          onRunFileExtraction: () => void handleRunFileExtraction(),
          onLoadExtractionHistory: (reset) => void loadExtractionHistory(reset),
        }}
        isManualTransformRunning={isManualTransformRunning}
        isTransforming={isTransforming}
        onOpenIntelligence={onOpenIntelligence}
        onResetTransform={handleResetTransform}
        previewedVersion={previewedVersion}
        transformError={transformError}
        transformRequestStatus={transformRequestStatus}
      />

      <ClipPreviewTransformControls
        activeManualTransformRef={activeManualTransformRef}
        activeTransformName={activeTransformName}
        activeTransformRef={activeTransformRef}
        canRunManualTransforms={viewPolicy.canRunManualTransforms}
        canTransformContent={canTransformContent}
        hasTransformPreview={transformedText !== null
          && transformedText !== (clip.text_content || '')
          && Boolean(transformPreviewOutcome)}
        isManualTransformRunning={isManualTransformRunning}
        manualTransforms={manualTransforms}
        onApplyTransform={() => void handleApplyTransform()}
        onManualTransformOutput={(destination) => void handleManualTransformOutput(destination)}
        onPreviewManualTransform={(transform) => void handlePreviewManualTransform(transform)}
        onPreviewTransform={(transform) => void handlePreviewTransform(transform)}
        onResetTransform={handleResetTransform}
        onRetryTransform={handleRetryTransform}
        pipelineAction={pipelineAction}
        pipelineError={pipelineError}
        requestStatus={transformClientRequestId ? transformRequestStatus : undefined}
        showSmartActions={features.transformations}
        smartActions={smartActions}
        transformedText={transformedText}
        transforms={transforms}
      />

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
