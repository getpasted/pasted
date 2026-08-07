import { useMemo } from 'react';
import { Play, Workflow, Wrench } from 'lucide-react';
import type { Operation, Pipeline, SavedTransform } from '../types';
import { PlaygroundRunStatus, type PlaygroundRunState } from './PlaygroundRunStatus';
import { TransformCategorySelect } from './TransformCategorySelect';
import { TransformationOutputActions } from './TransformationOutputActions';
import type { IntelligenceRequestStatus } from '../hooks/useIntelligenceRequestStatus';

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
      <section className="filter-sandbox-card border p-5 shadow-xl">
        <div className="mb-4 flex flex-wrap items-start justify-between gap-3">
          <div>
            <div className="filter-sandbox-heading pipelines flex items-center gap-2 text-xs font-semibold uppercase tracking-wider">
              <Play className="h-4 w-4" />
              <span>Playground</span>
            </div>
            <p className="mt-1 text-[10px] theme-text-muted">Run a saved Transform, one Operation, or a legacy Pipeline without changing a clip.</p>
          </div>
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

        <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
          <div>
            <label htmlFor="shared-playground-input" className="mb-1.5 block text-[10px] font-semibold theme-text-muted">Input</label>
            <textarea
              id="shared-playground-input"
              value={input}
              onChange={(event) => onInputChange(event.target.value)}
              className="theme-input ui-field-radius h-48 w-full resize-y border p-3 font-mono text-xs focus:outline-none"
            />
          </div>
          <div>
            <div className="mb-1.5 flex items-center justify-between gap-2">
              <span className="text-[10px] font-semibold theme-text-muted">Output</span>
              {output && !error && <span className="text-[10px] theme-text-subtle">{output.length} characters</span>}
            </div>
            <div className={`operation-playground-output ui-field-radius h-48 overflow-y-auto whitespace-pre-wrap break-words border p-3 font-mono text-xs ${error ? 'has-error' : ''}`}>
              {error || output || 'Run the selected item to preview its output.'}
            </div>
          </div>
        </div>

        <div className="mt-4 grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center">
          <PlaygroundRunStatus
            state={runState}
            label={targetName(target)}
            durationMs={runDurationMs}
            onRetry={onRetry}
            onStop={onStop}
            requestStatus={requestStatus}
          />
          <button
            type="button"
            onClick={onRun}
            disabled={!target || runState === 'running'}
            className={`transform-workspace-action ui-control-radius ${target?.kind === 'operation' ? 'operations' : 'pipelines'} flex h-9 items-center justify-center gap-2 px-5 text-xs font-bold shadow-sm disabled:cursor-not-allowed disabled:opacity-45`}
          >
            <Play className="h-3.5 w-3.5" />
            <span>{runState === 'running' ? (requestStatus?.phase === 'queued' ? 'Queued…' : 'Running…') : 'Run'}</span>
          </button>
        </div>
        <div className="mt-3">
          <TransformationOutputActions output={error ? '' : output} accent={target?.kind === 'operation' ? 'operations' : 'pipelines'} />
        </div>
      </section>
    </div>
  );
}
