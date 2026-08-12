import { useMemo } from 'react';
import { Play, Workflow, Wrench } from 'lucide-react';
import type { Operation, Pipeline, SavedTransform } from '../types';
import { PlaygroundRunStatus, type PlaygroundRunState } from './PlaygroundRunStatus';
import { TransformCategorySelect } from './TransformCategorySelect';
import { TransformationOutputActions } from './TransformationOutputActions';
import { TransformationPreviewPanel } from './TransformationPreviewPanel';
import type { IntelligenceRequestStatus } from '../hooks/useIntelligenceRequestStatus';
import { ActionButton } from './AppDialogLayout';

export type PlaygroundTarget =
  | { kind: 'transform'; item: SavedTransform }
  | { kind: 'operation'; item: Operation }
  | { kind: 'pipeline'; item: Pipeline };

interface TransformationPlaygroundProps {
  transforms: SavedTransform[];
  operations: Operation[];
  pipelines: Pipeline[];
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
  pipelines,
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
    ...pipelines.map((item) => ({ kind: 'pipeline' as const, item })),
  ], [operations, pipelines, transforms]);
  const options = targets.map((candidate) => ({
    value: targetValue(candidate),
    label: `${candidate.kind === 'transform' ? 'Transform' : candidate.kind === 'operation' ? 'Operation' : 'Pipeline'} · ${candidate.item.name}`,
  }));

  return (
    <div className="mx-auto w-full max-w-5xl space-y-4">
      <section className="theme-surface @container rounded-xl border p-4">
        <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
          <p className="theme-text-muted text-[10px]">Run a saved Transform, Operation, or Pipeline without changing a clip.</p>
          <TransformCategorySelect
            accent={target?.kind === 'operation' ? 'operations' : 'pipelines'}
            value={target ? targetValue(target) : options[0]?.value ?? ''}
            options={options.length ? options : [{ value: '', label: 'Nothing available' }]}
            onChange={(value) => {
              const nextTarget = targets.find((candidate) => targetValue(candidate) === value);
              if (nextTarget) onTargetChange(nextTarget);
            }}
            label="Choose what to run"
            leadingIcon={target?.kind === 'operation' ? <Wrench className="h-3.5 w-3.5 shrink-0" /> : <Workflow className="h-3.5 w-3.5 shrink-0" />}
          />
        </div>

        <TransformationPreviewPanel
          title="Preview"
          description={target ? `Testing ${targetName(target)}` : 'Choose an item to run'}
          status={<PlaygroundRunStatus
            state={runState}
            label={targetName(target)}
            durationMs={runDurationMs}
            onRetry={onRetry}
            onStop={onStop}
            requestStatus={requestStatus}
          />}
          input={<textarea
              id="shared-playground-input"
              value={input}
              onChange={(event) => onInputChange(event.target.value)}
              className="theme-input ui-field-radius h-48 w-full resize-y border p-3 font-mono text-xs focus:outline-none"
            />}
          output={<div className={`theme-input ui-field-radius h-48 overflow-y-auto whitespace-pre-wrap break-words border p-3 font-mono text-xs ${error ? 'theme-danger-text' : ''}`}>
              {error || output || 'Run the selected item to preview its output.'}
            </div>}
        />

        <div className="theme-divider mt-4 flex flex-wrap items-center justify-end gap-2 border-t pt-3">
          {output && !error && <span className="theme-text-subtle mr-auto text-[10px]">{output.length} characters</span>}
          <TransformationOutputActions output={error ? '' : output} />
          <ActionButton
            variant="primary"
            onClick={onRun}
            disabled={!target || runState === 'running'}
            className="h-9 min-h-9 px-5"
          >
            <Play className="h-3.5 w-3.5" />
            <span>{runState === 'running' ? (requestStatus?.phase === 'queued' ? 'Queued…' : 'Running…') : 'Run'}</span>
          </ActionButton>
        </div>
      </section>
    </div>
  );
}
