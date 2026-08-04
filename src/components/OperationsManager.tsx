import React, { useEffect, useMemo, useState } from 'react';
import { Operation } from '../types';
import {
  Code2,
  Edit3,
  LockKeyhole,
  Sparkles,
  Trash2,
  Wrench,
} from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { OperationEditorModal, CATEGORIES } from './OperationEditorModal';
import { startWindowDrag } from '../utils/windowDrag';
import { TransformLibraryToolbar } from './TransformLibraryToolbar';
import { TransformCategorySelect } from './TransformCategorySelect';

interface OperationsManagerProps {
  isEmbedded?: boolean;
  onOpenCreateModal?: () => void;
  onChooseOperation?: (operation: Operation) => void;
}

function isBuiltInOperation(operation: Operation) {
  return operation.stable_id.startsWith('builtin:');
}

export const OperationsManager: React.FC<OperationsManagerProps> = ({
  isEmbedded = false,
  onOpenCreateModal,
  onChooseOperation,
}) => {
  const [operations, setOperations] = useState<Operation[]>([]);
  const [activeCategory, setActiveCategory] = useState('All');
  const [selectedOperationId, setSelectedOperationId] = useState<string | null>(null);
  const [selectedOperationForEdit, setSelectedOperationForEdit] = useState<Operation | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);

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

  const dynamicCategories = useMemo(() => (
    Array.from(new Set([...CATEGORIES, ...operations.map((operation) => operation.category).filter(Boolean)]))
  ), [operations]);
  const categoryOptions = useMemo(() => [
    { value: 'All', label: 'All Operations', count: operations.length },
    ...dynamicCategories
      .map((category) => ({
        value: category,
        label: category,
        count: operations.filter((operation) => operation.category === category).length,
      }))
      .filter((option) => option.count > 0),
  ], [dynamicCategories, operations]);

  const filteredOperations = activeCategory === 'All'
    ? operations
    : operations.filter((operation) => operation.category === activeCategory);
  const builtInOperations = filteredOperations.filter(isBuiltInOperation);
  const customOperations = filteredOperations.filter((operation) => !isBuiltInOperation(operation));

  const renderOperationRow = (operation: Operation) => {
    const builtIn = isBuiltInOperation(operation);
    const selected = operation.stable_id === selectedOperationId;

    return (
      <div
        key={operation.stable_id}
        onClick={() => {
          setSelectedOperationId(operation.stable_id);
          onChooseOperation?.(operation);
        }}
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
              title="Edit Operation"
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
              title="Delete Operation"
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
            <p className="mt-1 text-xs theme-text-muted">Reusable building blocks for Advanced Transforms and Automations.</p>
          </div>
        </div>
      )}

      <TransformLibraryToolbar
        accent="operations"
        createLabel="New Operation"
        onCreate={onOpenCreateModal || handleOpenCreate}
      >
        <TransformCategorySelect
          accent="operations"
          value={activeCategory}
          options={categoryOptions}
          onChange={setActiveCategory}
          label="Filter Operations"
        />
      </TransformLibraryToolbar>

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
