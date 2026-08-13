import React, { useState, useEffect, useRef } from 'react';
import type { Operation, Pipeline, SavedTransform, TransformDefinition } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { PipelineEditorModal } from './PipelineEditorModal';
import { OperationsManager } from './OperationsManager';
import { soundManager } from '../utils/sound';
import { TransformWorkspaceHeader, type TransformWorkspace } from './TransformWorkspaceHeader';
import { TransformComposerModal } from './TransformComposerModal';
import type { PlaygroundRunState } from './PlaygroundRunStatus';
import { TransformationPlayground, type PlaygroundTarget } from './TransformationPlayground';
import { startTransformation, type TransformationExecutionHandle } from '../utils/transformExecution';
import { useIntelligenceRequestStatus } from '../hooks/useIntelligenceRequestStatus';
import { DeleteTransformationAssetDialog } from './DeleteTransformationAssetDialog';
import { useToast } from './ToastProvider';
import { TransformationLibrary } from './TransformationLibrary';

interface TransformationsViewProps {
  pipelines: Pipeline[];
  onRefreshPipelines: () => void;
  requestedWorkspace?: TransformWorkspace;
  navigationKey?: number;
}

export const TransformationsView: React.FC<TransformationsViewProps> = ({
  pipelines: externalPipelines,
  onRefreshPipelines,
  requestedWorkspace,
  navigationKey,
}) => {
  const { showToast } = useToast();
  const [activeSubTab, setActiveSubTab] = useState<TransformWorkspace>('transforms');
  const [activeLibraryFilter, setActiveLibraryFilter] = useState('all');

  const [selectedPipelineForEdit, setSelectedPipelineForEdit] = useState<Pipeline | null>(null);
  const [selectedTransformForEdit, setSelectedTransformForEdit] = useState<SavedTransform | null>(null);
  const [isEditorModalOpen, setIsEditorModalOpen] = useState(false);
  const [isComposerModalOpen, setIsComposerModalOpen] = useState(false);
  const [testText, setTestText] = useState('Hello there! :) https://example.com?utm_source=test');
  const [testResult, setTestResult] = useState('');
  const [testError, setTestError] = useState('');
  const [transforms, setTransforms] = useState<SavedTransform[]>([]);
  const [pipelines, setPipelines] = useState<Pipeline[]>(externalPipelines);
  const [playgroundTarget, setPlaygroundTarget] = useState<PlaygroundTarget | null>(null);
  const [playgroundRunState, setPlaygroundRunState] = useState<PlaygroundRunState>('idle');
  const [playgroundDurationMs, setPlaygroundDurationMs] = useState<number>();
  const playgroundRequestId = useRef(0);
  const playgroundExecution = useRef<TransformationExecutionHandle | null>(null);
  const [playgroundClientRequestId, setPlaygroundClientRequestId] = useState<string | null>(null);
  const playgroundRequestStatus = useIntelligenceRequestStatus(playgroundClientRequestId);
  const [deleteTarget, setDeleteTarget] = useState<
    | { kind: 'Transform'; storage: 'saved' | 'manual'; name: string; ref: string }
    | null
  >(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const showActionError = (error: unknown) => showToast({
    tone: 'error',
    message: error instanceof Error ? error.message : String(error),
    durationMs: 8000,
  });

  useEffect(() => {
    if (requestedWorkspace) setActiveSubTab(requestedWorkspace);
  }, [navigationKey, requestedWorkspace]);

  const handleOpenCreateModal = () => {
    setSelectedPipelineForEdit(null);
    setIsEditorModalOpen(true);
  };

  const handleOpenTransformComposer = (transform: SavedTransform | null = null) => {
    setSelectedTransformForEdit(transform);
    setIsComposerModalOpen(true);
  };

  const handleOpenEditModal = (pipeline: Pipeline) => {
    setSelectedPipelineForEdit(pipeline);
    setIsEditorModalOpen(true);
  };

  const handleDuplicatePipeline = async (pipeline: Pipeline) => {
    try {
      await invoke('create_pipeline', {
        name: `${pipeline.name} (Copy)`,
        steps: pipeline.steps,
        shortcut: null,
      });
      soundManager.playCopySound();
      onRefreshPipelines();
      fetchTransforms();
    } catch (e) {
      showActionError(e);
    }
  };

  const handleDeletePipeline = async (pipelineRef: string) => {
    setIsDeleting(true);
    try {
      await invoke('delete_pipeline', { pipelineRef });
      onRefreshPipelines();
      fetchTransforms();
      setDeleteTarget(null);
    } catch (e) {
      showActionError(e);
    } finally {
      setIsDeleting(false);
    }
  };

  const choosePlaygroundTarget = (target: PlaygroundTarget) => {
    setPlaygroundTarget(target);
    setPlaygroundRunState('idle');
    setTestError('');
    setActiveSubTab('playground');
  };

  const runPlayground = async () => {
    if (!playgroundTarget || playgroundRunState === 'running') return;
    const requestId = ++playgroundRequestId.current;
    const startedAt = performance.now();
    setPlaygroundRunState('running');
    setPlaygroundDurationMs(undefined);
    setTestError('');
    try {
      const target = playgroundTarget.kind === 'transform'
        ? { kind: 'transform' as const, transformRef: playgroundTarget.item.stableRef }
        : playgroundTarget.kind === 'operation'
          ? { kind: 'operation' as const, operationRef: playgroundTarget.item.stable_id }
          : { kind: 'transform' as const, transformRef: playgroundTarget.item.stableRef };
      const execution = startTransformation(testText, target);
      playgroundExecution.current = execution;
      setPlaygroundClientRequestId(execution.clientRequestId);
      const res = await execution.promise;
      if (requestId !== playgroundRequestId.current) return;
      setTestResult(res.output);
      setPlaygroundRunState('success');
      setPlaygroundDurationMs(res.durationMs || performance.now() - startedAt);
    } catch (e) {
      if (requestId !== playgroundRequestId.current) return;
      if (String(e).includes('execution_cancelled')) {
        setPlaygroundRunState('cancelled');
        return;
      }
      setTestError(String(e));
      setPlaygroundRunState('error');
    } finally {
      if (requestId === playgroundRequestId.current) {
        playgroundExecution.current = null;
        setPlaygroundClientRequestId(null);
      }
    }
  };

  const fetchTransforms = () => {
    invoke<TransformDefinition[]>('get_transforms')
      .then((definitions) => {
        setTransforms(definitions.flatMap((definition) => definition.authoringKind === 'intent' && definition.plan
          ? [{
              id: definition.id,
              stableRef: definition.stableRef,
              name: definition.name,
              plan: definition.plan,
              connectionId: definition.connectionId,
              revision: definition.revision,
              createdAt: definition.createdAt,
              updatedAt: definition.updatedAt,
            }]
          : []));
        setPipelines(definitions.flatMap((definition) => definition.authoringKind === 'manual'
          ? [{
              id: definition.id,
              stableRef: definition.stableRef,
              name: definition.name,
              steps: definition.steps,
              shortcut: definition.shortcut,
              revision: definition.revision,
              createdAt: definition.createdAt,
              updatedAt: definition.updatedAt,
            }]
          : []));
      })
      .catch(showActionError);
  };

  const cancelPlayground = () => {
    void playgroundExecution.current?.cancel();
    playgroundExecution.current = null;
    setPlaygroundClientRequestId(null);
    playgroundRequestId.current += 1;
    setPlaygroundRunState('cancelled');
  };

  const handleDeleteTransform = async (transformRef: string) => {
    setIsDeleting(true);
    try {
      await invoke('delete_saved_transform', { transformRef });
      setTransforms((current) => current.filter((transform) => transform.stableRef !== transformRef));
      setDeleteTarget(null);
    } catch (error) {
      showActionError(error);
    } finally {
      setIsDeleting(false);
    }
  };

  const handleDuplicateTransform = async (transform: SavedTransform) => {
    try {
      const duplicate = await invoke<SavedTransform>('save_saved_transform', {
        name: `${transform.name} Copy`,
        plan: transform.plan,
        connectionId: transform.connectionId,
      });
      setTransforms((current) => [duplicate, ...current]);
    } catch (error) {
      setTestError(String(error));
    }
  };

  const [operations, setOperations] = useState<Operation[]>([]);

  const libraryFilterOptions = [
    { value: 'all', label: 'All Transforms', count: transforms.length + pipelines.length },
    {
      value: 'local',
      label: 'Local · Replayable',
      count: pipelines.length + transforms.filter((transform) => transform.plan.steps.every((step) => step.executor.kind === 'deterministic')).length,
    },
    {
      value: 'assisted',
      label: 'AI-assisted',
      count: transforms.filter((transform) => transform.plan.steps.some((step) => step.executor.kind === 'semantic')).length,
    },
  ].filter((option) => option.value === 'all' || option.count > 0);
  const fetchOpCount = () => {
    invoke<Operation[]>('get_operations')
      .then(setOperations)
      .catch(showActionError);
  };

  useEffect(() => {
    fetchOpCount();
    fetchTransforms();
  }, []);

  useEffect(() => {
    fetchTransforms();
  }, [externalPipelines]);

  useEffect(() => {
    if (playgroundTarget) return;
    if (transforms[0]) setPlaygroundTarget({ kind: 'transform', item: transforms[0] });
    else if (operations[0]) setPlaygroundTarget({ kind: 'operation', item: operations[0] });
    else if (pipelines[0]) setPlaygroundTarget({ kind: 'pipeline', item: pipelines[0] });
  }, [operations, pipelines, playgroundTarget, transforms]);

  return (
    <div className="tools-page filters-page flex-1 h-screen flex flex-col overflow-hidden select-none filter-manager-wrapper">
      <TransformWorkspaceHeader
        activeWorkspace={activeSubTab}
        transformCount={transforms.length + pipelines.length}
        operationCount={operations.length}
        onChange={setActiveSubTab}
      />

      {/* Main Scrollable Content */}
      <div className="tools-scroll-region flex-1 overflow-y-auto p-6 space-y-6">
      {activeSubTab === 'advanced' ? (
        <OperationsManager
          isEmbedded={true}
          onChooseOperation={(operation) => choosePlaygroundTarget({ kind: 'operation', item: operation })}
        />
      ) : activeSubTab === 'playground' ? (
        <TransformationPlayground
          transforms={transforms}
          operations={operations}
          pipelines={pipelines}
          target={playgroundTarget}
          input={testText}
          output={testResult}
          error={testError}
          runState={playgroundRunState}
          runDurationMs={playgroundDurationMs}
          onTargetChange={(target) => {
            setPlaygroundTarget(target);
            setPlaygroundRunState('idle');
            setTestError('');
          }}
          onInputChange={setTestText}
          onRun={() => void runPlayground()}
          onRetry={() => void runPlayground()}
          onStop={cancelPlayground}
          requestStatus={playgroundRequestStatus}
        />
      ) : (
        <TransformationLibrary
          transforms={transforms}
          pipelines={pipelines}
          operations={operations}
          filter={activeLibraryFilter}
          filterOptions={libraryFilterOptions}
          onFilterChange={setActiveLibraryFilter}
          onCreateTransform={() => handleOpenTransformComposer()}
          onCreatePipeline={handleOpenCreateModal}
          onTestTransform={(transform) => choosePlaygroundTarget({ kind: 'transform', item: transform })}
          onTestPipeline={(pipeline) => choosePlaygroundTarget({ kind: 'pipeline', item: pipeline })}
          onEditTransform={handleOpenTransformComposer}
          onEditPipeline={handleOpenEditModal}
          onDuplicateTransform={(transform) => void handleDuplicateTransform(transform)}
          onDuplicatePipeline={(pipeline) => void handleDuplicatePipeline(pipeline)}
          onDeleteTransform={(transform) => setDeleteTarget({ kind: 'Transform', storage: 'saved', name: transform.name, ref: transform.stableRef })}
          onDeletePipeline={(pipeline) => setDeleteTarget({ kind: 'Transform', storage: 'manual', name: pipeline.name, ref: pipeline.stableRef })}
          onPipelineShortcutChange={(pipeline, shortcut) => {
            void invoke('update_pipeline_shortcut', { pipelineRef: pipeline.stableRef, shortcut })
              .then(onRefreshPipelines)
              .then(fetchTransforms)
              .catch(showActionError);
          }}
        />
      )}

      </div>

      {/* Editor Modal */}
      <PipelineEditorModal
        pipeline={selectedPipelineForEdit}
        isOpen={isEditorModalOpen}
        onClose={() => setIsEditorModalOpen(false)}
        onSaveSuccess={() => {
          onRefreshPipelines();
          fetchTransforms();
        }}
      />
      <TransformComposerModal
        isOpen={isComposerModalOpen}
        sampleInput={testText}
        transform={selectedTransformForEdit}
        onClose={() => {
          setIsComposerModalOpen(false);
          setSelectedTransformForEdit(null);
        }}
        onTestResult={(result) => {
          setTestError('');
          setTestResult(result.output);
        }}
        onTransformSaved={(transform) => {
          setTransforms((current) => [transform, ...current.filter((item) => item.stableRef !== transform.stableRef)]);
          setSelectedTransformForEdit(transform);
        }}
      />
      <DeleteTransformationAssetDialog
        asset={deleteTarget}
        isDeleting={isDeleting}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={() => deleteTarget?.storage === 'manual'
          ? handleDeletePipeline(deleteTarget.ref)
          : deleteTarget
            ? handleDeleteTransform(deleteTarget.ref)
            : undefined}
      />
    </div>
  );
};
