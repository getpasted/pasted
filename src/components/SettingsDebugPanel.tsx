import { useEffect, useState } from 'react';
import { Activity, Ban, CheckCircle2, CirclePlay, Clock3, GitFork, Layers3, LoaderCircle, ScanText, Shuffle, Square, XCircle } from 'lucide-react';
import type { IntelligenceSchedulerEvent, IntelligenceSchedulerSnapshot, OcrBackfillStatus } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
import { OverflowText } from './OverflowText';
import { useToast } from './ToastProvider';
import { ActionButton } from './AppDialogLayout';
import { useMinuteTick } from '../hooks/useMinuteTick';
import { dateTimeAttribute, formatFullDateTime, formatRelativeTime } from '../utils/date';

const EMPTY_SNAPSHOT: IntelligenceSchedulerSnapshot = {
  revision: 0,
  activeCount: 0,
  queuedCount: 0,
  jobs: [],
  recentEvents: [],
};

const EMPTY_OCR_STATUS: OcrBackfillStatus = {
  totalImages: 0,
  eligibleCount: 0,
  queuedCount: 0,
  runningCount: 0,
  completedCount: 0,
  noTextCount: 0,
  failedCount: 0,
};

function duration(milliseconds: number) {
  if (milliseconds < 1_000) return `${Math.round(milliseconds)} ms`;
  return `${(milliseconds / 1_000).toFixed(1)} s`;
}

function eventIcon(event: IntelligenceSchedulerEvent) {
  if (event.status === 'running') return <CirclePlay className="h-3.5 w-3.5" />;
  if (event.status === 'succeeded') return <CheckCircle2 className="h-3.5 w-3.5" />;
  if (event.status === 'failed' || event.status === 'cancelled') return <XCircle className="h-3.5 w-3.5" />;
  return <Clock3 className="h-3.5 w-3.5" />;
}

export function SettingsDebugPanel({ ocrEnabled }: { ocrEnabled: boolean }) {
  const { showToast } = useToast();
  const relativeTimeNow = useMinuteTick();
  const [snapshot, setSnapshot] = useState<IntelligenceSchedulerSnapshot>(EMPTY_SNAPSHOT);
  const [ocrStatus, setOcrStatus] = useState<OcrBackfillStatus>(EMPTY_OCR_STATUS);
  const [error, setError] = useState('');

  const runDemo = (scenario: 'fifo' | 'parallel' | 'cancel' | 'fallback', label: string) => {
    invoke('run_intelligence_scheduler_demo', { scenario })
      .then(() => {
        showToast({ tone: 'success', message: `Started: ${label}` });
        setError('');
      })
      .catch((reason) => setError(String(reason)));
  };

  useEffect(() => {
    let cancelled = false;
    const refresh = () => {
      invoke<IntelligenceSchedulerSnapshot>('get_intelligence_scheduler_snapshot')
        .then((next) => {
          if (!cancelled) {
            setSnapshot(next);
            setError('');
          }
        })
        .catch((reason) => {
          if (!cancelled) setError(String(reason));
        });
      invoke<OcrBackfillStatus>('get_ocr_backfill_status')
        .then((next) => {
          if (!cancelled) setOcrStatus(next);
        })
        .catch((reason) => {
          if (!cancelled) setError(String(reason));
        });
    };
    refresh();
    const timer = window.setInterval(refresh, 400);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  return (
    <div className="space-y-5">
      <SettingsPanelHeader
        icon={Activity}
        title="Diagnostics"
        description="Watch background work and review diagnostics."
      />

      <section className="theme-surface rounded-2xl border p-5 space-y-4">
        <SettingsSubsectionHeader
          icon={<ScanText className="h-4 w-4" />}
          title="OCR"
          description={
            <>
              {ocrStatus.eligibleCount > 0
                ? `${ocrStatus.eligibleCount} image${ocrStatus.eligibleCount === 1 ? '' : 's'} haven't been scanned.`
                : 'All eligible images have been scanned.'}
            </>
          }
          actions={(ocrStatus.runningCount > 0 || ocrStatus.queuedCount > 0) ? (
            <ActionButton onClick={() => void invoke('cancel_ocr_backfill')}>
              <Square className="h-3.5 w-3.5" /> Cancel
            </ActionButton>
          ) : (
            <ActionButton variant="primary" disabled={!ocrEnabled || ocrStatus.eligibleCount === 0} onClick={() => void invoke('start_ocr_backfill')}>
              <ScanText className="h-3.5 w-3.5" /> Scan Existing
            </ActionButton>
          )}
        />
        <div className="grid grid-cols-3 gap-2 sm:grid-cols-6">
          {[
            ['Images', ocrStatus.totalImages],
            ['Waiting', ocrStatus.eligibleCount],
            ['Running', ocrStatus.runningCount],
            ['Complete', ocrStatus.completedCount],
            ['No text', ocrStatus.noTextCount],
            ['Failed', ocrStatus.failedCount],
          ].map(([label, value]) => (
            <div key={label} className="theme-card-idle border px-2 py-2 text-center">
              <strong className="theme-title block text-sm tabular-nums">{value}</strong>
              <span className="theme-text-muted text-[9px]">{label}</span>
            </div>
          ))}
        </div>
        {!ocrEnabled && <p className="theme-status-info rounded-lg border px-3 py-2 text-[10px]">Enable OCR in Features to scan images.</p>}
        {ocrEnabled && ocrStatus.failedCount > 0 && (
          <ActionButton onClick={() => void invoke('retry_failed_ocr')} className="w-full">
            Retry {ocrStatus.failedCount} Failed Scan{ocrStatus.failedCount === 1 ? '' : 's'}
          </ActionButton>
        )}
      </section>

      <SettingsSubsectionHeader title="Intelligence scheduler" />
      <section className="theme-surface rounded-2xl border p-5">
        <div className="grid grid-cols-2 gap-3">
          <div className="theme-card-idle border p-3">
            <div className="theme-text-muted text-[10px] font-semibold uppercase tracking-wider">Running</div>
            <div className="theme-title mt-1 text-xl font-bold tabular-nums">{snapshot.activeCount}</div>
          </div>
          <div className="theme-card-idle border p-3">
            <div className="theme-text-muted text-[10px] font-semibold uppercase tracking-wider">Queued</div>
            <div className="theme-title mt-1 text-xl font-bold tabular-nums">{snapshot.queuedCount}</div>
          </div>
        </div>
      </section>

      {import.meta.env.DEV && (
        <section className="theme-surface rounded-2xl border p-5 space-y-3">
          <SettingsSubsectionHeader
            title="Test scheduler"
            description="Safe simulations use the real scheduler without contacting providers, consuming tokens, or changing clips."
          />
          <div className="grid gap-2 sm:grid-cols-2">
            <button type="button" onClick={() => runDemo('fifo', 'Same provider ×3')} className="theme-secondary-button flex items-center gap-2 rounded-xl border px-3 py-2.5 text-left text-xs font-semibold">
              <Layers3 className="h-4 w-4 shrink-0" />
              <span><strong className="block">Same provider ×3</strong><span className="theme-text-muted text-[10px] font-normal">One running, two queued</span></span>
            </button>
            <button type="button" onClick={() => runDemo('parallel', 'Two providers')} className="theme-secondary-button flex items-center gap-2 rounded-xl border px-3 py-2.5 text-left text-xs font-semibold">
              <GitFork className="h-4 w-4 shrink-0" />
              <span><strong className="block">Two providers</strong><span className="theme-text-muted text-[10px] font-normal">Independent lanes run together</span></span>
            </button>
            <button type="button" onClick={() => runDemo('cancel', 'Cancel queued')} className="theme-secondary-button flex items-center gap-2 rounded-xl border px-3 py-2.5 text-left text-xs font-semibold">
              <Ban className="h-4 w-4 shrink-0" />
              <span><strong className="block">Cancel queued</strong><span className="theme-text-muted text-[10px] font-normal">Remove work before it starts</span></span>
            </button>
            <button type="button" onClick={() => runDemo('fallback', 'Provider fallback')} className="theme-secondary-button flex items-center gap-2 rounded-xl border px-3 py-2.5 text-left text-xs font-semibold">
              <Shuffle className="h-4 w-4 shrink-0" />
              <span><strong className="block">Provider fallback</strong><span className="theme-text-muted text-[10px] font-normal">Fail primary, finish on fallback</span></span>
            </button>
          </div>
        </section>
      )}

      {snapshot.jobs.length > 0 && (
        <section className="space-y-2">
          <h3 className="theme-text-muted text-[10px] font-semibold uppercase tracking-wider">Current work</h3>
          {snapshot.jobs.map((job) => (
            <article key={job.id} className="theme-card-idle border px-3 py-2.5">
              <div className="flex items-center gap-2">
                {job.status === 'running'
                  ? <LoaderCircle className="theme-status-info-text h-3.5 w-3.5 animate-spin" />
                  : <Clock3 className="theme-text-muted h-3.5 w-3.5" />}
                <OverflowText text={job.label} className="theme-text-main min-w-0 flex-1 truncate text-xs font-semibold" />
                <span className="theme-badge rounded-full border px-2 py-0.5 text-[9px] font-semibold">{job.status}</span>
              </div>
              <div className="theme-text-muted mt-1 flex flex-wrap gap-x-3 gap-y-1 pl-5.5 text-[10px]">
                <span>{job.connectionName}</span>
                <span>wait {duration(job.waitMs)}</span>
                {job.status === 'running' && <span>run {duration(job.runMs)}</span>}
              </div>
            </article>
          ))}
        </section>
      )}

      <section className="space-y-2">
        <h3 className="theme-text-muted text-[10px] font-semibold uppercase tracking-wider">Recent scheduler events</h3>
        <div className="theme-surface overflow-hidden rounded-2xl border">
          {snapshot.recentEvents.length ? snapshot.recentEvents.map((event) => (
            <div key={event.sequence} className="theme-surface border-b px-3 py-2.5 last:border-b-0">
              <div className="flex items-center gap-2">
                <span className={event.status === 'failed' ? 'theme-danger-text' : event.status === 'succeeded' ? 'theme-status-success-text' : 'theme-text-muted'}>
                  {eventIcon(event)}
                </span>
                <OverflowText text={event.label} className="theme-text-main min-w-0 flex-1 truncate text-xs font-semibold" />
                <time
                  className="theme-text-muted text-[9px] tabular-nums"
                  dateTime={dateTimeAttribute(event.timestampMs)}
                  title={formatFullDateTime(event.timestampMs)}
                >
                  {formatRelativeTime(event.timestampMs, relativeTimeNow)}
                </time>
              </div>
              <div className="theme-text-muted mt-1 pl-5.5 text-[10px]">
                {event.connectionName} · {event.status}{event.detail ? ` · ${event.detail}` : ''}
              </div>
            </div>
          )) : (
            <div className="theme-text-muted p-5 text-center text-xs">No scheduler activity yet.</div>
          )}
        </div>
      </section>

      {error && <div className="theme-status-danger rounded-xl border px-3 py-2 text-xs">{error}</div>}
    </div>
  );
}
