import React, { useState, useEffect, useRef } from 'react';
import { ClipItem, Bin, ManualTransform } from '../types';
import type { AppSettings } from '../types';
import { parseColor, ColorFormats } from '../utils/color';
import { soundManager } from '../utils/sound';
import { ClipPreviewRevisionHistoryPanel } from './ClipPreviewRevisionHistoryPanel';
import { ClipPreviewFooter } from './ClipPreviewFooter';
import { ClipNoteViewer } from './ClipNoteViewer';
import { safeInvoke as invoke } from '../utils/tauri';
import { useClipPreviewNotes } from '../hooks/useClipPreviewNotes';
import { useClipPreviewAnalysis } from '../hooks/useClipPreviewAnalysis';
import { useClipPreviewTransforms } from '../hooks/useClipPreviewTransforms';
import type { ClipViewPolicy } from '../utils/clipViewPolicy';
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
import { ClipPreviewEmptyState } from './ClipPreviewEmptyState';

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
  const transformController = useClipPreviewTransforms({
    clip,
    manualTransforms,
    revisionsEnabled: features.revisions,
    canMutateContent: viewPolicy.canMutateContent,
    canRunManualTransforms: viewPolicy.canRunManualTransforms,
    onUpdateClip,
  });
  const {
    activeManualTransformName,
    activeManualTransformRef,
    activeTransformName,
    activeTransformRef,
    applyTransform: handleApplyTransform,
    clientRequestId: transformClientRequestId,
    isManualTransformRunning,
    isWorkflowMenuOpen,
    pipelineAction,
    pipelineError,
    previewManualTransform: handlePreviewManualTransform,
    previewTransform: handlePreviewTransform,
    provenance,
    requestStatus: transformRequestStatus,
    resetTransform: handleResetTransform,
    retryTransform: handleRetryTransform,
    revisions,
    setIsWorkflowMenuOpen,
    setTransformedText,
    transformedText,
    transformPreviewOutcome,
    transforms,
    outputTransform: handleManualTransformOutput,
  } = transformController;
  const {
    isOpen: showHistory,
    previewedVersion,
    count: versionCount,
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
    onRefreshRevisionCount: revisions.refreshCount,
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
    setCopied(false);
    setCopiedFormat(null);
    if (copiedTimerRef.current) clearTimeout(copiedTimerRef.current);
    if (copiedFormatTimerRef.current) clearTimeout(copiedFormatTimerRef.current);
  }, [clip]);

  useEffect(() => () => {
    if (copiedTimerRef.current) clearTimeout(copiedTimerRef.current);
    if (copiedFormatTimerRef.current) clearTimeout(copiedFormatTimerRef.current);
  }, []);

  if (!clip) {
    return <ClipPreviewEmptyState />;
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
        canSaveVersion={viewPolicy.canMutateContent}
        contentProps={{
          clip,
          displayText,
          previewingRevision: previewedVersion !== null,
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
          onColorChange: setTransformedText,
          onCopyFormat: (label, value) => void handleCopySpecificFormat(label, value),
          onRunOCR: () => void handleRunOCR(),
          onRunFileExtraction: () => void handleRunFileExtraction(),
          onLoadExtractionHistory: (reset) => void loadExtractionHistory(reset),
          ...analysis.previewContentAnalysisProps,
          visualLabels: previewedVersion
            ? previewedVersion.visual_labels ?? { clipId: clip.id, labels: [], hasOverrides: false }
            : analysis.previewContentAnalysisProps.visualLabels,
          readOnly: !viewPolicy.canMutateContent || previewedVersion !== null,
        }}
        isManualTransformRunning={isManualTransformRunning}
        isTransforming={isTransforming}
        isSavingVersion={revisions.restoringVersionId === previewedVersion?.id}
        onCancelVersionPreview={revisions.clearPreview}
        onOpenIntelligence={onOpenIntelligence}
        onResetTransform={handleResetTransform}
        onSaveVersion={() => {
          if (previewedVersion) void revisions.restore(previewedVersion);
        }}
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

      <ClipPreviewRevisionHistoryPanel
        visible={features.revisions && showHistory && clip.content_type !== 'file'}
        readOnly={!viewPolicy.canMutateContent}
        revisions={revisions}
      />

      <ClipPreviewFooter
        clip={clip}
        inspection={inspection}
        fileFormatsEnabled={features.fileFormats}
        revisionsEnabled={features.revisions}
        characterCount={charCount}
        wordCount={wordCount}
        lineCount={lineCount}
        versionCount={versionCount}
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
