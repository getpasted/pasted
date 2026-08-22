import { useEffect, useRef, useState } from 'react';

import { useIntelligenceRequestStatus } from './useIntelligenceRequestStatus';
import { useClipPreviewRevisions } from './useClipPreviewRevisions';
import { translate } from '../localization/runtime';
import type {
  ClipItem,
  ClipTransformationProvenance,
  ManualTransform,
  SavedTransform,
  TransformationExecutionOutcome,
} from '../types';
import { soundManager } from '../utils/sound';
import { safeInvoke as invoke } from '../utils/tauri';
import { startTransformation, type TransformationExecutionHandle } from '../utils/transformExecution';

export function useClipPreviewTransforms({
  canMutateContent,
  canRunManualTransforms,
  clip,
  manualTransforms,
  onUpdateClip,
  revisionsEnabled,
}: {
  canMutateContent: boolean;
  canRunManualTransforms: boolean;
  clip: ClipItem | null;
  manualTransforms: ManualTransform[];
  onUpdateClip: (updatedClip?: ClipItem) => void;
  revisionsEnabled: boolean;
}) {
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
  const [pipelineAction, setPipelineAction] = useState<'copied' | 'pasted' | null>(null);
  const [pipelineError, setPipelineError] = useState<string | null>(null);
  const requestIdRef = useRef(0);
  const activeExecutionRef = useRef<TransformationExecutionHandle | null>(null);
  const [clientRequestId, setClientRequestId] = useState<string | null>(null);
  const requestStatus = useIntelligenceRequestStatus(clientRequestId);

  const resetPreviewState = () => {
    setTransformedText(null);
    setActiveManualTransformRef(null);
    setActiveManualTransformName(null);
    setActiveTransformRef(null);
    setActiveTransformName(null);
    setTransformPreviewOutcome(null);
    setPipelineAction(null);
    setPipelineError(null);
  };

  const invalidateActiveExecution = () => {
    requestIdRef.current += 1;
    void activeExecutionRef.current?.cancel();
    activeExecutionRef.current = null;
    setClientRequestId(null);
    setIsManualTransformRunning(false);
  };

  const revisions = useClipPreviewRevisions({
    clip,
    enabled: revisionsEnabled,
    canRestore: canMutateContent,
    onBeforeRestore: () => {
      invalidateActiveExecution();
      resetPreviewState();
    },
    onUpdateClip,
  });

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
    invalidateActiveExecution();
    resetPreviewState();
    setIsWorkflowMenuOpen(false);
  }, [clip]);

  useEffect(() => {
    if (canRunManualTransforms) return;
    invalidateActiveExecution();
    resetPreviewState();
    setIsWorkflowMenuOpen(false);
    revisions.clearPreview();
  }, [canRunManualTransforms]);

  useEffect(() => () => {
    requestIdRef.current += 1;
    void activeExecutionRef.current?.cancel();
  }, []);

  const startExecution = (
    input: string,
    target: { kind: 'manual_transform' | 'transform'; transformRef: string },
    clipId: number,
  ) => {
    void activeExecutionRef.current?.cancel();
    const execution = startTransformation(input, target, { sourceClipId: clipId });
    activeExecutionRef.current = execution;
    setClientRequestId(execution.clientRequestId);
    return execution;
  };

  const previewManualTransform = async (manualTransform: ManualTransform) => {
    if (!clip?.text_content) return;
    revisions.clearPreview();
    const requestId = ++requestIdRef.current;
    setActiveManualTransformRef(manualTransform.stableRef);
    setActiveManualTransformName(manualTransform.name);
    setActiveTransformRef(null);
    setActiveTransformName(null);
    setTransformPreviewOutcome(null);
    setTransformedText(null);
    setIsManualTransformRunning(true);
    setPipelineAction(null);
    setPipelineError(null);
    try {
      const result = await startExecution(
        clip.text_content,
        { kind: 'manual_transform', transformRef: manualTransform.stableRef },
        clip.id,
      ).promise;
      if (requestId === requestIdRef.current) setTransformedText(result.output);
    } catch (error) {
      if (requestId !== requestIdRef.current) return;
      console.error(error);
      setPipelineError(error instanceof Error
        ? error.message
        : translate('component.clipPreview.advancedTransformFailedToRun'));
    } finally {
      if (requestId === requestIdRef.current) {
        activeExecutionRef.current = null;
        setClientRequestId(null);
        setIsManualTransformRunning(false);
      }
    }
  };

  const previewTransform = async (transform: SavedTransform) => {
    if (!clip?.text_content) return;
    revisions.clearPreview();
    const requestId = ++requestIdRef.current;
    setActiveTransformRef(transform.stableRef);
    setActiveTransformName(transform.name);
    setActiveManualTransformRef(null);
    setActiveManualTransformName(null);
    setTransformedText(null);
    setTransformPreviewOutcome(null);
    setIsManualTransformRunning(true);
    setPipelineAction(null);
    setPipelineError(null);
    try {
      const result = await startExecution(
        clip.text_content,
        { kind: 'transform', transformRef: transform.stableRef },
        clip.id,
      ).promise;
      if (requestId !== requestIdRef.current) return;
      setTransformPreviewOutcome(result);
      setTransformedText(result.output);
    } catch (error) {
      if (requestId !== requestIdRef.current) return;
      setPipelineError(error instanceof Error
        ? error.message
        : translate('component.clipPreview.transformFailedToRun'));
    } finally {
      if (requestId === requestIdRef.current) {
        activeExecutionRef.current = null;
        setClientRequestId(null);
        setIsManualTransformRunning(false);
      }
    }
  };

  const resetTransform = () => {
    invalidateActiveExecution();
    resetPreviewState();
    revisions.clearPreview();
  };

  const applyTransform = async () => {
    if (!clip?.text_content || !activeTransformRef || !transformPreviewOutcome || transformedText === null) return;
    setIsManualTransformRunning(true);
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
      revisions.noteRevisionAdded();
      soundManager.playCopySound();
      resetTransform();
      onUpdateClip();
    } catch (error) {
      setPipelineError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsManualTransformRunning(false);
    }
  };

  const retryTransform = () => {
    const transform = transforms.find((candidate) => candidate.stableRef === activeTransformRef);
    if (transform) {
      void previewTransform(transform);
      return;
    }
    const manualTransform = manualTransforms.find((candidate) => candidate.stableRef === activeManualTransformRef);
    if (manualTransform) void previewManualTransform(manualTransform);
  };

  const outputTransform = async (destination: 'copy' | 'paste') => {
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
      setPipelineError(translate(
        'component.clipPreview.couldNotDestinationTheAdvancedTransformResult',
        { destination },
      ));
    }
  };

  return {
    activeManualTransformName,
    activeManualTransformRef,
    activeTransformName,
    activeTransformRef,
    applyTransform,
    clientRequestId,
    isManualTransformRunning,
    isWorkflowMenuOpen,
    outputTransform,
    pipelineAction,
    pipelineError,
    previewManualTransform,
    previewTransform,
    provenance,
    requestStatus,
    resetTransform,
    retryTransform,
    revisions,
    setIsWorkflowMenuOpen,
    setTransformedText,
    transformedText,
    transformPreviewOutcome,
    transforms,
  };
}

export type ClipPreviewTransformController = ReturnType<typeof useClipPreviewTransforms>;
