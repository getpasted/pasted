import React, { useEffect, useMemo, useRef, useState } from 'react';
import { Operation } from '../types';
import {
  Code2,
  Edit3,
  LockKeyhole,
  Play,
  Sparkles,
  Trash2,
  Wrench,
} from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { OperationEditorModal, CATEGORIES } from './OperationEditorModal';
import { startWindowDrag } from '../utils/windowDrag';
import { TransformLibraryToolbar } from './TransformLibraryToolbar';
import { TransformationOutputActions } from './TransformationOutputActions';

interface OperationsManagerProps {
  isEmbedded?: boolean;
  onOpenCreateModal?: () => void;
}

function isBuiltInOperation(operation: Operation) {
  return operation.stable_id.startsWith('builtin:');
}

export const OperationsManager: React.FC<OperationsManagerProps> = ({
  isEmbedded = false,
  onOpenCreateModal,
}) => {
  const [operations, setOperations] = useState<Operation[]>([]);
  const [activeCategory, setActiveCategory] = useState('All');
  const [selectedOperationId, setSelectedOperationId] = useState<string | null>(null);
  const [selectedOperationForEdit, setSelectedOperationForEdit] = useState<Operation | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [testText, setTestText] = useState('Hello Pasted Operation Library! :) https://example.com?utm_source=test');
  const [testResult, setTestResult] = useState('');
  const [isRunning, setIsRunning] = useState(false);
  const [runError, setRunError] = useState('');
  const operationRequestId = useRef(0);

  const fetchOperations = async () => {
    try {
      const nextOperations = await invoke<Operation[]>('get_operations');
      setOperations(nextOperations);
      setSelectedOperationId((currentId) => {
        if (currentId && nextOperations.some((operation) => operation.stable_id === currentId)) {
          return currentId;
        }

        return (
          nextOperations.find((operation) => !isBuiltInOperation(operation))?.stable_id
          ?? nextOperations[0]?.stable_id
          ?? null
        );
      });
    } catch (error) {
      console.error(error);
    }
  };

  useEffect(() => {
    fetchOperations();
  }, []);

  const handleOpenCreate = () => {
    setSelectedOperationForEdit(null);
    setIsModalOpen(true);
  };

  const handleOpenEdit = (operation: Operation) => {
    setSelectedOperationForEdit(operation);
    setIsModalOpen(true);
  };

  const handleDelete = async (operation: Operation) => {
    try {
      await invoke('delete_operation', { id: operation.id });
      await fetchOperations();
    } catch (error) {
      console.error(error);
    }
  };

  const handleTestOperation = async (operation: Operation, showProgress = false) => {
    const requestId = ++operationRequestId.current;
    setSelectedOperationId(operation.stable_id);
    setIsRunning(showProgress || operation.op_type === 'ai');
    setRunError('');

    try {
      const response = await invoke<{ output: string }>('execute_transformation', {
        request: {
          input: testText,
          target: { kind: 'operation', operationRef: operation.stable_id },
          sourceClipId: null,
          trigger: 'manual',
        },
      });
      if (requestId !== operationRequestId.current) return;
      setTestResult(response.output);
    } catch (error) {
      if (requestId !== operationRequestId.current) return;
      console.error(error);
      setRunError(error instanceof Error ? error.message : String(error));
    } finally {
      if (requestId === operationRequestId.current) setIsRunning(false);
    }
  };

  const dynamicCategories = useMemo(() => (
    Array.from(new Set([...CATEGORIES, ...operations.map((operation) => operation.category).filter(Boolean)]))
  ), [operations]);

  const filteredOperations = activeCategory === 'All'
    ? operations
    : operations.filter((operation) => operation.category === activeCategory);
  const builtInOperations = filteredOperations.filter(isBuiltInOperation);
  const customOperations = filteredOperations.filter((operation) => !isBuiltInOperation(operation));
  const selectedOperation = operations.find((operation) => operation.stable_id === selectedOperationId) ?? null;

  const renderOperationRow = (operation: Operation) => {
    const builtIn = isBuiltInOperation(operation);
    const selected = operation.stable_id === selectedOperationId;

    return (
      <div
        key={operation.stable_id}
        onClick={() => void handleTestOperation(operation)}
        className={`operation-library-row group flex min-w-0 cursor-pointer items-center gap-2 rounded-xl border p-1.5 transition-[background-color,border-color,box-shadow] ${selected ? 'is-selected' : ''}`}
      >
        <button
          type="button"
          className="operation-library-select flex min-w-0 flex-1 items-center gap-2.5 rounded-lg p-1.5 text-left"
          aria-pressed={selected}
          title={`Run ${operation.name}`}
        >
          <span className="theme-badge grid h-8 w-8 shrink-0 place-items-center rounded-lg border">
            <Code2 className="transform-accent operations h-4 w-4" />
          </span>
          <span className="min-w-0 flex-1">
            <span className="block truncate text-xs font-semibold theme-text-main">{operation.name}</span>
            <span className="mt-0.5 flex min-w-0 items-center gap-1.5">
              <span className="transform-tag operations max-w-full truncate rounded border px-1.5 py-0.5 font-mono text-[10px]">
                {operation.category}
              </span>
            </span>
          </span>
        </button>

        {builtIn ? (
          <LockKeyhole className="mr-2 h-3.5 w-3.5 shrink-0 theme-text-subtle" aria-label="Built-in operation" />
        ) : (
          <span className="flex shrink-0 items-center gap-1 pr-0.5">
            <button
              type="button"
              onClick={(event) => {
                event.stopPropagation();
                handleOpenEdit(operation);
              }}
              className="theme-icon-button rounded-md border p-1.5 transition-colors"
              title="Edit custom operation"
            >
              <Edit3 className="h-3.5 w-3.5" />
            </button>
            <button
              type="button"
              onClick={(event) => {
                event.stopPropagation();
                void handleDelete(operation);
              }}
              className="theme-icon-button theme-danger-text rounded-md border p-1.5 transition-colors"
              title="Delete custom operation"
            >
              <Trash2 className="h-3.5 w-3.5" />
            </button>
          </span>
        )}
      </div>
    );
  };

  const content = (
    <div className="space-y-4">
      {!isEmbedded && (
        <div onMouseDown={startWindowDrag} className="theme-divider flex items-center justify-between border-b pb-4">
          <div>
            <h2 className="theme-title flex items-center space-x-2 text-lg font-bold">
              <Wrench className="transform-accent operations h-5 w-5 opacity-70" />
              <span>Operations</span>
            </h2>
            <p className="mt-1 text-xs theme-text-muted">Reusable transformations for clips, Pipelines, and Automations.</p>
          </div>
        </div>
      )}

      <TransformLibraryToolbar
        accent="operations"
        createLabel="New Operation"
        onCreate={onOpenCreateModal || handleOpenCreate}
      >
        <button
          type="button"
          onClick={() => setActiveCategory('All')}
          className={`transform-category-pill operations ui-pill whitespace-nowrap px-3 py-1.5 text-xs font-semibold transition-colors ${activeCategory === 'All' ? 'is-active shadow' : ''}`}
        >
          All ({operations.length})
        </button>
        {dynamicCategories.map((category) => {
          const count = operations.filter((operation) => operation.category === category).length;
          if (count === 0) return null;

          return (
            <button
              type="button"
              key={category}
              onClick={() => setActiveCategory(category)}
              className={`transform-category-pill operations ui-pill whitespace-nowrap px-3 py-1.5 text-xs font-semibold transition-colors ${activeCategory === category ? 'is-active shadow' : ''}`}
            >
              {category} ({count})
            </button>
          );
        })}
      </TransformLibraryToolbar>

      <div className="operations-workspace-grid">
        <div className="min-w-0 space-y-5">
          <section className="space-y-2" aria-labelledby="your-operations-heading">
            <div className="flex items-baseline gap-2 px-1">
              <h3 id="your-operations-heading" className="text-xs font-semibold theme-text-main">Your Operations</h3>
              <span className="text-[10px] theme-text-subtle">Editable, local, and connected</span>
            </div>

            {customOperations.length > 0 ? (
              <div className="space-y-1.5">{customOperations.map(renderOperationRow)}</div>
            ) : (
              <button
                type="button"
                onClick={onOpenCreateModal || handleOpenCreate}
                className="operation-library-empty flex w-full items-center gap-3 rounded-xl border border-dashed p-3 text-left transition-colors"
              >
                <span className="theme-badge grid h-8 w-8 place-items-center rounded-lg border">
                  <Sparkles className="transform-accent operations h-4 w-4" />
                </span>
                <span>
                  <span className="block text-xs font-semibold theme-text-main">Create your first Operation</span>
                  <span className="text-[10px] theme-text-muted">Start with a safe local regex replacement.</span>
                </span>
              </button>
            )}
          </section>

          <section className="space-y-2" aria-labelledby="built-in-operations-heading">
            <div className="flex items-baseline gap-2 px-1">
              <h3 id="built-in-operations-heading" className="text-xs font-semibold theme-text-main">Built-in Library</h3>
              <span className="text-[10px] theme-text-subtle">Maintained by Pasted · always available</span>
            </div>
            {builtInOperations.length > 0 ? (
              <div className="operations-built-in-grid">{builtInOperations.map(renderOperationRow)}</div>
            ) : (
              <p className="px-1 py-3 text-xs theme-text-muted">No built-in Operations match this category.</p>
            )}
          </section>

        </div>

        <aside className="operation-inspector min-w-0 self-start rounded-2xl border p-4 shadow-lg">
          {selectedOperation ? (
            <div className="space-y-4">
              <div className="flex min-w-0 items-start justify-between gap-3">
                <div className="min-w-0">
                  <div className="mb-1 flex items-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider transform-accent operations">
                    <Play className="h-3.5 w-3.5" />
                    <span>Playground</span>
                  </div>
                  <h3 className="truncate text-sm font-bold theme-text-main">{selectedOperation.name}</h3>
                  <p className="mt-1 text-[10px] theme-text-muted">
                    {isBuiltInOperation(selectedOperation) ? 'Built-in' : selectedOperation.op_type} · {selectedOperation.category}
                  </p>
                </div>
                {!isBuiltInOperation(selectedOperation) && (
                  <button
                    type="button"
                    onClick={() => handleOpenEdit(selectedOperation)}
                    className="theme-icon-button shrink-0 rounded-lg border p-2 transition-colors"
                    title="Edit custom operation"
                  >
                    <Edit3 className="h-4 w-4" />
                  </button>
                )}
              </div>

              <div>
                <label htmlFor="operation-playground-input" className="mb-1.5 block text-[10px] font-semibold theme-text-muted">Input</label>
                <textarea
                  id="operation-playground-input"
                  value={testText}
                  onChange={(event) => setTestText(event.target.value)}
                  className="theme-input h-28 w-full resize-y rounded-xl border p-3 font-mono text-xs focus:outline-none"
                />
              </div>

              <button
                type="button"
                onClick={() => void handleTestOperation(selectedOperation, true)}
                disabled={isRunning}
                className="transform-workspace-action operations flex h-9 w-full items-center justify-center gap-2 rounded-xl px-3 text-xs font-bold shadow-sm transition-[background-color,color,transform] active:scale-[0.99] disabled:cursor-wait disabled:opacity-60"
              >
                <Play className="h-3.5 w-3.5" />
                <span>{isRunning ? 'Running…' : 'Run Operation'}</span>
              </button>

              <div>
                <div className="mb-1.5 flex items-center justify-between gap-2">
                  <span className="text-[10px] font-semibold theme-text-muted">Output</span>
                  {testResult && !runError && <span className="text-[10px] theme-text-subtle">{testResult.length} characters</span>}
                </div>
                <div
                  className={`operation-playground-output min-h-28 whitespace-pre-wrap break-words rounded-xl border p-3 font-mono text-xs ${runError ? 'has-error' : ''}`}
                  aria-live="polite"
                >
                  {runError || testResult || 'Run the selected Operation to preview its output.'}
                </div>
              </div>

              <TransformationOutputActions output={runError ? '' : testResult} accent="operations" />
            </div>
          ) : (
            <div className="flex min-h-64 flex-col items-center justify-center p-6 text-center">
              <Code2 className="mb-3 h-6 w-6 transform-accent operations" />
              <h3 className="text-xs font-semibold theme-text-main">Choose an Operation</h3>
              <p className="mt-1 max-w-52 text-[10px] theme-text-muted">Select one from the library to inspect and test it.</p>
            </div>
          )}
        </aside>
      </div>

      <OperationEditorModal
        operation={selectedOperationForEdit}
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onSaveSuccess={fetchOperations}
      />
    </div>
  );

  return isEmbedded ? content : (
    <div className="tools-page tools-scroll-region operations-page filter-manager-wrapper h-screen flex-1 select-none space-y-6 overflow-y-auto p-6">
      {content}
    </div>
  );
};
