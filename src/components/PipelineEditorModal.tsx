import React, { useState, useEffect, useRef } from 'react';
import { Pipeline, PipelineStep, Operation } from '../types';
import { ArrowDown, ArrowUp, Sliders, Plus, Trash2, RotateCcw } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { HotkeyRecorder } from './HotkeyRecorder';
import { handleWindowDragDoubleClick, startWindowDrag } from '../utils/windowDrag';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading, SaveButtonContent } from './AppDialogLayout';
import { MenuSelect, type MenuSelectOption } from './MenuSelect';
import { startPipelinePreview, type CancellableTransformRequest } from '../utils/transformExecution';
import { PlaygroundRunStatus, type PlaygroundRunState } from './PlaygroundRunStatus';
import { TransformationPreviewPanel } from './TransformationPreviewPanel';
import { RegistryPanelHeader } from './RegistryPanelHeader';

export interface PipelineEditorStep {
  id: string;
  operation_ref: string;
  config?: string | null;
  findPattern?: string;
  replacePattern?: string;
  matchMode?: 'regex' | 'literal' | 'wildcard';
  caseSensitive?: boolean;
  tagName?: string;
  shellCommand?: string;
  quoteBefore?: string;
  quoteAfter?: string;
  applyToEachLine?: boolean;
}

interface PipelineEditorModalProps {
  pipeline: Pipeline | null; // null if creating new
  isOpen: boolean;
  onClose: () => void;
  onSaveSuccess: () => void;
}

const EXECUTOR_OPTIONS = [
  { value: 'regex', label: 'Find & Replace (Regex / Text)', category: 'Search' },
];

const OPERATION_CATEGORIES = [
  { key: 'Search', label: 'Search & Replace', registryCategory: null },
  { key: 'Cleaners', label: 'Cleaners & Sanitizers', registryCategory: 'Cleaners & Sanitizers' },
  { key: 'Format', label: 'Smart Formatting', registryCategory: 'Smart Formatting' },
  { key: 'Case', label: 'Case Transformations', registryCategory: 'Case Transformations' },
  { key: 'Extract', label: 'Data Extraction', registryCategory: 'Data Extraction' },
  { key: 'Lines', label: 'Line Operations', registryCategory: 'Line Operations' },
  { key: 'Structure', label: 'Structure & Formatting', registryCategory: 'Structure & Formatting' },
  { key: 'Encoding', label: 'Encodings & Decodings', registryCategory: 'Encodings & Decodings' },
  { key: 'Advanced', label: 'Advanced & Shell Scripts', registryCategory: null },
];

function operationTypeForRef(operationRef: string) {
  return operationRef.startsWith('builtin:') ? operationRef.slice('builtin:'.length) : null;
}

function pipelineStepToEditorStep(step: PipelineStep, index: number): PipelineEditorStep {
  const operationType = operationTypeForRef(step.operationRef);
  let parsedConfig: Record<string, unknown> = {};
  if (operationType === 'regex' && step.configJson) {
    try {
      parsedConfig = JSON.parse(step.configJson) as Record<string, unknown>;
    } catch {
      parsedConfig = {};
    }
  } else if (operationType === 'quote_text' && step.configJson) {
    try {
      parsedConfig = JSON.parse(step.configJson) as Record<string, unknown>;
    } catch {
      parsedConfig = {};
    }
  }
  return {
    id: `step-${index}-${Date.now()}`,
    operation_ref: step.operationRef,
    config: step.configJson,
    findPattern: typeof parsedConfig.pattern === 'string' ? parsedConfig.pattern : '',
    replacePattern: typeof parsedConfig.replacement === 'string' ? parsedConfig.replacement : '',
    matchMode: parsedConfig.matchMode === 'literal' || parsedConfig.matchMode === 'wildcard' ? parsedConfig.matchMode : 'regex',
    caseSensitive: parsedConfig.caseSensitive === true,
    tagName: operationType === 'wrap_tags' ? step.configJson || 'code' : 'code',
    shellCommand: operationType === 'shell_script' ? step.configJson || 'cat' : 'cat',
    quoteBefore: typeof parsedConfig.before === 'string' ? parsedConfig.before : '> ',
    quoteAfter: typeof parsedConfig.after === 'string' ? parsedConfig.after : '',
    applyToEachLine: typeof parsedConfig.applyToEachLine === 'boolean' ? parsedConfig.applyToEachLine : true,
  };
}

function compilePipelineStep(step: PipelineEditorStep) {
  const operationType = operationTypeForRef(step.operation_ref);
  let configJson: string | null = step.config || null;
  if (operationType === 'regex') {
    configJson = JSON.stringify({
      pattern: step.findPattern || '',
      replacement: step.replacePattern || '',
      matchMode: step.matchMode || 'regex',
      caseSensitive: step.caseSensitive || false,
    });
  } else if (operationType === 'wrap_tags') {
    configJson = step.tagName || 'code';
  } else if (operationType === 'shell_script') {
    configJson = step.shellCommand || 'cat';
  } else if (operationType === 'quote_text') {
    configJson = JSON.stringify({
      before: step.quoteBefore ?? '> ',
      after: step.quoteAfter ?? '',
      applyToEachLine: step.applyToEachLine ?? true,
    });
  }
  return {
    operationRef: step.operation_ref,
    configJson,
    failurePolicy: 'stop' as const,
  };
}

const PipelineStepEditor: React.FC<{
  step: PipelineEditorStep;
  idx: number;
  totalSteps: number;
  onRemove: () => void;
  onUpdate: (updates: Partial<PipelineEditorStep>) => void;
  operationsList: Operation[];
  onMoveUp: () => void;
  onMoveDown: () => void;
}> = ({
  step,
  idx,
  totalSteps,
  onRemove,
  onUpdate,
  operationsList,
  onMoveUp,
  onMoveDown,
}) => {
  const operationType = step.operation_ref.startsWith('builtin:')
    ? step.operation_ref.slice('builtin:'.length)
    : null;
  const hasConfig = operationType === 'regex'
    || operationType === 'quote_text'
    || operationType === 'shell_script'
    || operationType === 'wrap_tags';
  const operationOptions: MenuSelectOption[] = OPERATION_CATEGORIES.flatMap((category) => {
    const executors = EXECUTOR_OPTIONS
      .filter((option) => option.category === category.key)
      .map((option) => ({ value: `builtin:${option.value}`, label: option.label, group: category.label }));
    const builtIns = category.registryCategory
      ? operationsList
        .filter((operation) => operation.stable_id.startsWith('builtin:') && operation.category === category.registryCategory)
        .map((operation) => ({ value: operation.stable_id, label: operation.name, group: category.label }))
      : [];
    return [...executors, ...builtIns];
  });
  operationOptions.push(...operationsList
    .filter((operation) => operation.stable_id.startsWith('custom:'))
    .map((operation) => ({ value: operation.stable_id, label: operation.name, group: 'Custom Operations' })));
  return (
    <section className="theme-card-idle border p-2" aria-label={`Transform step ${idx + 1}`}>
      <div className="flex flex-wrap items-center gap-2">
        <span className="theme-text-subtle grid h-5 w-5 shrink-0 place-items-center rounded-full border text-[9px] font-bold">{idx + 1}</span>
        <MenuSelect
          value={step.operation_ref}
          options={operationOptions}
          onChange={(value) => onUpdate({ operation_ref: value })}
          label={`Step ${idx + 1} operation`}
          className="min-w-44 flex-1 font-sans"
          compact
          searchable
          searchPlaceholder="Search Operations…"
        />
        <span className="flex shrink-0 items-center gap-1">
          <button type="button" onClick={onMoveUp} disabled={idx === 0} className="theme-icon-button rounded-md border p-1.5 disabled:opacity-35" aria-label="Move step up" title="Move step up"><ArrowUp className="h-3.5 w-3.5" /></button>
          <button type="button" onClick={onMoveDown} disabled={idx === totalSteps - 1} className="theme-icon-button rounded-md border p-1.5 disabled:opacity-35" aria-label="Move step down" title="Move step down"><ArrowDown className="h-3.5 w-3.5" /></button>
          {totalSteps > 1 && (
            <button
              type="button"
              onClick={onRemove}
              className="theme-icon-button theme-danger-text rounded-md border p-1.5"
              aria-label="Delete step"
              title="Delete step"
            >
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          )}
        </span>
      </div>

      {hasConfig && <div className="theme-divider mt-2 grid grid-cols-1 gap-3 border-t pt-3 text-xs sm:grid-cols-2">
        {/* Step Specific Config Inputs */}
        {operationType === 'regex' && (
          <div className="space-y-2 sm:col-span-2">
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
              <div>
                <label className="block mb-1 theme-text-muted">Find</label>
                <textarea
                  placeholder="Text pattern or Regex pattern"
                  value={step.findPattern || ''}
                  onChange={(e) => onUpdate({ findPattern: e.target.value })}
                  className="w-full h-16 border rounded-lg p-2 font-mono text-xs focus:outline-none theme-input"
                />
              </div>
              <div>
                <label className="block mb-1 theme-text-muted">Replace with</label>
                <textarea
                  placeholder="Replacement string"
                  value={step.replacePattern || ''}
                  onChange={(e) => onUpdate({ replacePattern: e.target.value })}
                  className="w-full h-16 border rounded-lg p-2 font-mono text-xs focus:outline-none theme-input"
                />
              </div>
            </div>
            <div className="flex flex-wrap items-center gap-3 pt-1">
              <div className="flex items-center space-x-1.5 text-xs theme-text-muted">
                <span>Match:</span>
                <MenuSelect
                  value={step.matchMode || 'regex'}
                  onChange={(value) => onUpdate({ matchMode: value as PipelineEditorStep['matchMode'] })}
                  options={[
                    { value: 'literal', label: 'Contains' },
                    { value: 'wildcard', label: 'Wildcard' },
                    { value: 'regex', label: 'Regular Expression' },
                  ]}
                  label="Match mode"
                  className="w-40"
                  compact
                />
              </div>
              <label className="flex items-center space-x-1.5 text-xs cursor-pointer theme-text-muted">
                <input
                  type="checkbox"
                  checked={step.caseSensitive || false}
                  onChange={(e) => onUpdate({ caseSensitive: e.target.checked })}
                  className="theme-checkbox rounded focus:ring-0"
                />
                <span>Case Sensitive</span>
              </label>
            </div>
          </div>
        )}

        {operationType === 'quote_text' && (
          <div className="space-y-2 sm:col-span-2">
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
              <div>
                <label className="block mb-1 theme-text-muted">Before content</label>
                <textarea
                  value={step.quoteBefore ?? '> '}
                  onChange={(e) => onUpdate({ quoteBefore: e.target.value })}
                  className="w-full h-16 border rounded-lg p-2 font-mono text-xs focus:outline-none theme-input"
                />
              </div>
              <div>
                <label className="block mb-1 theme-text-muted">After content</label>
                <textarea
                  value={step.quoteAfter ?? ''}
                  onChange={(e) => onUpdate({ quoteAfter: e.target.value })}
                  className="w-full h-16 border rounded-lg p-2 font-mono text-xs focus:outline-none theme-input"
                />
              </div>
            </div>
            <label className="flex items-center space-x-1.5 text-xs cursor-pointer theme-text-muted">
              <input
                type="checkbox"
                checked={step.applyToEachLine ?? true}
                onChange={(e) => onUpdate({ applyToEachLine: e.target.checked })}
                className="theme-checkbox rounded focus:ring-0"
              />
              <span>Apply to each line</span>
            </label>
          </div>
        )}

        {operationType === 'shell_script' && (
          <div className="sm:col-span-2">
            <label className="block mb-1 theme-text-muted">Shell command (stdin → stdout)</label>
            <input
              type="text"
              placeholder='e.g. tr "a-z" "A-Z"'
              value={step.shellCommand || ''}
              onChange={(e) => onUpdate({ shellCommand: e.target.value })}
              className="w-full border rounded-lg p-2 font-mono text-xs focus:outline-none theme-input"
            />
          </div>
        )}

        {operationType === 'wrap_tags' && (
          <div>
            <label className="block mb-1 theme-text-muted">HTML tag name</label>
            <input
              type="text"
              placeholder="code, b, blockquote"
              value={step.tagName || ''}
              onChange={(e) => onUpdate({ tagName: e.target.value })}
              className="w-full border rounded-lg p-2 font-mono text-xs focus:outline-none theme-input"
            />
          </div>
        )}
      </div>}
    </section>
  );
};

export const PipelineEditorModal: React.FC<PipelineEditorModalProps> = ({
  pipeline,
  isOpen,
  onClose,
  onSaveSuccess,
}) => {
  const [pipelineName, setPipelineName] = useState('');
  const [shortcut, setShortcut] = useState<string | null>(null);
  const [steps, setSteps] = useState<PipelineEditorStep[]>([]);
  const [testInput, setTestInput] = useState('Hello Pasted User! :) https://example.com?utm_source=test');
  const [testOutput, setTestOutput] = useState('');
  const [testRunState, setTestRunState] = useState<PlaygroundRunState>('idle');
  const [testDurationMs, setTestDurationMs] = useState<number>();
  const [saveError, setSaveError] = useState('');
  const [isSaving, setIsSaving] = useState(false);
  const testRequestIdRef = useRef(0);
  const activeTestExecutionRef = useRef<CancellableTransformRequest<string> | null>(null);
  const [operationsList, setOperationsList] = useState<Operation[]>([]);
  const initialSnapshotRef = useRef('');

  const refreshOps = () => {
    invoke<Operation[]>('get_operations')
      .then((ops) => setOperationsList(ops))
      .catch((error) => setSaveError(error instanceof Error ? error.message : String(error)));
  };

  useEffect(() => {
    if (!isOpen) return;

    // Fetch operations from SQLite database
    refreshOps();

    if (pipeline) {
      const nextSteps = pipeline.steps.map(pipelineStepToEditorStep);
      setPipelineName(pipeline.name);
      setShortcut(pipeline.shortcut || null);
      setSteps(nextSteps);
      initialSnapshotRef.current = JSON.stringify({
        pipelineName: pipeline.name,
        shortcut: pipeline.shortcut || null,
        steps: nextSteps,
      });
    } else {
      const nextSteps = [createDefaultStep('builtin:smart_punctuation', null)];
      setPipelineName('');
      setShortcut(null);
      setSteps(nextSteps);
      initialSnapshotRef.current = JSON.stringify({ pipelineName: '', shortcut: null, steps: nextSteps });
    }
    setSaveError('');
  }, [isOpen, pipeline]);

  const handleReset = () => {
    if (pipeline) {
      setPipelineName(pipeline.name);
      setShortcut(pipeline.shortcut || null);
      setSteps(pipeline.steps.map(pipelineStepToEditorStep));
    } else {
      setPipelineName('');
      setShortcut(null);
      setSteps([createDefaultStep('builtin:smart_punctuation', null)]);
      setTestInput('Hello Pasted User! :) https://example.com?utm_source=test');
    }
  };

  // Debounce automatic previews so connected operations do not start a provider
  // process for every keystroke. Superseded previews are cancelled below.
  useEffect(() => {
    if (!isOpen) return;
    const timer = window.setTimeout(() => void runLiveTest(), 350);
    return () => {
      window.clearTimeout(timer);
      void activeTestExecutionRef.current?.cancel();
      activeTestExecutionRef.current = null;
      testRequestIdRef.current += 1;
    };
  }, [steps, testInput, isOpen]);

  const createDefaultStep = (operationRef: string, config: string | null): PipelineEditorStep => {
    return {
      id: `step-${Date.now()}-${Math.random()}`,
      operation_ref: operationRef,
      config,
      findPattern: '',
      replacePattern: '',
      matchMode: 'regex',
      caseSensitive: false,
      tagName: 'code',
      shellCommand: 'tr "a-z" "A-Z"',
      quoteBefore: '> ',
      quoteAfter: '',
      applyToEachLine: true,
    };
  };

  const handleAddStep = () => {
    setSteps((prev) => [...prev, createDefaultStep('builtin:smart_punctuation', null)]);
  };

  const handleRemoveStep = (id: string) => {
    if (steps.length === 1) return; // Keep at least one step
    setSteps((prev) => prev.filter((s) => s.id !== id));
  };

  const handleUpdateStep = (id: string, updates: Partial<PipelineEditorStep>) => {
    setSteps((prev) =>
      prev.map((s) => (s.id === id ? { ...s, ...updates } : s))
    );
  };

  const handleMoveStep = (index: number, offset: -1 | 1) => {
    setSteps((current) => {
      const destination = index + offset;
      if (destination < 0 || destination >= current.length) return current;
      const next = [...current];
      [next[index], next[destination]] = [next[destination], next[index]];
      return next;
    });
  };

  const runLiveTest = async () => {
    if (!testInput) {
      setTestOutput('');
      return;
    }
    const requestId = ++testRequestIdRef.current;
    const startedAt = performance.now();
    setTestRunState('running');
    setTestDurationMs(undefined);
    try {
      const execution = startPipelinePreview(testInput, steps.map(compilePipelineStep));
      activeTestExecutionRef.current = execution;
      const current = await execution.promise;
      if (requestId !== testRequestIdRef.current) return;
      setTestOutput(current);
      setTestRunState('success');
      setTestDurationMs(performance.now() - startedAt);
    } catch (e) {
      if (requestId !== testRequestIdRef.current) return;
      setTestOutput(`Error: ${e}`);
      setTestRunState(String(e).includes('execution_cancelled') ? 'cancelled' : 'error');
    } finally {
      if (requestId === testRequestIdRef.current) activeTestExecutionRef.current = null;
    }
  };

  const cancelLiveTest = () => {
    void activeTestExecutionRef.current?.cancel();
    activeTestExecutionRef.current = null;
    testRequestIdRef.current += 1;
    setTestRunState('cancelled');
  };

  const handleSavePipeline = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!pipelineName.trim()) return;

    setSaveError('');
    setIsSaving(true);
    try {
      const compiledSteps = steps.map(compilePipelineStep);

      if (pipeline) {
        await invoke('update_pipeline', {
          pipelineRef: pipeline.stableRef,
          name: pipelineName.trim(),
          steps: compiledSteps,
          shortcut,
        });
      } else {
        await invoke('create_pipeline', {
          name: pipelineName.trim(),
          steps: compiledSteps,
          shortcut,
        });
      }

      onSaveSuccess();
      onClose();
    } catch (e) {
      setSaveError(e instanceof Error ? e.message : String(e));
    } finally {
      setIsSaving(false);
    }
  };

  if (!isOpen) return null;
  const isDirty = JSON.stringify({ pipelineName, shortcut, steps }) !== initialSnapshotRef.current;

  return (
    <AppDialog
      isOpen={isOpen}
      onClose={onClose}
      labelledBy="pipeline-editor-title"
      isDirty={isDirty}
      overlayClassName="p-6"
      panelClassName="theme-panel flex max-h-[90vh] w-full max-w-4xl flex-col overflow-hidden border"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose} onMouseDown={startWindowDrag} onDoubleClick={handleWindowDragDoubleClick}>
          <AppDialogHeading id="pipeline-editor-title" title={pipeline ? 'Edit Transform' : 'Build Transform Manually'} description="Chain reusable Operations into a local, replayable Transform." icon={<Sliders />} tone="info" />
        </AppDialogHeader>

        <AppDialogBody className="space-y-6 relative">
          {/* Filter Metadata */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 items-end">
            <div className="md:col-span-2">
              <label className="mb-1 block text-xs font-semibold theme-text-muted">
                Name
              </label>
              <input
                type="text"
                placeholder="e.g. Sanitize HTML & Convert Smileys"
                value={pipelineName}
                onChange={(e) => setPipelineName(e.target.value)}
                className="theme-input ui-field-radius w-full border px-3 py-2 text-xs font-medium focus:outline-none"
                autoFocus
              />
            </div>
            <div>
              <label className="mb-1 block text-xs font-semibold theme-text-muted">
                Shortcut
              </label>
              <HotkeyRecorder
                value={shortcut}
                placeholder="+ Set Hotkey"
                onChange={(newShortcut) => setShortcut(newShortcut)}
              />
            </div>
          </div>

          <TransformationPreviewPanel
            description="Updates automatically as steps change"
            status={<PlaygroundRunStatus
              state={testRunState}
              label="preview"
              durationMs={testDurationMs}
              onRetry={() => void runLiveTest()}
              onStop={cancelLiveTest}
            />}
            input={<textarea
                  value={testInput}
                  onChange={(e) => setTestInput(e.target.value)}
                  className="theme-input ui-field-radius w-full h-24 border p-2.5 focus:outline-none"
                />}
            output={<div className="theme-input ui-field-radius overlay-scroll-region w-full h-24 border p-2.5 overflow-y-auto whitespace-pre-wrap font-mono">
                  {testOutput || 'Transformed output will appear here...'}
                </div>}
          />

          {/* Sequential Step Builder */}
          <section className="theme-surface overflow-hidden rounded-xl border">
            <RegistryPanelHeader
              title={<>Steps <span className="theme-text-subtle font-normal">({steps.length})</span></>}
              actions={<AppDialogButton
                onClick={handleAddStep}
                className="h-7 min-h-7 px-2.5"
              >
                <Plus className="h-3 w-3" />
                <span>Add Step</span>
              </AppDialogButton>}
            />

            <div className="theme-subtle-surface space-y-1 p-1.5">
                {steps.map((step, idx) => (
                  <PipelineStepEditor
                    key={step.id}
                    step={step}
                    idx={idx}
                    totalSteps={steps.length}
                    onRemove={() => handleRemoveStep(step.id)}
                    onUpdate={(updates) => handleUpdateStep(step.id, updates)}
                    operationsList={operationsList}
                    onMoveUp={() => handleMoveStep(idx, -1)}
                    onMoveDown={() => handleMoveStep(idx, 1)}
                  />
                ))}
            </div>
          </section>
          {saveError && <div role="alert" className="theme-status-danger rounded-xl border px-3 py-2 text-xs">{saveError}</div>}
        </AppDialogBody>

        <AppDialogFooter align="between">
          <AppDialogButton
            onClick={handleReset}
            title="Reset Transform"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            <span>Reset</span>
          </AppDialogButton>

          <div className="flex items-center space-x-3">
            <AppDialogButton onClick={requestClose}>Cancel</AppDialogButton>
            <AppDialogButton variant="primary" onClick={handleSavePipeline} disabled={isSaving}>
              <SaveButtonContent isSaving={isSaving} />
            </AppDialogButton>
          </div>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
};
