import { useMemo, useState } from 'react';
import { Copy, Edit3, Play, Sparkles, Trash2, Workflow } from 'lucide-react';
import type { Operation, Pipeline, SavedTransform } from '../types';
import { HotkeyRecorder } from './HotkeyRecorder';
import { RegistryEditorActions } from './RegistryEditorActions';
import { RegistryDetailHeader } from './RegistryDetailHeader';
import { RegistryEditorShell } from './RegistryEditorShell';
import { RegistryListItem } from './RegistryListItem';
import { TransformCategorySelect, type TransformCategoryOption } from './TransformCategorySelect';
import { TransformLibraryToolbar } from './TransformLibraryToolbar';
import { ActionButton } from './AppDialogLayout';

type LibrarySelection = { kind: 'transform' | 'pipeline'; stableRef: string };

interface TransformationLibraryProps {
  transforms: SavedTransform[];
  pipelines: Pipeline[];
  operations: Operation[];
  filter: string;
  filterOptions: TransformCategoryOption[];
  onFilterChange: (filter: string) => void;
  onCreateTransform: () => void;
  onCreatePipeline: () => void;
  onTestTransform: (transform: SavedTransform) => void;
  onTestPipeline: (pipeline: Pipeline) => void;
  onEditTransform: (transform: SavedTransform) => void;
  onEditPipeline: (pipeline: Pipeline) => void;
  onDuplicateTransform: (transform: SavedTransform) => void;
  onDuplicatePipeline: (pipeline: Pipeline) => void;
  onDeleteTransform: (transform: SavedTransform) => void;
  onDeletePipeline: (pipeline: Pipeline) => void;
  onPipelineShortcutChange: (pipeline: Pipeline, shortcut: string | null) => void;
}

export function TransformationLibrary({
  transforms,
  pipelines,
  operations,
  filter,
  filterOptions,
  onFilterChange,
  onCreateTransform,
  onCreatePipeline,
  onTestTransform,
  onTestPipeline,
  onEditTransform,
  onEditPipeline,
  onDuplicateTransform,
  onDuplicatePipeline,
  onDeleteTransform,
  onDeletePipeline,
  onPipelineShortcutChange,
}: TransformationLibraryProps) {
  const [selection, setSelection] = useState<LibrarySelection | null>(null);
  const showTransforms = filter === 'all' || filter === 'saved';
  const showPipelines = filter === 'all' || filter === 'pipelines' || filter.startsWith('pipeline:');
  const pipelineCategory = filter.startsWith('pipeline:') ? filter.slice('pipeline:'.length) : null;
  const visibleTransforms = showTransforms ? transforms : [];
  const visiblePipelines = useMemo(() => showPipelines
    ? pipelines.filter((pipeline) => !pipelineCategory || pipeline.steps.some((step) => (
      operations.find((operation) => operation.stable_id === step.operationRef)?.category === pipelineCategory
    )))
    : [], [operations, pipelineCategory, pipelines, showPipelines]);

  const selectedTransform = selection?.kind === 'transform'
    ? visibleTransforms.find(({ stableRef }) => stableRef === selection.stableRef) ?? null
    : null;
  const selectedPipeline = selection?.kind === 'pipeline'
    ? visiblePipelines.find(({ stableRef }) => stableRef === selection.stableRef) ?? null
    : null;
  const effectiveTransform = selectedTransform ?? (!selectedPipeline ? visibleTransforms[0] ?? null : null);
  const effectivePipeline = selectedPipeline ?? (!effectiveTransform ? visiblePipelines[0] ?? null : null);

  const selectTransform = (transform: SavedTransform) => setSelection({ kind: 'transform', stableRef: transform.stableRef });
  const selectPipeline = (pipeline: Pipeline) => setSelection({ kind: 'pipeline', stableRef: pipeline.stableRef });

  return (
    <div className="space-y-4">
      <TransformLibraryToolbar
        createLabel="New Transform"
        onCreate={onCreateTransform}
        secondaryAction={{ label: 'New Pipeline', onClick: onCreatePipeline }}
      >
        <TransformCategorySelect
          accent="pipelines"
          value={filter}
          options={filterOptions}
          onChange={onFilterChange}
          label="Filter Library"
        />
      </TransformLibraryToolbar>

      <RegistryEditorShell>
        <section className="theme-surface overflow-hidden rounded-xl border" aria-label="Transformation library">
          <div className="max-h-96 overflow-y-auto p-1.5 @4xl:max-h-[560px]">
            {visibleTransforms.length > 0 && (
              <div className="space-y-1">
                <p className="theme-text-subtle px-2 pb-0.5 pt-1 text-[9px] font-bold uppercase tracking-wider">Transforms</p>
                {visibleTransforms.map((transform) => {
                  const semanticSteps = transform.plan.steps.filter((step) => step.executor.kind === 'semantic').length;
                  return <RegistryListItem
                    key={transform.stableRef}
                    selected={effectiveTransform?.stableRef === transform.stableRef}
                    onSelect={() => selectTransform(transform)}
                    icon={<span className="theme-badge grid h-8 w-8 place-items-center rounded-lg border">
                      {semanticSteps > 0 ? <Sparkles className="transform-accent pipelines h-4 w-4" /> : <Workflow className="transform-accent pipelines h-4 w-4" />}
                    </span>}
                    title={transform.name}
                    subtitle={`Revision ${transform.revision}`}
                    trailing={<span className="theme-text-subtle tabular-nums text-[9px]">{transform.plan.steps.length}</span>}
                  />;
                })}
              </div>
            )}

            {visiblePipelines.length > 0 && (
              <div className={`space-y-1 ${visibleTransforms.length > 0 ? 'mt-3' : ''}`}>
                <p className="theme-text-subtle px-2 pb-0.5 pt-1 text-[9px] font-bold uppercase tracking-wider">Pipelines</p>
                {visiblePipelines.map((pipeline) => <RegistryListItem
                  key={pipeline.stableRef}
                  selected={effectivePipeline?.stableRef === pipeline.stableRef}
                  onSelect={() => selectPipeline(pipeline)}
                  icon={<span className="theme-badge grid h-8 w-8 place-items-center rounded-lg border">
                    <Workflow className="transform-accent pipelines h-4 w-4" />
                  </span>}
                  title={pipeline.name}
                  subtitle={`Revision ${pipeline.revision}`}
                  trailing={<span className="theme-text-subtle tabular-nums text-[9px]">{pipeline.steps.length}</span>}
                />)}
              </div>
            )}

            {visibleTransforms.length === 0 && visiblePipelines.length === 0 && (
              <div className="grid min-h-56 place-items-center px-4 text-center">
                <div>
                  <Workflow className="transform-accent pipelines mx-auto mb-2 h-5 w-5" />
                  <p className="theme-text-main text-xs font-semibold">Nothing in this view yet</p>
                  <p className="theme-text-muted mt-1 text-[10px]">Create a Transform or Pipeline to add it to the Library.</p>
                </div>
              </div>
            )}
          </div>
        </section>

        <section className="theme-surface min-w-0 rounded-xl border p-3 @md:p-4" aria-label="Transformation details">
          {effectiveTransform ? (
            <TransformDetails
              transform={effectiveTransform}
              operations={operations}
              onTest={() => onTestTransform(effectiveTransform)}
              onEdit={() => onEditTransform(effectiveTransform)}
              onDuplicate={() => onDuplicateTransform(effectiveTransform)}
              onDelete={() => onDeleteTransform(effectiveTransform)}
            />
          ) : effectivePipeline ? (
            <PipelineDetails
              pipeline={effectivePipeline}
              operations={operations}
              onTest={() => onTestPipeline(effectivePipeline)}
              onEdit={() => onEditPipeline(effectivePipeline)}
              onDuplicate={() => onDuplicatePipeline(effectivePipeline)}
              onDelete={() => onDeletePipeline(effectivePipeline)}
              onShortcutChange={(shortcut) => onPipelineShortcutChange(effectivePipeline, shortcut)}
            />
          ) : (
            <div className="grid min-h-72 place-items-center text-center"><p className="theme-text-muted text-xs">Select or create a Library item.</p></div>
          )}
        </section>
      </RegistryEditorShell>
    </div>
  );
}

function TransformDetails({
  transform,
  operations,
  onTest,
  onEdit,
  onDuplicate,
  onDelete,
}: {
  transform: SavedTransform;
  operations: Operation[];
  onTest: () => void;
  onEdit: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
}) {
  const semanticSteps = transform.plan.steps.filter((step) => step.executor.kind === 'semantic').length;
  const provenance = semanticSteps > 0
    ? transform.connectionId ? 'AI-assisted · pinned connection' : 'AI-assisted · automatic connection'
    : 'Local · replayable';
  return (
    <div className="flex h-full flex-col gap-4">
      <RegistryDetailHeader icon={semanticSteps > 0 ? <Sparkles className="h-5 w-5" /> : <Workflow className="h-5 w-5" />} title={transform.name} meta={`Transform · Revision ${transform.revision}`} iconClassName="transform-accent pipelines" />
      <div className="theme-subtle-surface rounded-xl border p-3">
        <p className="theme-text-main text-xs font-semibold">{transform.plan.summary || transform.plan.intent}</p>
        <p className="theme-text-muted mt-1 text-[10px]">{provenance}</p>
      </div>
      <ol className="space-y-2">
        {transform.plan.steps.map((step, index) => {
          const operationRef = step.executor.kind === 'deterministic' ? step.executor.operation_ref : null;
          const name = operationRef
            ? operations.find(({ stable_id }) => stable_id === operationRef)?.name ?? step.name
            : step.name;
          return <LibraryStep key={`${step.name}-${index}`} index={index} name={name} meta={step.executor.kind === 'semantic' ? 'Semantic' : 'Deterministic'} />;
        })}
      </ol>
      <RegistryEditorActions
        leading={<>
          <ActionButton onClick={onTest}><Play className="h-3.5 w-3.5" /> Test in Playground</ActionButton>
          <ActionButton onClick={onDuplicate}><Copy className="h-3.5 w-3.5" /> Duplicate</ActionButton>
          <ActionButton variant="danger" onClick={onDelete}><Trash2 className="h-3.5 w-3.5" /> Delete</ActionButton>
        </>}
        trailing={<ActionButton variant="primary" onClick={onEdit}><Edit3 className="h-3.5 w-3.5" /> Edit</ActionButton>}
      />
    </div>
  );
}

function PipelineDetails({
  pipeline,
  operations,
  onTest,
  onEdit,
  onDuplicate,
  onDelete,
  onShortcutChange,
}: {
  pipeline: Pipeline;
  operations: Operation[];
  onTest: () => void;
  onEdit: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
  onShortcutChange: (shortcut: string | null) => void;
}) {
  return (
    <div className="flex h-full flex-col gap-4">
      <RegistryDetailHeader
        icon={<Workflow className="h-5 w-5" />}
        title={pipeline.name}
        meta={`Pipeline · Revision ${pipeline.revision}`}
        trailing={<HotkeyRecorder value={pipeline.shortcut} onChange={onShortcutChange} />}
        iconClassName="transform-accent pipelines"
      />
      <ol className="space-y-2">
        {pipeline.steps.map((step, index) => (
          <LibraryStep
            key={`${step.operationRef}-${index}`}
            index={index}
            name={operations.find(({ stable_id }) => stable_id === step.operationRef)?.name ?? step.operationRef.replace(/^(?:builtin|custom):/, '')}
            meta={step.failurePolicy}
          />
        ))}
      </ol>
      <RegistryEditorActions
        leading={<>
          <ActionButton onClick={onTest}><Play className="h-3.5 w-3.5" /> Test in Playground</ActionButton>
          <ActionButton onClick={onDuplicate}><Copy className="h-3.5 w-3.5" /> Duplicate</ActionButton>
          <ActionButton variant="danger" onClick={onDelete}><Trash2 className="h-3.5 w-3.5" /> Delete</ActionButton>
        </>}
        trailing={<ActionButton variant="primary" onClick={onEdit}><Edit3 className="h-3.5 w-3.5" /> Edit</ActionButton>}
      />
    </div>
  );
}

function LibraryStep({ index, name, meta }: { index: number; name: string; meta: string }) {
  return (
    <li className="theme-card-idle flex items-center gap-3 border p-3">
      <span className="theme-text-subtle grid h-5 w-5 shrink-0 place-items-center rounded-full border text-[9px] font-bold">{index + 1}</span>
      <span className="theme-text-main min-w-0 flex-1 truncate text-xs font-semibold">{name}</span>
      <span className="theme-text-subtle text-[9px] capitalize">{meta}</span>
    </li>
  );
}
