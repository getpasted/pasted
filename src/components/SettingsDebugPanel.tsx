import { useEffect, useState } from 'react';
import { Activity, CheckCircle2, Clock3, LoaderCircle, XCircle } from 'lucide-react';
import type { IntelligenceSchedulerEvent, IntelligenceSchedulerSnapshot } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';

const EMPTY_SNAPSHOT: IntelligenceSchedulerSnapshot = {
  revision: 0,
  activeCount: 0,
  queuedCount: 0,
  jobs: [],
  recentEvents: [],
};

function duration(milliseconds: number) {
  if (milliseconds < 1_000) return `${Math.round(milliseconds)} ms`;
  return `${(milliseconds / 1_000).toFixed(1)} s`;
}

function eventIcon(event: IntelligenceSchedulerEvent) {
  if (event.status === 'running') return <LoaderCircle className="h-3.5 w-3.5 animate-spin" />;
  if (event.status === 'succeeded') return <CheckCircle2 className="h-3.5 w-3.5" />;
  if (event.status === 'failed' || event.status === 'cancelled') return <XCircle className="h-3.5 w-3.5" />;
  return <Clock3 className="h-3.5 w-3.5" />;
}

export function SettingsDebugPanel() {
  const [snapshot, setSnapshot] = useState<IntelligenceSchedulerSnapshot>(EMPTY_SNAPSHOT);
  const [error, setError] = useState('');

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
      <section className="theme-surface rounded-2xl border p-5 space-y-4">
        <div className="flex items-start gap-3">
          <span className="theme-badge rounded-xl border p-2.5"><Activity className="h-5 w-5" /></span>
          <div className="min-w-0 flex-1">
            <h2 className="theme-title text-sm font-bold">Intelligence scheduler</h2>
            <p className="theme-text-muted mt-1 text-xs leading-relaxed">
              Live, in-memory diagnostics. Completed outcomes remain in Activity Log; transient queue events disappear when Pasted quits.
            </p>
          </div>
        </div>
        <div className="grid grid-cols-2 gap-3">
          <div className="theme-card-idle rounded-xl border p-3">
            <div className="theme-text-muted text-[10px] font-semibold uppercase tracking-wider">Running</div>
            <div className="theme-title mt-1 text-xl font-bold tabular-nums">{snapshot.activeCount}</div>
          </div>
          <div className="theme-card-idle rounded-xl border p-3">
            <div className="theme-text-muted text-[10px] font-semibold uppercase tracking-wider">Queued</div>
            <div className="theme-title mt-1 text-xl font-bold tabular-nums">{snapshot.queuedCount}</div>
          </div>
        </div>
      </section>

      {snapshot.jobs.length > 0 && (
        <section className="space-y-2">
          <h3 className="theme-text-muted text-[10px] font-semibold uppercase tracking-wider">Current work</h3>
          {snapshot.jobs.map((job) => (
            <article key={job.id} className="theme-card-idle rounded-xl border px-3 py-2.5">
              <div className="flex items-center gap-2">
                {job.status === 'running'
                  ? <LoaderCircle className="theme-status-info-text h-3.5 w-3.5 animate-spin" />
                  : <Clock3 className="theme-text-muted h-3.5 w-3.5" />}
                <span className="theme-text-main min-w-0 flex-1 truncate text-xs font-semibold">{job.label}</span>
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
            <div key={event.sequence} className="border-b px-3 py-2.5 last:border-b-0">
              <div className="flex items-center gap-2">
                <span className={event.status === 'failed' ? 'theme-danger-text' : event.status === 'succeeded' ? 'theme-status-success-text' : 'theme-text-muted'}>
                  {eventIcon(event)}
                </span>
                <span className="theme-text-main min-w-0 flex-1 truncate text-xs font-semibold">{event.label}</span>
                <span className="theme-text-muted text-[9px] tabular-nums">{new Date(event.timestampMs).toLocaleTimeString()}</span>
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
