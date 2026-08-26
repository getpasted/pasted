import React, { useState, useEffect, useRef } from 'react';
import type { Operation, ManualTransform, SavedTransform, TransformDefinition } from '../types';
import { transformsApi } from '../api/transforms';
import { safeInvoke as invoke } from '../utils/tauri';
import { ManualTransformEditorModal } from './ManualTransformEditorModal';
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
import { translate } from '../localization/runtime';

interface TransformationsViewProps {
  manualTransforms: ManualTransform[];
  onRefreshManualTransforms: () => void;
  activeWorkspace: TransformWorkspace;
  onActiveWorkspaceChange: (workspace: TransformWorkspace) => void;
}

export const TransformationsView: React.FC<TransformationsViewProps> = ({
  manualTransforms: externalManualTransforms,
  onRefreshManualTransforms,
  activeWorkspace,
  onActiveWorkspaceChange,
}) => {
  const { showToast } = useToast();
  const [activeLibraryFilter, setActiveLibraryFilter] = useState('all');

  const [selectedManualTransformForEdit, setSelectedManualTransformForEdit] = useState<ManualTransform | null>(null);
  const [selectedTransformForEdit, setSelectedTransformForEdit] = useState<SavedTransform | null>(null);
  const [isEditorModalOpen, setIsEditorModalOpen] = useState(false);
  const [isComposerModalOpen, setIsComposerModalOpen] = useState(false);
  const [testText, setTestText] = useState('Hello there! :) https://example.com?utm_source=test');
  const [testResult, setTestResult] = useState('');
  const [testError, setTestError] = useState('');
  const [transforms, setTransforms] = useState<SavedTransform[]>([]);
  const [manualTransforms, setManualTransforms] = useState<ManualTransform[]>(externalManualTransforms);
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

  const handleOpenCreateModal = () => {
    setSelectedManualTransformForEdit(null);
    setIsEditorModalOpen(true);
  };

  const handleOpenTransformComposer = (transform: SavedTransform | null = null) => {
    setSelectedTransformForEdit(transform);
    setIsComposerModalOpen(true);
  };

  const handleOpenEditModal = (manualTransform: ManualTransform) => {
    setSelectedManualTransformForEdit(manualTransform);
    setIsEditorModalOpen(true);
  };

  const handleDuplicateManualTransform = async (manualTransform: ManualTransform) => {
    try {
      await transformsApi.createManual({
        name: `${manualTransform.name} (Copy)`,
        steps: manualTransform.steps,
        hotkey: null,
      });
      soundManager.playCopySound();
      onRefreshManualTransforms();
      fetchTransforms();
    } catch (e) {
      showActionError(e);
    }
  };

  const handleDeleteManualTransform = async (pipelineRef: string) => {
    setIsDeleting(true);
    try {
      await transformsApi.deleteManual(pipelineRef);
      onRefreshManualTransforms();
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
    onActiveWorkspaceChange('playground');
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
        setManualTransforms(definitions.flatMap((definition) => definition.authoringKind === 'manual'
          ? [{
              id: definition.id,
              stableRef: definition.stableRef,
              name: definition.name,
              steps: definition.steps,
              hotkey: definition.hotkey,
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
    { value: 'all', get label() { return translate('component.transformationsView.allTransforms'); }, count: transforms.length + manualTransforms.length },
    {
      value: 'local',
      get label() { return translate('component.transformationsView.localReplayable'); },
      count: manualTransforms.length + transforms.filter((transform) => transform.plan.steps.every((step) => step.executor.kind === 'deterministic')).length,
    },
    {
      value: 'assisted',
      get label() { return translate('component.transformationsView.aiAssisted'); },
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
  }, [externalManualTransforms]);

  useEffect(() => {
    if (playgroundTarget) return;
    if (transforms[0]) setPlaygroundTarget({ kind: 'transform', item: transforms[0] });
    else if (operations[0]) setPlaygroundTarget({ kind: 'operation', item: operations[0] });
    else if (manualTransforms[0]) setPlaygroundTarget({ kind: 'manual_transform', item: manualTransforms[0] });
  }, [operations, manualTransforms, playgroundTarget, transforms]);

  return (
    <div className="tools-page filters-page flex-1 h-screen flex flex-col overflow-hidden select-none filter-manager-wrapper">
      <TransformWorkspaceHeader
        activeWorkspace={activeWorkspace}
        transformCount={transforms.length + manualTransforms.length}
        operationCount={operations.length}
        onChange={onActiveWorkspaceChange}
      />

      {/* Main Scrollable Content */}
      <div data-pasted-scroll-key={`transformations:${activeWorkspace}`} className="tools-scroll-region flex-1 overflow-y-auto p-6 space-y-6">
      {activeWorkspace === 'advanced' ? (
        <OperationsManager
          isEmbedded={true}
          onChooseOperation={(operation) => choosePlaygroundTarget({ kind: 'operation', item: operation })}
        />
      ) : activeWorkspace === 'playground' ? (
        <TransformationPlayground
          transforms={transforms}
          operations={operations}
          manualTransforms={manualTransforms}
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
          manualTransforms={manualTransforms}
          operations={operations}
          filter={activeLibraryFilter}
          filterOptions={libraryFilterOptions}
          onFilterChange={setActiveLibraryFilter}
          onCreateTransform={() => handleOpenTransformComposer()}
          onCreateManualTransform={handleOpenCreateModal}
          onTestTransform={(transform) => choosePlaygroundTarget({ kind: 'transform', item: transform })}
          onTestManualTransform={(manualTransform) => choosePlaygroundTarget({ kind: 'manual_transform', item: manualTransform })}
          onEditTransform={handleOpenTransformComposer}
          onEditManualTransform={handleOpenEditModal}
          onDuplicateTransform={(transform) => void handleDuplicateTransform(transform)}
          onDuplicateManualTransform={(manualTransform) => void handleDuplicateManualTransform(manualTransform)}
          onDeleteTransform={(transform) => setDeleteTarget({ kind: 'Transform', storage: 'saved', name: transform.name, ref: transform.stableRef })}
          onDeleteManualTransform={(manualTransform) => setDeleteTarget({ kind: 'Transform', storage: 'manual', name: manualTransform.name, ref: manualTransform.stableRef })}
          onManualTransformHotkeyChange={(manualTransform, hotkey) => {
            void transformsApi.updateManualHotkey(manualTransform.stableRef, hotkey)
              .then(onRefreshManualTransforms)
              .then(fetchTransforms)
              .catch(showActionError);
          }}
        />
      )}

      </div>

      {/* Editor Modal */}
      <ManualTransformEditorModal
        manualTransform={selectedManualTransformForEdit}
        isOpen={isEditorModalOpen}
        onClose={() => setIsEditorModalOpen(false)}
        onSaveSuccess={() => {
          onRefreshManualTransforms();
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
          ? handleDeleteManualTransform(deleteTarget.ref)
          : deleteTarget
            ? handleDeleteTransform(deleteTarget.ref)
            : undefined}
      />
    </div>
  );
};
