import { AlertCircle, CheckCircle2, Clock3, LoaderCircle, RotateCcw, X } from 'lucide-react';
import type { IntelligenceRequestStatus } from '../hooks/useIntelligenceRequestStatus';

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

  const duration = typeof durationMs === 'number' ? `${(durationMs / 1000).toFixed(1)}s` : '';

  return (
    <div className={`playground-run-status is-${state} flex min-h-8 items-center gap-2 rounded-lg border px-2.5 py-1.5 text-[10px]`} aria-live="polite">
      {state === 'running' && requestStatus?.phase === 'queued'
        ? <Clock3 className="h-3.5 w-3.5 shrink-0" />
        : state === 'running' && <LoaderCircle className="h-3.5 w-3.5 shrink-0 animate-spin" />}
      {state === 'success' && <CheckCircle2 className="h-3.5 w-3.5 shrink-0" />}
      {state === 'error' && <AlertCircle className="h-3.5 w-3.5 shrink-0" />}
      {state === 'cancelled' && <X className="h-3.5 w-3.5 shrink-0" />}
      <span className="min-w-0 flex-1 truncate font-semibold">
        {state === 'running' && requestStatus?.phase === 'starting' && `Starting${label ? ` ${label}` : ''}…`}
        {state === 'running' && requestStatus?.phase === 'queued' && `Queued${requestStatus.connectionName ? ` for ${requestStatus.connectionName}` : ''}…`}
        {state === 'running' && requestStatus?.phase === 'running' && `Running${label ? ` ${label}` : ''}${requestStatus.connectionName ? ` with ${requestStatus.connectionName}` : ''}${requestStatus.didFallback ? ' · fallback' : ''}…`}
        {state === 'running' && !requestStatus && `Running${label ? ` ${label}` : ''}…`}
        {state === 'success' && `Ready${duration ? ` · ${duration}` : ''}`}
        {state === 'error' && `Couldn’t run${label ? ` ${label}` : ''}`}
        {state === 'cancelled' && 'Cancelled'}
      </span>
      {state === 'running' && onStop && (
        <button type="button" onClick={onStop} className="playground-run-status-action rounded-md px-2 py-1 font-semibold" title="Cancel Transform">
          Cancel
        </button>
      )}
      {(state === 'error' || state === 'cancelled') && onRetry && (
        <button type="button" onClick={onRetry} className="playground-run-status-action inline-flex items-center gap-1 rounded-md px-2 py-1 font-semibold">
          <RotateCcw className="h-3 w-3" /> Retry
        </button>
      )}
    </div>
  );
}
