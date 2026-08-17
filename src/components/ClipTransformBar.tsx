import { Check, Clock3, LoaderCircle, RotateCcw, Workflow } from 'lucide-react';
import type { IntelligenceRequestStatus } from '../hooks/useIntelligenceRequestStatus';
import { OverflowText } from './OverflowText';
import { formatTransformRequestPhase, translate } from '../localization/runtime';

interface ClipTransformBarProps {
  activeTransformName: string;
  isRunning: boolean;
  hasPreview: boolean;
  error: string | null;
  onApply: () => void;
  onRetry: () => void;
  onReset: () => void;
  requestStatus?: IntelligenceRequestStatus;
}

export function ClipTransformBar({
  activeTransformName,
  isRunning,
  hasPreview,
  error,
  onApply,
  onRetry,
  onReset,
  requestStatus,
}: ClipTransformBarProps) {
  const runningLabel = formatTransformRequestPhase({
    phase: requestStatus?.phase ?? 'running',
    connectionName: requestStatus?.connectionName,
    didFallback: requestStatus?.didFallback,
  });
  const statusText = isRunning
    ? translate('component.clipTransformBar.statusTransform', { status: runningLabel, transform: activeTransformName })
    : translate('component.clipTransformBar.previewingTransform', { transform: activeTransformName });

  return (
    <div className="preview-filter-bar px-4 py-2.5 border-t select-none">
      <div className="flex items-center gap-3">
        <div className="flex min-w-0 flex-1 items-center space-x-2">
          <Workflow className="preview-filter-accent w-4 h-4" />
          <OverflowText text={statusText} className="theme-text-main truncate text-xs font-semibold" />
        </div>
        <div className="ml-auto flex items-center gap-1.5 shrink-0">
          <button
            type="button"
            onClick={onApply}
            disabled={isRunning || !hasPreview}
            className="transform-workspace-action pipelines flex items-center gap-1.5 rounded-lg px-2.5 py-1 text-xs font-bold transition-colors disabled:cursor-not-allowed disabled:opacity-40"
            title={translate('component.clipTransformBar.applyAndSaveRevision')}
          >
            {isRunning
              ? requestStatus?.phase === 'queued'
                ? <Clock3 className="h-3.5 w-3.5" />
                : <LoaderCircle className="h-3.5 w-3.5 animate-spin" />
              : <Check className="h-3.5 w-3.5" />}
            <span>{isRunning ? (requestStatus?.phase === 'queued' ? translate('component.clipTransformBar.queued') : translate('component.clipTransformBar.running')) : translate('component.clipTransformBar.apply')}</span>
          </button>
          <button
            type="button"
            onClick={onReset}
            className="preview-filter-reset px-2.5 py-1 rounded-lg border text-xs font-semibold transition-colors"
          >
            {translate('common.cancel')}
          </button>
        </div>
      </div>
      {!isRunning && hasPreview && (
        <p className="theme-text-muted mt-2 text-[10px]">{translate('component.clipTransformBar.previewOnlyApplyReplacesTheClipAndKeepsTheOriginalInRevision')}</p>
      )}
      {error && (
        <div role="status" className="theme-status-error mt-2 flex items-center gap-2 rounded-lg border px-2.5 py-1.5 text-[11px]">
          <span className="min-w-0 flex-1">{error}</span>
          <button type="button" onClick={onRetry} className="playground-run-status-action inline-flex shrink-0 items-center gap-1 rounded-md px-2 py-1 font-semibold">
            <RotateCcw className="h-3 w-3" /> {translate('common.retry')}
          </button>
        </div>
      )}
    </div>
  );
}
