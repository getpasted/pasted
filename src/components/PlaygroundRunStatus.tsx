import { AlertCircle, CheckCircle2, LoaderCircle, RotateCcw, X } from 'lucide-react';

export type PlaygroundRunState = 'idle' | 'running' | 'success' | 'error' | 'stopped';

interface PlaygroundRunStatusProps {
  state: PlaygroundRunState;
  label?: string;
  durationMs?: number;
  onRetry?: () => void;
  onStop?: () => void;
}

export function PlaygroundRunStatus({ state, label, durationMs, onRetry, onStop }: PlaygroundRunStatusProps) {
  if (state === 'idle') return null;

  const duration = typeof durationMs === 'number' ? `${(durationMs / 1000).toFixed(1)}s` : '';

  return (
    <div className={`playground-run-status is-${state} flex min-h-8 items-center gap-2 rounded-lg border px-2.5 py-1.5 text-[10px]`} aria-live="polite">
      {state === 'running' && <LoaderCircle className="h-3.5 w-3.5 shrink-0 animate-spin" />}
      {state === 'success' && <CheckCircle2 className="h-3.5 w-3.5 shrink-0" />}
      {state === 'error' && <AlertCircle className="h-3.5 w-3.5 shrink-0" />}
      {state === 'stopped' && <X className="h-3.5 w-3.5 shrink-0" />}
      <span className="min-w-0 flex-1 truncate font-semibold">
        {state === 'running' && `Running${label ? ` ${label}` : ''}…`}
        {state === 'success' && `Ready${duration ? ` · ${duration}` : ''}`}
        {state === 'error' && `Couldn’t run${label ? ` ${label}` : ''}`}
        {state === 'stopped' && 'Stopped waiting'}
      </span>
      {state === 'running' && onStop && (
        <button type="button" onClick={onStop} className="playground-run-status-action rounded-md px-2 py-1 font-semibold" title="Stop waiting; the provider may finish in the background">
          Stop
        </button>
      )}
      {(state === 'error' || state === 'stopped') && onRetry && (
        <button type="button" onClick={onRetry} className="playground-run-status-action inline-flex items-center gap-1 rounded-md px-2 py-1 font-semibold">
          <RotateCcw className="h-3 w-3" /> Retry
        </button>
      )}
    </div>
  );
}
