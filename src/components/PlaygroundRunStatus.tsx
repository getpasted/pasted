import { AlertCircle, CheckCircle2, Clock3, LoaderCircle, RotateCcw, X } from 'lucide-react';
import type { IntelligenceRequestStatus } from '../hooks/useIntelligenceRequestStatus';
import { OverflowText } from './OverflowText';
import { formatNumber, formatTransformRequestPhase, translate } from '../localization/runtime';

export type PlaygroundRunState = 'idle' | 'running' | 'success' | 'error' | 'cancelled';

interface PlaygroundRunStatusProps {
  state: PlaygroundRunState;
  label?: string;
  durationMs?: number;
  onRetry?: () => void;
  onStop?: () => void;
  requestStatus?: IntelligenceRequestStatus;
}

export function PlaygroundRunStatus({ state, label, durationMs, onRetry, onStop, requestStatus }: PlaygroundRunStatusProps) {
  if (state === 'idle') return null;

  const duration = typeof durationMs === 'number'
    ? translate('component.playgroundRunStatus.durationSeconds', { seconds: formatNumber(durationMs / 1000, { maximumFractionDigits: 1, minimumFractionDigits: 1 }) })
    : '';
  const statusText = state === 'running' && requestStatus?.phase === 'starting'
    ? formatTransformRequestPhase({ phase: 'starting', label, ellipsis: true })
    : state === 'running' && requestStatus?.phase === 'queued'
      ? formatTransformRequestPhase({ ...requestStatus, ellipsis: true })
      : state === 'running' && requestStatus?.phase === 'running'
        ? formatTransformRequestPhase({ ...requestStatus, label, ellipsis: true })
        : state === 'running'
          ? formatTransformRequestPhase({ phase: 'running', label, ellipsis: true })
          : state === 'success'
            ? duration ? translate('component.playgroundRunStatus.readyDuration', { duration }) : translate('component.playgroundRunStatus.ready')
            : state === 'error'
              ? label ? translate('component.playgroundRunStatus.couldNotRunLabel', { label }) : translate('component.playgroundRunStatus.couldNotRun')
              : translate('component.playgroundRunStatus.cancelled');

  return (
    <div className={`playground-run-status is-${state} flex min-h-8 items-center gap-2 rounded-lg border px-2.5 py-1.5 text-[10px]`} aria-live="polite">
      {state === 'running' && requestStatus?.phase === 'queued'
        ? <Clock3 className="h-3.5 w-3.5 shrink-0" />
        : state === 'running' && <LoaderCircle className="h-3.5 w-3.5 shrink-0 animate-spin" />}
      {state === 'success' && <CheckCircle2 className="h-3.5 w-3.5 shrink-0" />}
      {state === 'error' && <AlertCircle className="h-3.5 w-3.5 shrink-0" />}
      {state === 'cancelled' && <X className="h-3.5 w-3.5 shrink-0" />}
      <OverflowText text={statusText} className="min-w-0 flex-1 truncate font-semibold" />
      {state === 'running' && onStop && (
        <button type="button" onClick={onStop} className="playground-run-status-action rounded-md px-2 py-1 font-semibold" title={translate('component.playgroundRunStatus.cancelTransform')}>
          {translate('common.cancel')}
        </button>
      )}
      {(state === 'error' || state === 'cancelled') && onRetry && (
        <button type="button" onClick={onRetry} className="playground-run-status-action inline-flex items-center gap-1 rounded-md px-2 py-1 font-semibold">
          <RotateCcw className="h-3 w-3" /> {translate('common.retry')}
        </button>
      )}
    </div>
  );
}
