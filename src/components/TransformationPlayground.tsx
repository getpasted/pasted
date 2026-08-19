import { useMemo } from 'react';
import { Play, Workflow, Wrench } from 'lucide-react';
import type { Operation, ManualTransform, SavedTransform } from '../types';
import { PlaygroundRunStatus, type PlaygroundRunState } from './PlaygroundRunStatus';
import { TransformCategorySelect } from './TransformCategorySelect';
import { TransformationOutputActions } from './TransformationOutputActions';
import { TransformationPreviewPanel } from './TransformationPreviewPanel';
import type { IntelligenceRequestStatus } from '../hooks/useIntelligenceRequestStatus';
import { ActionButton } from './AppDialogLayout';
import { translate } from '../localization/runtime';
import { localizedBuiltinName } from '../localization/presentation';

export type PlaygroundTarget =
  | { kind: 'transform'; item: SavedTransform }
  | { kind: 'operation'; item: Operation }
  | { kind: 'manual_transform'; item: ManualTransform };

interface TransformationPlaygroundProps {
  transforms: SavedTransform[];
  operations: Operation[];
  manualTransforms: ManualTransform[];
  target: PlaygroundTarget | null;
  input: string;
  output: string;
  error: string;
  runState: PlaygroundRunState;
  runDurationMs?: number;
  onTargetChange: (target: PlaygroundTarget) => void;
  onInputChange: (input: string) => void;
  onRun: () => void;
  onRetry: () => void;
  onStop: () => void;
  requestStatus?: IntelligenceRequestStatus;
}

function targetValue(target: PlaygroundTarget) {
  if (target.kind === 'operation') return `operation:${target.item.stable_id}`;
  return `${target.kind}:${target.item.stableRef}`;
}

function targetName(target: PlaygroundTarget | null) {
  return target?.item.name ?? '';
}

export function TransformationPlayground({
  transforms,
  operations,
  manualTransforms,
  target,
  input,
  output,
  error,
  runState,
  runDurationMs,
  onTargetChange,
  onInputChange,
  onRun,
  onRetry,
  onStop,
  requestStatus,
}: TransformationPlaygroundProps) {
  const targets = useMemo<PlaygroundTarget[]>(() => [
    ...transforms.map((item) => ({ kind: 'transform' as const, item })),
    ...operations.map((item) => ({ kind: 'operation' as const, item })),
    ...manualTransforms.map((item) => ({ kind: 'manual_transform' as const, item })),
  ], [operations, manualTransforms, transforms]);
  const options = targets
    .map((candidate, sourceIndex) => {
      const group = candidate.kind === 'operation'
        ? translate('component.transformationPlayground.operationsCategory', { category: candidate.item.category })
        : candidate.kind === 'manual_transform'
          ? translate('component.transformationPlayground.manuallyBuiltTransforms')
          : candidate.item.plan.steps.some((step) => step.executor.kind === 'semantic')
            ? translate('component.transformationPlayground.aiAssistedTransforms')
            : translate('component.transformationPlayground.plannedLocalTransforms');
      const groupOrder = candidate.kind === 'operation'
        ? 3
        : candidate.kind === 'manual_transform'
          ? 2
          : candidate.item.plan.steps.some((step) => step.executor.kind === 'semantic') ? 0 : 1;
      return {
        value: targetValue(candidate),
        label: candidate.kind === 'operation'
          ? localizedBuiltinName('operation', candidate.item.stable_id, candidate.item.name, candidate.item.stable_id.startsWith('builtin:'))
          : candidate.item.name,
        group,
        groupOrder,
        sourceIndex,
      };
    })
    .sort((left, right) => left.groupOrder - right.groupOrder
      || left.group.localeCompare(right.group)
      || left.sourceIndex - right.sourceIndex)
    .map(({ groupOrder: _groupOrder, sourceIndex: _sourceIndex, ...option }) => option);

  return (
    <div className="mx-auto w-full max-w-5xl space-y-4">
      <section className="theme-surface @container rounded-xl border p-4">
        <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
          <p className="theme-text-muted text-[10px]">{translate('component.transformationPlayground.runATransformOrOperationWithoutChangingAClip')}</p>
          <TransformCategorySelect
            accent={target?.kind === 'operation' ? 'operations' : 'manual-transforms'}
            value={target ? targetValue(target) : options[0]?.value ?? ''}
            options={options.length ? options : [{ value: '', get label() { return translate('component.transformationPlayground.nothingAvailable'); } }]}
            onChange={(value) => {
              const nextTarget = targets.find((candidate) => targetValue(candidate) === value);
              if (nextTarget) onTargetChange(nextTarget);
            }}
            label={translate('component.transformationPlayground.chooseWhatToRun')}
            searchable
            searchPlaceholder={translate('component.transformationPlayground.searchTransformsAndOperations')}
            leadingIcon={target?.kind === 'operation' ? <Wrench className="h-3.5 w-3.5 shrink-0" /> : <Workflow className="h-3.5 w-3.5 shrink-0" />}
          />
        </div>

        <TransformationPreviewPanel
          title={translate('component.transformationPlayground.preview')}
          description={target ? translate('component.transformationPlayground.testingValue', { value: targetName(target) }) : translate('component.transformationPlayground.chooseAnItemToRun')}
          status={<PlaygroundRunStatus
            state={runState}
            label={targetName(target)}
            durationMs={runDurationMs}
            onRetry={onRetry}
            onStop={onStop}
            requestStatus={requestStatus}
          />}
          input={<textarea dir="auto"
              id="shared-playground-input"
              value={input}
              onChange={(event) => onInputChange(event.target.value)}
              className="theme-input ui-field-radius h-48 w-full resize-y border p-3 font-mono text-xs focus:outline-none"
            />}
          output={<div dir="auto" className={`theme-input ui-field-radius h-48 overflow-y-auto whitespace-pre-wrap break-words border p-3 font-mono text-xs ${error ? 'theme-danger-text' : ''}`}>
              {error || output || translate('component.transformationPlayground.runTheSelectedItemToPreviewItsOutput')}
            </div>}
        />

        <div className="theme-divider mt-4 flex flex-wrap items-center justify-end gap-2 border-t pt-3">
          {output && !error && <span className="theme-text-subtle me-auto text-[10px]">{translate('format.characterCount', { count: output.length })}</span>}
          <TransformationOutputActions output={error ? '' : output} />
          <ActionButton
            variant="primary"
            onClick={onRun}
            disabled={!target || runState === 'running'}
            className="h-9 min-h-9 px-5"
          >
            <Play className="h-3.5 w-3.5" />
            <span>{runState === 'running' ? (requestStatus?.phase === 'queued' ? translate('component.transformationPlayground.queued') : translate('component.transformationPlayground.running')) : translate('component.transformationPlayground.run')}</span>
          </ActionButton>
        </div>
      </section>
    </div>
  );
}
