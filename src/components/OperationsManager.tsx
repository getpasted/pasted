import React, { useEffect, useMemo, useState } from 'react';
import { Operation, type LibraryItemView } from '../types';
import {
  Code2,
  Copy,
  Edit3,
  LockKeyhole,
  Play,
  Trash2,
  Wrench,
} from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { OperationEditorModal, CATEGORIES } from './OperationEditorModal';
import { handleWindowDragDoubleClick, startWindowDrag } from '../utils/windowDrag';
import { TransformLibraryToolbar } from './TransformLibraryToolbar';
import { TransformCategorySelect } from './TransformCategorySelect';
import { DeleteTransformationAssetDialog } from './DeleteTransformationAssetDialog';
import { OverflowText } from './OverflowText';
import { RegistryEditorShell } from './RegistryEditorShell';
import { RegistryEditorActions } from './RegistryEditorActions';
import { RegistryDetailHeader } from './RegistryDetailHeader';
import { RegistryListItem } from './RegistryListItem';
import { SettingsSwitch } from './SettingsSwitch';
import { ActionButton } from './AppDialogLayout';

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
  const [libraryItems, setLibraryItems] = useState<LibraryItemView[]>([]);
  const [activeCategory, setActiveCategory] = useState('All');
  const [selectedOperationId, setSelectedOperationId] = useState<string | null>(null);
  const [selectedOperationForEdit, setSelectedOperationForEdit] = useState<Operation | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [libraryError, setLibraryError] = useState('');
  const [operationToDelete, setOperationToDelete] = useState<Operation | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [togglingOperationId, setTogglingOperationId] = useState<string | null>(null);

  const fetchOperations = async () => {
    try {
      const [nextOperations, nextLibraryItems] = await Promise.all([
        invoke<Operation[]>('get_operations'),
        invoke<LibraryItemView[]>('get_library_items', { kind: 'operation', includeArchived: false }),
      ]);
      setOperations(nextOperations);
      setLibraryItems(nextLibraryItems);
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
      setLibraryError(error instanceof Error ? error.message : String(error));
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
    setLibraryError('');
    setIsDeleting(true);
    try {
      await invoke('delete_operation', { id: operation.id });
      await fetchOperations();
      setOperationToDelete(null);
    } catch (error) {
      setLibraryError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsDeleting(false);
    }
  };

  const handleDuplicate = async (operation: Operation) => {
    setLibraryError('');
    try {
      await invoke('duplicate_operation', {
        reference: operation.stable_id,
        name: `${operation.name} Copy`,
      });
      await fetchOperations();
    } catch (error) {
      setLibraryError(error instanceof Error ? error.message : String(error));
    }
  };

  const handleToggle = async (metadata: LibraryItemView) => {
    if (!metadata.capabilities.canDisable || metadata.enabled === null) return;
    setTogglingOperationId(metadata.stableRef);
    setLibraryError('');
    try {
      await invoke('set_library_item_enabled', {
        kind: 'operation',
        stableRef: metadata.stableRef,
        enabled: !metadata.enabled,
      });
      await fetchOperations();
    } catch (error) {
      setLibraryError(error instanceof Error ? error.message : String(error));
    } finally {
      setTogglingOperationId(null);
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
  const selectedOperation = operations.find(({ stable_id }) => stable_id === selectedOperationId) ?? null;
  const selectedMetadata = libraryItems.find(({ stableRef }) => stableRef === selectedOperationId) ?? null;

  const renderOperationRow = (operation: Operation) => {
    const metadata = libraryItems.find(({ stableRef }) => stableRef === operation.stable_id);
    const builtIn = metadata?.isBuiltin ?? isBuiltInOperation(operation);
    const selected = operation.stable_id === selectedOperationId;

    return (
      <RegistryListItem
        key={operation.stable_id}
        selected={selected}
        onSelect={() => setSelectedOperationId(operation.stable_id)}
        icon={<span className="theme-badge grid h-8 w-8 place-items-center rounded-lg border">
            <Code2 className="transform-accent operations h-4 w-4" />
          </span>}
        title={<OverflowText text={operation.name} className="block truncate text-xs" />}
        subtitle={operation.category}
        trailing={builtIn ? (
          <LockKeyhole className="mr-2 h-3.5 w-3.5 shrink-0 theme-text-subtle" aria-label="Built-in operation" />
        ) : metadata?.capabilities.canDisable ? (
          <SettingsSwitch
            checked={metadata.enabled ?? false}
            label={operation.name}
            busy={togglingOperationId === operation.stable_id}
            onClick={() => void handleToggle(metadata)}
            className="mr-1"
          />
        ) : null}
      />
    );
  };

  const content = (
    <div className="space-y-4">
      {!isEmbedded && (
        <div onMouseDown={startWindowDrag} onDoubleClick={handleWindowDragDoubleClick} className="theme-divider flex items-center justify-between border-b pb-4">
          <div>
            <h2 className="theme-title flex items-center space-x-2 text-sm font-bold">
              <Wrench className="transform-accent operations h-4 w-4 opacity-70" />
              <span>Operations</span>
            </h2>
            <p className="mt-1 text-xs theme-text-muted">Reusable building blocks for Transforms.</p>
          </div>
        </div>
      )}

      <TransformLibraryToolbar
        createLabel="New Operation"
        onCreate={onOpenCreateModal || handleOpenCreate}
      >
        <TransformCategorySelect
          accent="operations"
          value={activeCategory}
          options={categoryOptions}
          onChange={setActiveCategory}
          label="Filter operations"
        />
      </TransformLibraryToolbar>

      {libraryError && (
        <div role="alert" className="theme-status-danger rounded-xl border px-3 py-2 text-xs">
          {libraryError}
        </div>
      )}

      <RegistryEditorShell>
        <section className="theme-surface overflow-hidden rounded-xl border" aria-label="Operations">
          <div className="max-h-80 space-y-1 overflow-y-auto p-1.5 @4xl:max-h-[520px]">
            {filteredOperations.length > 0
              ? filteredOperations.map(renderOperationRow)
              : <p className="theme-text-muted px-2 py-4 text-xs">No Operations match this category.</p>}
          </div>
        </section>

        <section className="theme-surface min-w-0 rounded-xl border p-3 @md:p-4" aria-label="Operation details">
          {selectedOperation && selectedMetadata ? (
            <div className="flex h-full min-h-72 flex-col gap-4">
              <RegistryDetailHeader
                icon={<Code2 className="h-5 w-5" />}
                title={selectedOperation.name}
                meta={<>{selectedOperation.category} · {selectedMetadata.isBuiltin ? 'Built-in' : 'Custom'}</>}
                trailing={selectedMetadata.isBuiltin && <LockKeyhole className="h-4 w-4 theme-text-subtle" aria-label="Built-in operation" />}
                iconClassName="transform-accent operations"
              />

              <dl className="theme-subtle-surface divide-y theme-divide overflow-hidden rounded-xl border text-xs">
                <div className="grid grid-cols-[5rem_minmax(0,1fr)] items-center gap-3 px-3 py-2">
                  <dt className="theme-text-subtle text-[9px] font-bold uppercase tracking-wider">Input</dt>
                  <dd className="theme-text-main truncate font-mono">{selectedMetadata.inputContract}</dd>
                </div>
                <div className="grid grid-cols-[5rem_minmax(0,1fr)] items-center gap-3 px-3 py-2">
                  <dt className="theme-text-subtle text-[9px] font-bold uppercase tracking-wider">Output</dt>
                  <dd className="theme-text-main truncate font-mono">{selectedMetadata.outputContract}</dd>
                </div>
                <div className="grid grid-cols-[5rem_minmax(0,1fr)] items-center gap-3 px-3 py-2">
                  <dt className="theme-text-subtle text-[9px] font-bold uppercase tracking-wider">Executor</dt>
                  <dd className="theme-text-main truncate font-mono">{selectedOperation.op_type}</dd>
                </div>
              </dl>

              <p className="theme-text-muted text-xs leading-relaxed">
                {selectedMetadata.isBuiltin
                  ? 'This built-in Operation is maintained automatically and can be used directly in Transforms.'
                  : selectedMetadata.enabled
                    ? 'This custom Operation is enabled and available to Transforms.'
                    : 'This custom Operation is disabled and cannot run until it is enabled again.'}
              </p>

              <RegistryEditorActions
                leading={<>
                  <ActionButton onClick={() => onChooseOperation?.(selectedOperation)}>
                    <Play className="h-3.5 w-3.5" /> Test in Playground
                  </ActionButton>
                  {selectedMetadata.capabilities.canDuplicate && <ActionButton onClick={() => void handleDuplicate(selectedOperation)}><Copy className="h-3.5 w-3.5" /> Duplicate</ActionButton>}
                  {selectedMetadata.capabilities.canDelete && <ActionButton variant="danger" onClick={() => setOperationToDelete(selectedOperation)}><Trash2 className="h-3.5 w-3.5" /> Delete</ActionButton>}
                </>}
                trailing={selectedMetadata.capabilities.canEdit && <ActionButton variant="primary" onClick={() => handleOpenEdit(selectedOperation)}><Edit3 className="h-3.5 w-3.5" /> Edit</ActionButton>}
              />
            </div>
          ) : (
            <div className="grid min-h-72 place-items-center text-center">
              <p className="theme-text-muted text-xs">Select an Operation to see its settings.</p>
            </div>
          )}
        </section>
      </RegistryEditorShell>

      <OperationEditorModal
        operation={selectedOperationForEdit}
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onSaveSuccess={fetchOperations}
      />
      <DeleteTransformationAssetDialog
        asset={operationToDelete ? { kind: 'Operation', name: operationToDelete.name } : null}
        isDeleting={isDeleting}
        onCancel={() => setOperationToDelete(null)}
        onConfirm={() => operationToDelete ? handleDelete(operationToDelete) : undefined}
      />
    </div>
  );

  return isEmbedded ? content : (
    <div className="tools-page tools-scroll-region operations-page filter-manager-wrapper h-screen flex-1 select-none space-y-6 overflow-y-auto p-6">
      {content}
    </div>
  );
};
