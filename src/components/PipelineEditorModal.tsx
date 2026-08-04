import React, { useState, useEffect, useRef } from 'react';
import { Pipeline, PipelineStep, Operation } from '../types';
import { Sliders, Plus, Trash2, Play, ArrowDown, ArrowUp, GripVertical, Wrench, RotateCcw } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { useStableVerticalReorder } from '../hooks/useStableVerticalReorder';
import { HotkeyRecorder } from './HotkeyRecorder';
import { OperationEditorModal } from './OperationEditorModal';
import { startWindowDrag } from '../utils/windowDrag';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';

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
  { value: 'shell_script', label: 'Shell Script Command (sh -c)', category: 'Advanced' },
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
    failurePolicy: 'stop',
  };
}

const StepReorderCard: React.FC<{
  step: PipelineEditorStep;
  idx: number;
  totalSteps: number;
  onMoveUp: () => void;
  onMoveDown: () => void;
  onInsertBelow: () => void;
  onRemove: () => void;
  onUpdate: (updates: Partial<PipelineEditorStep>) => void;
  operationsList: Operation[];
  setIsOpModalOpen: (open: boolean) => void;
  isDragging: boolean;
  reorderOffsetY: number;
  onReorderPointerDown: (event: React.PointerEvent) => void;
}> = ({
  step,
  idx,
  totalSteps,
  onMoveUp,
  onMoveDown,
  onInsertBelow,
  onRemove,
  onUpdate,
  operationsList,
  setIsOpModalOpen,
  isDragging,
  reorderOffsetY,
  onReorderPointerDown,
}) => {
  const operationType = step.operation_ref.startsWith('builtin:')
    ? step.operation_ref.slice('builtin:'.length)
    : null;
  return (
      <div
        data-stable-reorder-id={step.id}
        style={reorderOffsetY !== 0 || isDragging ? {
          transform: `translateY(${reorderOffsetY}px)`,
          zIndex: isDragging ? 'var(--layer-drag)' : 1,
        } : undefined}
        className={`filter-step-card p-3.5 rounded-xl border space-y-3 relative group select-none transition-[background-color,border-color,box-shadow,opacity,transform] duration-100 ease-out ${
          isDragging ? 'is-dragging' : ''
        }`}
      >
        {/* Step Header */}
        <div className="theme-divider flex items-center justify-between border-b pb-2">
          {/* Left: Step Number Badge, Drag Handle, Arrow Buttons */}
          <div className="flex items-center space-x-1.5">
            <span className="theme-status-info w-5 h-5 rounded-full text-[11px] font-bold flex items-center justify-center font-mono border mr-0.5">
              {idx + 1}
            </span>
            <button
              type="button"
              onPointerDown={onReorderPointerDown}
              className="step-drag-handle theme-icon-button titlebar-no-drag p-1.5 rounded touch-none select-none shrink-0 border outline-none"
              style={{ touchAction: 'none' }}
              title="Reorder Step"
            >
              <GripVertical className="w-4 h-4 pointer-events-none" />
            </button>
            <button
              type="button"
              disabled={idx === 0}
              onClick={onMoveUp}
              className="theme-icon-button p-1 border disabled:opacity-20 rounded transition-colors"
              title="Move Up"
            >
              <ArrowUp className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              disabled={idx === totalSteps - 1}
              onClick={onMoveDown}
              className="theme-icon-button p-1 border disabled:opacity-20 rounded transition-colors"
              title="Move Down"
            >
              <ArrowDown className="w-3.5 h-3.5" />
            </button>
          </div>

        {/* Right: Insert Below & Remove Step Actions */}
        <div className="flex items-center space-x-1.5">
          <button
            type="button"
            onClick={onInsertBelow}
            className="theme-icon-button p-1 border rounded transition-colors"
            title="Insert Below"
          >
            <Plus className="w-3.5 h-3.5" />
          </button>

          {totalSteps > 1 && (
            <button
              type="button"
              onClick={onRemove}
              className="theme-icon-button theme-danger-text p-1 border rounded transition-colors"
              title="Remove Step"
            >
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          )}
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs">
        <div>
          <div className="flex items-center justify-between mb-1">
            <label className="block theme-text-muted">Step Operation:</label>
            <button
              type="button"
              onClick={() => setIsOpModalOpen(true)}
              className="theme-status-info-text text-[10px] flex items-center space-x-0.5 hover:underline"
              title="New Operation"
            >
              <Wrench className="w-2.5 h-2.5" />
              <span>+ New Operation</span>
            </button>
          </div>
          <select
            value={step.operation_ref}
            onChange={(e) => onUpdate({ operation_ref: e.target.value })}
            className="w-full border rounded-lg p-2 focus:outline-none theme-input font-sans"
          >
            {OPERATION_CATEGORIES.map((cat) => {
              const executors = EXECUTOR_OPTIONS.filter((option) => option.category === cat.key);
              const builtIns = cat.registryCategory
                ? operationsList.filter(
                    (operation) => operation.stable_id.startsWith('builtin:') && operation.category === cat.registryCategory
                  )
                : [];
              if (executors.length === 0 && builtIns.length === 0) return null;
              return (
                <optgroup key={cat.key} label={cat.label}>
                  {executors.map((option) => (
                    <option key={option.value} value={`builtin:${option.value}`}>
                      {option.label}
                    </option>
                  ))}
                  {builtIns.map((operation) => (
                    <option key={operation.stable_id} value={operation.stable_id}>
                      {operation.name}
                    </option>
                  ))}
                </optgroup>
              );
            })}

            {operationsList.some((operation) => operation.stable_id.startsWith('custom:')) && (
              <optgroup label="Custom Operations">
                {operationsList
                  .filter((operation) => operation.stable_id.startsWith('custom:'))
                  .map((op) => (
                    <option key={`custom-${op.id}`} value={op.stable_id}>
                      {op.name}
                    </option>
                  ))}
              </optgroup>
            )}
          </select>
        </div>

        {/* Step Specific Config Inputs */}
        {operationType === 'regex' && (
          <div className="space-y-2 col-span-2">
            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block mb-1 theme-text-muted">Find:</label>
                <textarea
                  placeholder="Text pattern or Regex pattern"
                  value={step.findPattern || ''}
                  onChange={(e) => onUpdate({ findPattern: e.target.value })}
                  className="w-full h-16 border rounded-lg p-2 font-mono text-xs focus:outline-none theme-input"
                />
              </div>
              <div>
                <label className="block mb-1 theme-text-muted">Replace with:</label>
                <textarea
                  placeholder="Replacement string"
                  value={step.replacePattern || ''}
                  onChange={(e) => onUpdate({ replacePattern: e.target.value })}
                  className="w-full h-16 border rounded-lg p-2 font-mono text-xs focus:outline-none theme-input"
                />
              </div>
            </div>
            <div className="flex items-center space-x-4 pt-1">
              <label className="flex items-center space-x-1.5 text-xs theme-text-muted">
                <span>Match:</span>
                <select
                  value={step.matchMode || 'regex'}
                  onChange={(e) => onUpdate({ matchMode: e.target.value as PipelineEditorStep['matchMode'] })}
                  className="border rounded-lg px-2 py-1 focus:outline-none theme-input"
                >
                  <option value="literal">Contains</option>
                  <option value="wildcard">Wildcard</option>
                  <option value="regex">Regular Expression</option>
                </select>
              </label>
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
          <div className="space-y-2 col-span-2">
            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className="block mb-1 theme-text-muted">Before content:</label>
                <textarea
                  value={step.quoteBefore ?? '> '}
                  onChange={(e) => onUpdate({ quoteBefore: e.target.value })}
                  className="w-full h-16 border rounded-lg p-2 font-mono text-xs focus:outline-none theme-input"
                />
              </div>
              <div>
                <label className="block mb-1 theme-text-muted">After content:</label>
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
          <div className="col-span-2">
            <label className="block mb-1 theme-text-muted">Shell Script Command (stdin -&gt; stdout):</label>
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
            <label className="block mb-1 theme-text-muted">HTML Tag Name:</label>
            <input
              type="text"
              placeholder="code, b, blockquote"
              value={step.tagName || ''}
              onChange={(e) => onUpdate({ tagName: e.target.value })}
              className="w-full border rounded-lg p-2 font-mono text-xs focus:outline-none theme-input"
            />
          </div>
        )}
      </div>
    </div>
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
  const [operationsList, setOperationsList] = useState<Operation[]>([]);
  const [isOpModalOpen, setIsOpModalOpen] = useState(false);
  const stepListRef = useRef<HTMLDivElement>(null);
  const initialSnapshotRef = useRef('');
  const {
    activeId: activeStepId,
    offsets: stepReorderOffsets,
    isSettling: isStepReorderSettling,
    startPointerReorder: startStepPointerReorder,
  } = useStableVerticalReorder({
    itemIds: steps.map((step) => step.id),
    containerRef: stepListRef,
    onCommit: (orderedIds) => {
      setSteps((current) => {
        const byId = new Map(current.map((step) => [step.id, step]));
        return orderedIds.map((id) => byId.get(id)).filter((step): step is PipelineEditorStep => Boolean(step));
      });
    },
  });

  const refreshOps = () => {
    invoke<Operation[]>('get_operations')
      .then((ops) => setOperationsList(ops))
      .catch(console.error);
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

  // Run live test execution when steps or testInput change
  useEffect(() => {
    if (!isOpen) return;
    runLiveTest();
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

  const handleMoveStepUp = (index: number) => {
    if (index <= 0) return;
    setSteps((prev) => {
      const copy = [...prev];
      const temp = copy[index - 1];
      copy[index - 1] = copy[index];
      copy[index] = temp;
      return copy;
    });
  };

  const handleMoveStepDown = (index: number) => {
    if (index >= steps.length - 1) return;
    setSteps((prev) => {
      const copy = [...prev];
      const temp = copy[index + 1];
      copy[index + 1] = copy[index];
      copy[index] = temp;
      return copy;
    });
  };

  const handleInsertStepAt = (index: number) => {
    setSteps((prev) => {
      const copy = [...prev];
      copy.splice(index, 0, createDefaultStep('builtin:trim', null));
      return copy;
    });
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

  const runLiveTest = async () => {
    if (!testInput) {
      setTestOutput('');
      return;
    }
    try {
      let current = testInput;
      for (const step of steps) {
        const compiled = compilePipelineStep(step);
        const operationType = operationTypeForRef(step.operation_ref);
        if (operationType) {
          current = await invoke<string>('transform_text', {
            input: current,
            filterType: operationType,
            config: compiled.configJson,
          });
        } else {
          const result = await invoke<{ output: string }>('execute_transformation', {
            request: {
              input: current,
              target: { kind: 'operation', operationRef: step.operation_ref },
              sourceClipId: null,
              trigger: 'manual',
              destination: 'preview',
            },
          });
          current = result.output;
        }
      }
      setTestOutput(current);
    } catch (e) {
      setTestOutput(`Error: ${e}`);
    }
  };

  const handleSavePipeline = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!pipelineName.trim()) return;

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
      console.error(e);
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
      panelClassName="filter-editor-card w-full max-w-4xl max-h-[90vh] border rounded-2xl flex flex-col overflow-hidden"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose} onMouseDown={startWindowDrag}>
          <AppDialogHeading id="pipeline-editor-title" title={pipeline ? 'Edit Pipeline' : 'New Pipeline'} description="Chain reusable Operations into a transformation that runs as one step." icon={<Sliders />} tone="info" />
        </AppDialogHeader>

        <AppDialogBody className="space-y-6 relative">
          {/* Filter Metadata */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 items-end">
            <div className="md:col-span-2">
              <label className="block text-xs font-semibold uppercase tracking-wider mb-2 theme-text-muted">
                Pipeline Name:
              </label>
              <input
                type="text"
                placeholder="e.g. Sanitize HTML & Convert Smileys"
                value={pipelineName}
                onChange={(e) => setPipelineName(e.target.value)}
                className="w-full border rounded-xl px-4 py-2.5 text-sm focus:outline-none font-medium theme-input"
                autoFocus
              />
            </div>
            <div>
              <label className="block text-xs font-semibold uppercase tracking-wider mb-2 theme-text-muted">
                Global Hotkey Shortcut:
              </label>
              <div className="border rounded-xl p-2 flex items-center justify-between theme-input">
                <span className="text-xs theme-text-muted">Shortcut:</span>
                <HotkeyRecorder
                  value={shortcut}
                  placeholder="+ Set Hotkey"
                  onChange={(newShortcut) => setShortcut(newShortcut)}
                />
              </div>
            </div>
          </div>

          {/* Sticky Interactive Split-Pane Sandbox Tester */}
          <div className="filter-sandbox-card sticky-filter-sandbox sticky top-0 p-4 rounded-2xl border space-y-3 shadow-xl backdrop-blur-xl">
            <div className="flex items-center justify-between">
              <span className="theme-status-info-text text-xs font-semibold uppercase tracking-wider flex items-center space-x-1.5">
                <Play className="w-3.5 h-3.5" />
                <span>Sticky Live Sandbox Tester</span>
              </span>
              <span className="text-[10px] theme-text-muted">Live preview updates automatically on step edit</span>
            </div>

            <div className="grid grid-cols-2 gap-4 text-xs font-mono">
              <div>
                <label className="block mb-1 font-sans theme-text-muted">Input Text:</label>
                <textarea
                  value={testInput}
                  onChange={(e) => setTestInput(e.target.value)}
                  className="w-full h-24 border rounded-xl p-2.5 focus:outline-none theme-input"
                />
              </div>
              <div>
                <label className="filter-sandbox-output-label block mb-1 font-sans font-semibold">Live Output Preview:</label>
                <div className="filter-sandbox-output w-full h-24 border rounded-xl p-2.5 overflow-y-auto whitespace-pre-wrap theme-input font-mono">
                  {testOutput || 'Transformed output will appear here...'}
                </div>
              </div>
            </div>
          </div>

          {/* Sequential Step Builder */}
          <div className="space-y-3 pt-2">
            <div className="flex items-center justify-between">
              <label className="text-xs font-semibold uppercase tracking-wider theme-text-muted">
                Pipeline Execution Steps ({steps.length})
              </label>
            </div>

            {/* Dark Wrapper Container */}
            <div className="filter-step-list p-3 rounded-2xl border space-y-2.5 shadow-inner">
              <div
                ref={stepListRef}
                className={`space-y-2.5 ${isStepReorderSettling ? 'is-settling-stable-reorder' : ''}`}
              >
                {steps.map((step, idx) => (
                  <StepReorderCard
                    key={step.id}
                    step={step}
                    idx={idx}
                    totalSteps={steps.length}
                    onMoveUp={() => handleMoveStepUp(idx)}
                    onMoveDown={() => handleMoveStepDown(idx)}
                    onInsertBelow={() => handleInsertStepAt(idx + 1)}
                    onRemove={() => handleRemoveStep(step.id)}
                    onUpdate={(updates) => handleUpdateStep(step.id, updates)}
                    operationsList={operationsList}
                    setIsOpModalOpen={setIsOpModalOpen}
                    isDragging={activeStepId === step.id}
                    reorderOffsetY={stepReorderOffsets[step.id] ?? 0}
                    onReorderPointerDown={(event) => startStepPointerReorder(step.id, event)}
                  />
                ))}
              </div>

              {/* Bottom Add Step Button inside dark wrapper */}
              <div className="pt-1 flex justify-center">
                <button
                  type="button"
                  onClick={handleAddStep}
                  className="theme-primary-button flex items-center space-x-1.5 px-4 py-2 rounded-xl border text-xs font-semibold shadow-lg active:scale-95 transition-[background-color,transform]"
                >
                  <Plus className="w-4 h-4" />
                  <span>Add Step</span>
                </button>
              </div>
            </div>
          </div>
        </AppDialogBody>

        <AppDialogFooter align="between">
          <AppDialogButton
            onClick={handleReset}
            title="Reset Pipeline"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            <span>Reset</span>
          </AppDialogButton>

          <div className="flex items-center space-x-3">
            <AppDialogButton onClick={requestClose}>Cancel</AppDialogButton>
            <AppDialogButton variant="primary" onClick={handleSavePipeline}>Save Pipeline</AppDialogButton>
          </div>
        </AppDialogFooter>
      {/* Embedded Operation Editor Modal */}
      <OperationEditorModal
        operation={null}
        isOpen={isOpModalOpen}
        onClose={() => setIsOpModalOpen(false)}
        onSaveSuccess={refreshOps}
      />
      </>}
    </AppDialog>
  );
};
