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
import { translate } from '../localization/runtime';
import { localizedBuiltinName } from '../localization/presentation';

type LibrarySelection = { stableRef: string };
type TransformLibraryItem =
  | { storage: 'saved'; stableRef: string; updatedAt: string; item: SavedTransform }
  | { storage: 'manual'; stableRef: string; updatedAt: string; item: Pipeline };

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
  const visibleItems = useMemo<TransformLibraryItem[]>(() => {
    const items: TransformLibraryItem[] = [
      ...transforms.map((item) => ({ storage: 'saved' as const, stableRef: item.stableRef, updatedAt: item.updatedAt, item })),
      ...pipelines.map((item) => ({ storage: 'manual' as const, stableRef: item.stableRef, updatedAt: item.updatedAt, item })),
    ];
    return items
      .filter((candidate) => {
        if (filter === 'all') return true;
        if (candidate.storage === 'manual') return filter === 'local';
        const isAssisted = candidate.item.plan.steps.some((step) => step.executor.kind === 'semantic');
        return filter === (isAssisted ? 'assisted' : 'local');
      })
      .sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));
  }, [filter, pipelines, transforms]);
  const effectiveItem = visibleItems.find(({ stableRef }) => stableRef === selection?.stableRef) ?? visibleItems[0] ?? null;

  return (
    <div className="space-y-4">
      <TransformLibraryToolbar
        createLabel={translate('component.transformationLibrary.newTransform')}
        onCreate={onCreateTransform}
        secondaryAction={{ get label() { return translate('component.transformationLibrary.buildManually'); }, onClick: onCreatePipeline }}
      >
        <TransformCategorySelect
          accent="pipelines"
          value={filter}
          options={filterOptions}
          onChange={onFilterChange}
          label={translate('component.transformationLibrary.filterLibrary')}
        />
      </TransformLibraryToolbar>

      <RegistryEditorShell>
        <section className="theme-surface overflow-hidden rounded-xl border" aria-label={translate('component.transformationLibrary.transformationLibrary')}>
          <div className="max-h-96 overflow-y-auto p-1.5 @4xl:max-h-[560px]">
            {visibleItems.length > 0 && (
              <div className="space-y-1">
                <p className="theme-text-subtle px-2 pb-0.5 pt-1 text-[9px] font-bold uppercase tracking-wider">{translate('component.transformationLibrary.transforms')}</p>
                {visibleItems.map((candidate) => {
                  const semanticSteps = candidate.storage === 'saved'
                    ? candidate.item.plan.steps.filter((step) => step.executor.kind === 'semantic').length
                    : 0;
                  const stepCount = candidate.storage === 'saved' ? candidate.item.plan.steps.length : candidate.item.steps.length;
                  return <RegistryListItem
                    key={candidate.stableRef}
                    selected={effectiveItem?.stableRef === candidate.stableRef}
                    onSelect={() => setSelection({ stableRef: candidate.stableRef })}
                    icon={<span className="theme-badge grid h-8 w-8 place-items-center rounded-lg border">
                      {semanticSteps > 0 ? <Sparkles className="transform-accent pipelines h-4 w-4" /> : <Workflow className="transform-accent pipelines h-4 w-4" />}
                    </span>}
                    title={candidate.item.name}
                    subtitle={semanticSteps > 0 ? translate('component.transformationLibrary.aiAssisted') : translate('component.transformationLibrary.localReplayable')}
                    trailing={<span className="theme-text-subtle tabular-nums text-[9px]">{stepCount}</span>}
                  />;
                })}
              </div>
            )}

            {visibleItems.length === 0 && (
              <div className="grid min-h-56 place-items-center px-4 text-center">
                <div>
                  <Workflow className="transform-accent pipelines mx-auto mb-2 h-5 w-5" />
                  <p className="theme-text-main text-xs font-semibold">{translate('component.transformationLibrary.nothingInThisViewYet')}</p>
                  <p className="theme-text-muted mt-1 text-[10px]">{translate('component.transformationLibrary.createOrManuallyBuildATransformToAddItToTheLibrary')}</p>
                </div>
              </div>
            )}
          </div>
        </section>

        <section className="theme-surface min-w-0 rounded-xl border p-3 @md:p-4" aria-label={translate('component.transformationLibrary.transformationDetails')}>
          {effectiveItem?.storage === 'saved' ? (
            <TransformDetails
              transform={effectiveItem.item}
              operations={operations}
              onTest={() => onTestTransform(effectiveItem.item)}
              onEdit={() => onEditTransform(effectiveItem.item)}
              onDuplicate={() => onDuplicateTransform(effectiveItem.item)}
              onDelete={() => onDeleteTransform(effectiveItem.item)}
            />
          ) : effectiveItem?.storage === 'manual' ? (
            <PipelineDetails
              pipeline={effectiveItem.item}
              operations={operations}
              onTest={() => onTestPipeline(effectiveItem.item)}
              onEdit={() => onEditPipeline(effectiveItem.item)}
              onDuplicate={() => onDuplicatePipeline(effectiveItem.item)}
              onDelete={() => onDeletePipeline(effectiveItem.item)}
              onShortcutChange={(shortcut) => onPipelineShortcutChange(effectiveItem.item, shortcut)}
            />
          ) : (
            <div className="grid min-h-72 place-items-center text-center"><p className="theme-text-muted text-xs">{translate('component.transformationLibrary.selectOrCreateALibraryItem')}</p></div>
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
    ? transform.connectionId
      ? translate('component.transformationLibrary.aiAssistedPinnedConnection')
      : translate('component.transformationLibrary.aiAssistedAutomaticConnection')
    : translate('component.transformationLibrary.localReplayable');
  return (
    <div className="flex h-full flex-col gap-4">
      <RegistryDetailHeader icon={semanticSteps > 0 ? <Sparkles className="h-5 w-5" /> : <Workflow className="h-5 w-5" />} title={transform.name} meta={translate('component.transformationLibrary.transformRevisionRevision', { revision: transform.revision })} iconClassName="transform-accent pipelines" />
      <div className="theme-subtle-surface rounded-xl border p-3">
        <p className="theme-text-main text-xs font-semibold">{transform.plan.summary || transform.plan.intent}</p>
        <p className="theme-text-muted mt-1 text-[10px]">{provenance}</p>
      </div>
      <ol className="space-y-2">
        {transform.plan.steps.map((step, index) => {
          const operationRef = step.executor.kind === 'deterministic' ? step.executor.operation_ref : null;
          const name = operationRef
            ? (() => {
                const operation = operations.find(({ stable_id }) => stable_id === operationRef);
                return operation ? localizedBuiltinName('operation', operation.stable_id, operation.name, operation.stable_id.startsWith('builtin:')) : step.name;
              })()
            : step.name;
          return <LibraryStep key={`${step.name}-${index}`} index={index} name={name} meta={step.executor.kind === 'semantic' ? translate('component.transformationLibrary.semantic') : translate('component.transformationLibrary.deterministic')} />;
        })}
      </ol>
      <RegistryEditorActions
        leading={<>
          <ActionButton onClick={onTest}><Play className="h-3.5 w-3.5" /> {translate('action.testInPlayground')}</ActionButton>
          <ActionButton onClick={onDuplicate}><Copy className="h-3.5 w-3.5" /> {translate('common.duplicate')}</ActionButton>
          <ActionButton variant="danger" onClick={onDelete}><Trash2 className="h-3.5 w-3.5" /> {translate('common.delete')}</ActionButton>
        </>}
        trailing={<ActionButton variant="primary" onClick={onEdit}><Edit3 className="h-3.5 w-3.5" /> {translate('common.edit')}</ActionButton>}
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
        meta={translate('component.transformationLibrary.transformLocalBuilderRevisionRevision', { revision: pipeline.revision })}
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
          <ActionButton onClick={onTest}><Play className="h-3.5 w-3.5" /> {translate('action.testInPlayground')}</ActionButton>
          <ActionButton onClick={onDuplicate}><Copy className="h-3.5 w-3.5" /> {translate('common.duplicate')}</ActionButton>
          <ActionButton variant="danger" onClick={onDelete}><Trash2 className="h-3.5 w-3.5" /> {translate('common.delete')}</ActionButton>
        </>}
        trailing={<ActionButton variant="primary" onClick={onEdit}><Edit3 className="h-3.5 w-3.5" /> {translate('common.edit')}</ActionButton>}
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
