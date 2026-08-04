import React, { useState, useEffect } from 'react';
import type { ExecutePlanOutcome, Pipeline, TransformationRecipe } from '../types';
import { Trash2, Code2, Edit3, Copy, Play, Download, LoaderCircle, Sparkles } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { PipelineEditorModal } from './PipelineEditorModal';
import { OperationsManager } from './OperationsManager';
import { HotkeyRecorder } from './HotkeyRecorder';
import { soundManager } from '../utils/sound';
import { TransformWorkspaceHeader, type TransformWorkspace } from './TransformWorkspaceHeader';
import { TransformLibraryToolbar } from './TransformLibraryToolbar';
import { TransformationOutputActions } from './TransformationOutputActions';
import { IntentRecipeComposer } from './IntentRecipeComposer';

interface TransformationsViewProps {
  pipelines: Pipeline[];
  onRefreshPipelines: () => void;
}

export const TransformationsView: React.FC<TransformationsViewProps> = ({ pipelines, onRefreshPipelines }) => {
  const [activeSubTab, setActiveSubTab] = useState<TransformWorkspace>('recipes');
  const [activeCategory, setActiveCategory] = useState<string>('All');

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
  const [isEditorModalOpen, setIsEditorModalOpen] = useState(false);
  const [testText, setTestText] = useState('Hello Pasted User! :) https://example.com?utm_source=test');
  const [testResult, setTestResult] = useState('');
  const [testError, setTestError] = useState('');
  const [recipes, setRecipes] = useState<TransformationRecipe[]>([]);
  const [runningRecipeRef, setRunningRecipeRef] = useState('');
  const [pipelineContextMenu, setPipelineContextMenu] = useState<{ x: number; y: number; pipeline: Pipeline } | null>(null);

  useEffect(() => {
    const handleClick = () => setPipelineContextMenu(null);
    window.addEventListener('click', handleClick);
    return () => window.removeEventListener('click', handleClick);
  }, []);

  const handleOpenCreateModal = () => {
    setSelectedPipelineForEdit(null);
    setIsEditorModalOpen(true);
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

  const handleTestTransformation = async (pipeline: Pipeline) => {
    try {
      setTestError('');
      const res = await invoke<{ output: string }>('execute_transformation', {
        request: {
          input: testText,
          target: { kind: 'pipeline', pipelineRef: pipeline.stableRef },
          sourceClipId: null,
          trigger: 'manual',
        },
      });
      setTestResult(res.output);
    } catch (e) {
      setTestError(String(e));
    }
  };

  const fetchRecipes = () => {
    invoke<TransformationRecipe[]>('get_transformation_recipes')
      .then(setRecipes)
      .catch(console.error);
  };

  const handleRunRecipe = async (recipe: TransformationRecipe) => {
    if (runningRecipeRef) return;
    setRunningRecipeRef(recipe.stableRef);
    setTestError('');
    try {
      const result = await invoke<ExecutePlanOutcome>('execute_transformation_recipe', {
        recipeRef: recipe.stableRef,
        input: testText,
      });
      setTestResult(result.output);
    } catch (error) {
      setTestError(String(error));
    } finally {
      setRunningRecipeRef('');
    }
  };

  const handleDeleteRecipe = async (recipeRef: string) => {
    try {
      await invoke('delete_transformation_recipe', { recipeRef });
      setRecipes((current) => current.filter((recipe) => recipe.stableRef !== recipeRef));
    } catch (error) {
      setTestError(String(error));
    }
  };

  const [operations, setOperations] = useState<Array<{ stable_id: string; name: string; category: string }>>([]);

  const fetchOpCount = () => {
    invoke<Array<{ stable_id: string; name: string; category: string }>>('get_operations')
      .then(setOperations)
      .catch(console.error);
  };

  useEffect(() => {
    fetchOpCount();
    fetchRecipes();
  }, [activeSubTab]);

  return (
    <div className="tools-page filters-page flex-1 h-screen flex flex-col overflow-hidden select-none filter-manager-wrapper">
      <TransformWorkspaceHeader
        activeWorkspace={activeSubTab}
        filterCount={recipes.length}
        operationCount={operations.length}
        onChange={setActiveSubTab}
      />

      {/* Main Scrollable Content */}
      <div className="tools-scroll-region flex-1 overflow-y-auto p-6 space-y-6">
      {activeSubTab === 'advanced' ? (
        <OperationsManager isEmbedded={true} />
      ) : (
        <>
          <IntentRecipeComposer
            sampleInput={testText}
            onTestResult={(result) => {
              setTestError('');
              setTestResult(result.output);
            }}
            onRecipeSaved={(recipe) => {
              setRecipes((current) => [recipe, ...current.filter((item) => item.stableRef !== recipe.stableRef)]);
            }}
          />

          {/* Sticky Filter Sandbox */}
          <div className="sticky-filter-sandbox filter-sandbox-card sticky top-0 p-4 rounded-xl border space-y-3 shadow-xl backdrop-blur-xl">
            <div className="flex items-center justify-between">
              <h3 className="filter-sandbox-heading pipelines text-xs font-semibold uppercase tracking-wider flex items-center space-x-1.5">
                <Play className="w-3.5 h-3.5" />
                <span>Recipe Playground</span>
              </h3>
              <span className="text-[10px] theme-text-muted">Test a draft, saved Recipe, or legacy Pipeline</span>
            </div>
            <div className="grid grid-cols-2 gap-4 text-xs font-mono">
              <div>
                <label className="block theme-text-muted mb-1 font-sans">Input Text:</label>
                <textarea
                  value={testText}
                  onChange={(e) => setTestText(e.target.value)}
                  className="w-full h-24 theme-input border rounded-lg p-2.5 focus:outline-none text-xs"
                />
              </div>
              <div>
                <label className="filter-sandbox-output-label block mb-1 font-sans font-semibold">Output Preview:</label>
                <div className="filter-sandbox-output w-full h-24 theme-input border rounded-lg p-2.5 overflow-y-auto whitespace-pre-wrap">
                  {testError || testResult || 'Test a draft or choose a saved Recipe to preview its output.'}
                </div>
              </div>
            </div>
            <TransformationOutputActions output={testError ? '' : testResult} accent="pipelines" />
          </div>

          <section className="space-y-3" aria-labelledby="saved-recipes-heading">
            <div className="flex items-baseline gap-2 px-1">
              <h3 id="saved-recipes-heading" className="text-xs font-semibold theme-text-main">Saved Recipes</h3>
              <span className="text-[10px] theme-text-subtle">Ready to reuse with the same plan</span>
            </div>
            {recipes.length === 0 ? (
              <div className="theme-card-idle rounded-xl border border-dashed px-4 py-5 text-center">
                <Sparkles className="transform-accent pipelines mx-auto mb-2 h-5 w-5" />
                <p className="text-xs font-semibold theme-text-main">No saved Recipes yet</p>
                <p className="mt-1 text-[10px] theme-text-muted">Build and test a draft above, then save it here.</p>
              </div>
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                {recipes.map((recipe) => {
                  const semanticSteps = recipe.plan.steps.filter((step) => step.executor.kind === 'semantic').length;
                  const isRunning = runningRecipeRef === recipe.stableRef;
                  return (
                    <div
                      key={recipe.stableRef}
                      role="button"
                      tabIndex={0}
                      onClick={() => void handleRunRecipe(recipe)}
                      onKeyDown={(event) => {
                        if (event.key === 'Enter' || event.key === ' ') {
                          event.preventDefault();
                          void handleRunRecipe(recipe);
                        }
                      }}
                      className="transform-card pipelines group flex cursor-pointer items-center justify-between rounded-xl border p-3.5 shadow-md transition-[background-color,border-color,box-shadow,transform] theme-card-idle"
                    >
                      <div className="flex min-w-0 items-center gap-3 pr-2">
                        <span className="theme-badge grid h-9 w-9 shrink-0 place-items-center rounded-lg border">
                          {isRunning ? <LoaderCircle className="h-4 w-4 animate-spin" /> : <Sparkles className="transform-accent pipelines h-4 w-4" />}
                        </span>
                        <span className="min-w-0">
                          <span className="block truncate text-xs font-bold theme-text-main">{recipe.name}</span>
                          <span className="mt-1 flex items-center gap-1.5 text-[10px] theme-text-muted">
                            <span>{recipe.plan.steps.length} {recipe.plan.steps.length === 1 ? 'step' : 'steps'}</span>
                            <span>·</span>
                            <span>{semanticSteps > 0 ? `${semanticSteps} connected` : 'Replayable locally'}</span>
                          </span>
                        </span>
                      </div>
                      <button
                        type="button"
                        onClick={(event) => {
                          event.stopPropagation();
                          void handleDeleteRecipe(recipe.stableRef);
                        }}
                        className="theme-icon-button theme-danger-text shrink-0 rounded-md border p-1.5 transition-colors"
                        title="Delete Recipe"
                      >
                        <Trash2 className="h-4 w-4" />
                      </button>
                    </div>
                  );
                })}
              </div>
            )}
          </section>

          {/* Filter Category Filter Pills */}
          <div className="flex items-baseline gap-2 px-1 -mb-3">
            <h3 className="text-xs font-semibold theme-text-main">Legacy Pipelines</h3>
            <span className="text-[10px] theme-text-subtle">Deterministic building blocks remain available</span>
          </div>
          <TransformLibraryToolbar
            accent="pipelines"
            createLabel="New Pipeline"
            onCreate={handleOpenCreateModal}
          >
            {FILTER_CATEGORIES.map((cat) => (
              <button
                key={cat}
                onClick={() => setActiveCategory(cat)}
                className={`transform-category-pill pipelines ui-pill px-3 py-1 text-xs font-semibold whitespace-nowrap transition-colors ${activeCategory === cat ? 'is-active shadow' : ''}`}
              >
                {cat}
              </button>
            ))}
          </TransformLibraryToolbar>

          {/* Active Filters Grid */}
          <div className="space-y-3">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              {pipelines
                .filter((f) => {
                  if (activeCategory === 'All') return true;
                  return f.steps.some((step) => operations.find((operation) => operation.stable_id === step.operationRef)?.category === activeCategory);
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
                      onClick={() => handleTestTransformation(f)}
                      onKeyDown={(event) => {
                        if (event.key === 'Enter' || event.key === ' ') {
                          event.preventDefault();
                          void handleTestTransformation(f);
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
          </div>

          {/* Floating Filter Context Menu */}
          {pipelineContextMenu && (
            <div
              className="filter-card-menu theme-menu fixed w-48 border rounded-xl py-1.5 text-xs animate-in fade-in duration-100 font-sans"
              style={{ top: pipelineContextMenu.y, left: pipelineContextMenu.x }}
              onClick={(e) => e.stopPropagation()}
            >
              <div className="theme-text-muted theme-divider px-3 py-1 text-[10px] uppercase font-bold border-b truncate">
                {pipelineContextMenu.pipeline.name}
              </div>

              <button
                onClick={() => {
                  handleOpenEditModal(pipelineContextMenu.pipeline);
                  setPipelineContextMenu(null);
                }}
                className="theme-menu-item w-full px-3 py-1.5 text-left flex items-center space-x-2 transition-colors"
              >
                <Edit3 className="w-3.5 h-3.5" />
                <span>Edit Pipeline</span>
              </button>

              <button
                onClick={() => {
                  handleDuplicatePipeline(pipelineContextMenu.pipeline);
                  setPipelineContextMenu(null);
                }}
                className="theme-menu-item w-full px-3 py-1.5 text-left flex items-center space-x-2 transition-colors"
              >
                <Copy className="w-3.5 h-3.5" />
                <span>Duplicate Pipeline</span>
              </button>

              <button
                onClick={() => {
                  handleTestTransformation(pipelineContextMenu.pipeline);
                  setPipelineContextMenu(null);
                }}
                className="theme-menu-item w-full px-3 py-1.5 text-left flex items-center space-x-2 transition-colors"
              >
                <Play className="w-3.5 h-3.5" />
                <span>Test in Playground</span>
              </button>

              <button
                onClick={() => {
                  handleExportPipeline(pipelineContextMenu.pipeline);
                  setPipelineContextMenu(null);
                }}
                className="theme-menu-item w-full px-3 py-1.5 text-left flex items-center space-x-2 transition-colors"
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
                className="theme-menu-item theme-danger-text w-full px-3 py-1.5 text-left flex items-center space-x-2 transition-colors"
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
    </div>
  );
};
