import React, { useState, useEffect, useRef } from 'react';
import type { ExecutePlanOutcome, Operation, Pipeline, SavedTransform } from '../types';
import { Trash2, Code2, Edit3, Copy, Play, Download, Plus, Sparkles, Workflow } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { PipelineEditorModal } from './PipelineEditorModal';
import { OperationsManager } from './OperationsManager';
import { HotkeyRecorder } from './HotkeyRecorder';
import { soundManager } from '../utils/sound';
import { TransformWorkspaceHeader, type TransformWorkspace } from './TransformWorkspaceHeader';
import { TransformLibraryToolbar } from './TransformLibraryToolbar';
import { TransformCategorySelect } from './TransformCategorySelect';
import { TransformComposerModal } from './TransformComposerModal';
import type { PlaygroundRunState } from './PlaygroundRunStatus';
import { TransformationPlayground, type PlaygroundTarget } from './TransformationPlayground';
import { FloatingActionStrip } from './FloatingActionStrip';

interface TransformationsViewProps {
  pipelines: Pipeline[];
  onRefreshPipelines: () => void;
}

export const TransformationsView: React.FC<TransformationsViewProps> = ({ pipelines, onRefreshPipelines }) => {
  const [activeSubTab, setActiveSubTab] = useState<TransformWorkspace>('transforms');
  const [activeLibraryFilter, setActiveLibraryFilter] = useState('all');

  const FILTER_CATEGORIES = [
    'All',
    'Cleaners & Sanitizers',
    'Case Transformations',
    'Smart Formatting',
    'Data Extraction',
    'Line Operations',
    'Structure & Formatting',
    'Encodings & Decodings',
    'Advanced & Shell Scripts',
  ];
  const [selectedPipelineForEdit, setSelectedPipelineForEdit] = useState<Pipeline | null>(null);
  const [selectedTransformForEdit, setSelectedTransformForEdit] = useState<SavedTransform | null>(null);
  const [isEditorModalOpen, setIsEditorModalOpen] = useState(false);
  const [isComposerModalOpen, setIsComposerModalOpen] = useState(false);
  const [testText, setTestText] = useState('Hello Pasted User! :) https://example.com?utm_source=test');
  const [testResult, setTestResult] = useState('');
  const [testError, setTestError] = useState('');
  const [transforms, setTransforms] = useState<SavedTransform[]>([]);
  const [playgroundTarget, setPlaygroundTarget] = useState<PlaygroundTarget | null>(null);
  const [playgroundRunState, setPlaygroundRunState] = useState<PlaygroundRunState>('idle');
  const [playgroundDurationMs, setPlaygroundDurationMs] = useState<number>();
  const playgroundRequestId = useRef(0);
  const [pipelineContextMenu, setPipelineContextMenu] = useState<{ x: number; y: number; pipeline: Pipeline } | null>(null);
  const [transformContextMenu, setTransformContextMenu] = useState<{ x: number; y: number; transform: SavedTransform } | null>(null);

  useEffect(() => {
    const handleClick = () => {
      setPipelineContextMenu(null);
      setTransformContextMenu(null);
    };
    window.addEventListener('click', handleClick);
    return () => window.removeEventListener('click', handleClick);
  }, []);

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
      soundManager.playCopySound(true);
      onRefreshPipelines();
    } catch (e) {
      console.error(e);
    }
  };

  const handleExportPipeline = async (pipeline: Pipeline) => {
    try {
      const exportJson = JSON.stringify(pipeline, null, 2);
      await invoke('copy_clip_to_system', { text: exportJson, imageBase64: null });
      soundManager.playCopySound(true);
    } catch (e) {
      console.error(e);
    }
  };

  const handleDeletePipeline = async (pipelineRef: string) => {
    try {
      await invoke('delete_pipeline', { pipelineRef });
      onRefreshPipelines();
    } catch (e) {
      console.error(e);
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
      const res = playgroundTarget.kind === 'transform'
        ? await invoke<ExecutePlanOutcome>('execute_saved_transform', {
          transformRef: playgroundTarget.item.stableRef,
          input: testText,
        })
        : await invoke<{ output: string }>('execute_transformation', {
          request: {
            input: testText,
            target: playgroundTarget.kind === 'operation'
              ? { kind: 'operation', operationRef: playgroundTarget.item.stable_id }
              : { kind: 'pipeline', pipelineRef: playgroundTarget.item.stableRef },
            sourceClipId: null,
            trigger: 'manual',
            destination: 'preview',
          },
        });
      if (requestId !== playgroundRequestId.current) return;
      setTestResult(res.output);
      setPlaygroundRunState('success');
      setPlaygroundDurationMs(
        playgroundTarget.kind === 'transform'
          ? (res as ExecutePlanOutcome).durationMs
          : performance.now() - startedAt,
      );
    } catch (e) {
      if (requestId !== playgroundRequestId.current) return;
      setTestError(String(e));
      setPlaygroundRunState('error');
    }
  };

  const fetchTransforms = () => {
    invoke<SavedTransform[]>('get_saved_transforms')
      .then(setTransforms)
      .catch(console.error);
  };

  const stopWaitingForPlayground = () => {
    playgroundRequestId.current += 1;
    setPlaygroundRunState('stopped');
  };

  const handleDeleteTransform = async (transformRef: string) => {
    try {
      await invoke('delete_saved_transform', { transformRef });
      setTransforms((current) => current.filter((transform) => transform.stableRef !== transformRef));
    } catch (error) {
      setTestError(String(error));
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
    { value: 'saved', label: 'Saved Transforms', count: transforms.length },
    { value: 'pipelines', label: 'Legacy Pipelines', count: pipelines.length },
    ...FILTER_CATEGORIES.filter((category) => category !== 'All').map((category) => ({
      value: `pipeline:${category}`,
      label: `Pipelines · ${category}`,
      count: pipelines.filter((pipeline) => pipeline.steps.some((step) => (
        operations.find((operation) => operation.stable_id === step.operationRef)?.category === category
      ))).length,
    })),
  ].filter((option) => !option.value.startsWith('pipeline:') || option.count > 0);
  const showSavedTransforms = activeLibraryFilter === 'all' || activeLibraryFilter === 'saved';
  const showLegacyPipelines = activeLibraryFilter === 'all'
    || activeLibraryFilter === 'pipelines'
    || activeLibraryFilter.startsWith('pipeline:');
  const activePipelineCategory = activeLibraryFilter.startsWith('pipeline:')
    ? activeLibraryFilter.slice('pipeline:'.length)
    : null;

  const fetchOpCount = () => {
    invoke<Operation[]>('get_operations')
      .then(setOperations)
      .catch(console.error);
  };

  useEffect(() => {
    fetchOpCount();
    fetchTransforms();
  }, []);

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
        transformCount={transforms.length}
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
          onStop={stopWaitingForPlayground}
        />
      ) : (
        <>
          <div className="transform-library-column min-w-0 space-y-6">
          <TransformLibraryToolbar
            accent="pipelines"
            createLabel="New Transform"
            onCreate={() => handleOpenTransformComposer()}
          >
            <TransformCategorySelect
              accent="pipelines"
              value={activeLibraryFilter}
              options={libraryFilterOptions}
              onChange={setActiveLibraryFilter}
              label="Filter Transforms"
            />
          </TransformLibraryToolbar>

          {showSavedTransforms && <section className="space-y-3" aria-labelledby="saved-transforms-heading">
            <div className="flex items-baseline gap-2 px-1">
                <h3 id="saved-transforms-heading" className="text-xs font-semibold theme-text-main">Saved Transforms</h3>
                <span className="text-[10px] theme-text-subtle">Ready to reuse with the same plan</span>
            </div>
            {transforms.length === 0 ? (
              <div className="theme-card-idle rounded-xl border border-dashed px-4 py-5 text-center">
                <Workflow className="transform-accent pipelines mx-auto mb-2 h-5 w-5" />
                <p className="text-xs font-semibold theme-text-main">No saved Transforms yet</p>
                <p className="mt-1 text-[10px] theme-text-muted">Create and test a draft, then save it here.</p>
              </div>
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                {transforms.map((transform) => {
                  const semanticSteps = transform.plan.steps.filter((step) => step.executor.kind === 'semantic').length;
                  const provenance = semanticSteps > 0
                    ? (transform.connectionId ? 'AI-assisted · pinned connection' : 'AI-assisted · automatic connection')
                    : 'Local · replayable';
                  return (
                    <div
                      key={transform.stableRef}
                      role="button"
                      tabIndex={0}
                      onClick={() => choosePlaygroundTarget({ kind: 'transform', item: transform })}
                      onContextMenu={(event) => {
                        event.preventDefault();
                        event.stopPropagation();
                        setTransformContextMenu({ x: event.clientX, y: event.clientY, transform });
                      }}
                      onKeyDown={(event) => {
                        if (event.key === 'Enter' || event.key === ' ') {
                          event.preventDefault();
                          choosePlaygroundTarget({ kind: 'transform', item: transform });
                        }
                      }}
                      className="transform-card pipelines group relative flex cursor-pointer items-center justify-between rounded-xl border p-3.5 shadow-md transition-[background-color,border-color,box-shadow,transform] theme-card-idle"
                    >
                      <div className="flex min-w-0 items-center gap-3 pr-2">
                        <span className="theme-badge grid h-9 w-9 shrink-0 place-items-center rounded-lg border">
                          <Workflow className="transform-accent pipelines h-4 w-4" />
                        </span>
                        <span className="min-w-0">
                          <span className="block truncate text-xs font-bold theme-text-main">{transform.name}</span>
                          <span className="mt-1 flex min-w-0 items-center gap-1.5 text-[10px] theme-text-muted">
                            <span>{transform.plan.steps.length} {transform.plan.steps.length === 1 ? 'step' : 'steps'}</span>
                            <span>·</span>
                            <span className="inline-flex min-w-0 items-center gap-1 truncate">
                              {semanticSteps > 0 && <Sparkles className="h-3 w-3" />}
                              <span className="truncate">{provenance}</span>
                            </span>
                            {transform.revision > 1 && <><span>·</span><span>v{transform.revision}</span></>}
                          </span>
                        </span>
                      </div>
                      <FloatingActionStrip label="Transform actions" revealOnGroupInteraction>
                        <button
                          type="button"
                          onClick={(event) => {
                            event.stopPropagation();
                            handleOpenTransformComposer(transform);
                          }}
                          className="floating-action-button"
                          title="Edit Transform"
                        >
                          <Edit3 className="h-3.5 w-3.5" />
                        </button>
                        <button
                          type="button"
                          onClick={(event) => {
                            event.stopPropagation();
                            void handleDuplicateTransform(transform);
                          }}
                          className="floating-action-button"
                          title="Duplicate Transform"
                        >
                          <Copy className="h-3.5 w-3.5" />
                        </button>
                        <button
                          type="button"
                          onClick={(event) => {
                            event.stopPropagation();
                            void handleDeleteTransform(transform.stableRef);
                          }}
                          className="floating-action-button is-danger"
                          title="Delete Transform"
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </button>
                      </FloatingActionStrip>
                    </div>
                  );
                })}
              </div>
            )}
          </section>}

          {showLegacyPipelines && <section className="space-y-3" aria-labelledby="legacy-pipelines-heading">
            <div className="flex items-center justify-between gap-3 px-1">
              <div className="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-1">
                <h3 id="legacy-pipelines-heading" className="text-xs font-semibold theme-text-main">Legacy Pipelines</h3>
                <span className="text-[10px] theme-text-subtle">Deterministic building blocks remain available</span>
              </div>
              <button
                type="button"
                onClick={handleOpenCreateModal}
                className="theme-secondary-button flex h-8 shrink-0 items-center gap-1.5 rounded-xl border px-3 text-xs font-semibold transition-colors"
              >
                <Plus className="h-3.5 w-3.5" />
                <span>New Pipeline</span>
              </button>
            </div>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              {pipelines
                .filter((f) => {
                  if (!activePipelineCategory) return true;
                  return f.steps.some((step) => operations.find((operation) => operation.stable_id === step.operationRef)?.category === activePipelineCategory);
                })
                .map((f) => {
                  const stepTypes = f.steps.map((step) => (
                    operations.find((operation) => operation.stable_id === step.operationRef)?.name
                    || step.operationRef.replace(/^(?:builtin|custom):/, '')
                  ));

                  return (
                    <div
                      key={f.id}
                      role="button"
                      tabIndex={0}
                      aria-label={`Preview ${f.name} Pipeline`}
                      onClick={() => choosePlaygroundTarget({ kind: 'pipeline', item: f })}
                      onKeyDown={(event) => {
                        if (event.key === 'Enter' || event.key === ' ') {
                          event.preventDefault();
                          choosePlaygroundTarget({ kind: 'pipeline', item: f });
                        }
                      }}
                      onContextMenu={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        setPipelineContextMenu({ x: e.clientX, y: e.clientY, pipeline: f });
                      }}
                      className="transform-card pipelines group p-3.5 theme-card-idle rounded-xl border cursor-pointer transition-[background-color,border-color,box-shadow,transform] flex items-center justify-between shadow-md"
                    >
                      <div className="flex items-center space-x-3 truncate pr-2">
                        <div className="p-2 rounded-lg theme-badge border shrink-0">
                          <Code2 className="transform-accent pipelines w-4 h-4" />
                        </div>
                        <div className="truncate">
                          <div className="flex items-center space-x-2">
                            <h4 className="text-xs font-bold theme-text-main truncate">{f.name}</h4>
                            {stepTypes.length > 1 && (
                              <span className="transform-tag pipelines text-[9px] font-bold border px-1.5 py-0.2 rounded-full">
                                ⚡ {stepTypes.length} Steps
                              </span>
                            )}
                          </div>
                          <div className="flex items-center space-x-1.5 mt-1 overflow-x-auto scrollbar-none">
                            {stepTypes.map((st, i) => (
                              <React.Fragment key={i}>
                                {i > 0 && <span className="transform-accent pipelines text-[10px] opacity-60 font-bold">➔</span>}
                                <span className="transform-tag pipelines text-[10px] font-mono px-1.5 py-0.5 rounded border whitespace-nowrap">
                                  {st}
                                </span>
                              </React.Fragment>
                            ))}
                          </div>
                        </div>
                      </div>

                  <div className="flex items-center space-x-2 shrink-0">
                    <div onClick={(e) => e.stopPropagation()}>
                      <HotkeyRecorder
                        value={f.shortcut}
                        onChange={async (newShortcut) => {
                          try {
                            await invoke('update_pipeline_shortcut', { pipelineRef: f.stableRef, shortcut: newShortcut });
                            onRefreshPipelines();
                          } catch (err) {
                            console.error(err);
                          }
                        }}
                      />
                    </div>

                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleOpenEditModal(f);
                      }}
                      className="theme-icon-button p-1.5 border rounded-md transition-colors"
                      title="Edit Pipeline"
                    >
                      <Edit3 className="w-4 h-4" />
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleDeletePipeline(f.stableRef);
                      }}
                      className="theme-icon-button theme-danger-text p-1.5 border rounded-md transition-colors"
                      title="Delete Pipeline"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                </div>
              );
              })}
            </div>
          </section>}
          </div>

          {transformContextMenu && (
            <div
              className="theme-menu fixed w-48 rounded-xl border p-1.5 text-xs font-medium select-none animate-in fade-in duration-100"
              style={{
                top: Math.min(transformContextMenu.y, window.innerHeight - 160),
                left: Math.min(transformContextMenu.x, window.innerWidth - 205),
              }}
              onClick={(event) => event.stopPropagation()}
              role="menu"
            >
              <div className="theme-text-muted px-3 py-1 text-[10px] font-bold uppercase truncate">
                {transformContextMenu.transform.name}
              </div>
              <button
                type="button"
                onClick={() => {
                  handleOpenTransformComposer(transformContextMenu.transform);
                  setTransformContextMenu(null);
                }}
                className="theme-menu-item flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-left"
              >
                <Edit3 className="h-3.5 w-3.5" />
                <span>Edit Transform</span>
              </button>
              <button
                type="button"
                onClick={() => {
                  void handleDuplicateTransform(transformContextMenu.transform);
                  setTransformContextMenu(null);
                }}
                className="theme-menu-item flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-left"
              >
                <Copy className="h-3.5 w-3.5" />
                <span>Duplicate Transform</span>
              </button>
              <button
                type="button"
                onClick={() => {
                  choosePlaygroundTarget({ kind: 'transform', item: transformContextMenu.transform });
                  setTransformContextMenu(null);
                }}
                className="theme-menu-item flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-left"
              >
                <Play className="h-3.5 w-3.5" />
                <span>Test in Playground</span>
              </button>
              <div className="theme-menu-divider my-1 border-t" />
              <button
                type="button"
                onClick={() => {
                  void handleDeleteTransform(transformContextMenu.transform.stableRef);
                  setTransformContextMenu(null);
                }}
                className="theme-menu-item theme-danger-text flex w-full items-center gap-2 rounded-md px-3 py-1.5 text-left"
              >
                <Trash2 className="h-3.5 w-3.5" />
                <span>Delete Transform</span>
              </button>
            </div>
          )}

          {/* Pipeline context menu */}
          {pipelineContextMenu && (
            <div
              className="theme-menu fixed w-48 rounded-xl border p-1.5 text-xs font-medium select-none animate-in fade-in duration-100"
              style={{ top: pipelineContextMenu.y, left: pipelineContextMenu.x }}
              onClick={(e) => e.stopPropagation()}
              role="menu"
            >
              <div className="theme-text-muted theme-divider px-3 py-1 text-[10px] uppercase font-bold border-b truncate">
                {pipelineContextMenu.pipeline.name}
              </div>

              <button
                onClick={() => {
                  handleOpenEditModal(pipelineContextMenu.pipeline);
                  setPipelineContextMenu(null);
                }}
                className="theme-menu-item flex w-full items-center space-x-2 rounded-md px-3 py-1.5 text-left"
              >
                <Edit3 className="w-3.5 h-3.5" />
                <span>Edit Pipeline</span>
              </button>

              <button
                onClick={() => {
                  handleDuplicatePipeline(pipelineContextMenu.pipeline);
                  setPipelineContextMenu(null);
                }}
                className="theme-menu-item flex w-full items-center space-x-2 rounded-md px-3 py-1.5 text-left"
              >
                <Copy className="w-3.5 h-3.5" />
                <span>Duplicate Pipeline</span>
              </button>

              <button
                onClick={() => {
                  choosePlaygroundTarget({ kind: 'pipeline', item: pipelineContextMenu.pipeline });
                  setPipelineContextMenu(null);
                }}
                className="theme-menu-item flex w-full items-center space-x-2 rounded-md px-3 py-1.5 text-left"
              >
                <Play className="w-3.5 h-3.5" />
                <span>Test in Playground</span>
              </button>

              <button
                onClick={() => {
                  handleExportPipeline(pipelineContextMenu.pipeline);
                  setPipelineContextMenu(null);
                }}
                className="theme-menu-item flex w-full items-center space-x-2 rounded-md px-3 py-1.5 text-left"
              >
                <Download className="w-3.5 h-3.5" />
                <span>Export / Copy JSON</span>
              </button>

              <div className="theme-menu-divider my-1 border-t" />

              <button
                onClick={() => {
                  handleDeletePipeline(pipelineContextMenu.pipeline.stableRef);
                  setPipelineContextMenu(null);
                }}
                className="theme-menu-item theme-danger-text flex w-full items-center space-x-2 rounded-md px-3 py-1.5 text-left"
              >
                <Trash2 className="w-3.5 h-3.5" />
                <span>Delete Pipeline</span>
              </button>
            </div>
          )}
        </>
      )}

      </div>

      {/* Editor Modal */}
      <PipelineEditorModal
        pipeline={selectedPipelineForEdit}
        isOpen={isEditorModalOpen}
        onClose={() => setIsEditorModalOpen(false)}
        onSaveSuccess={onRefreshPipelines}
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
    </div>
  );
};
