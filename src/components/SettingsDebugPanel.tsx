import { useEffect, useState } from 'react';
import { Activity, Ban, CheckCircle2, CirclePlay, Clock3, GitFork, Layers3, LoaderCircle, Shuffle, XCircle } from 'lucide-react';
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
  if (event.status === 'running') return <CirclePlay className="h-3.5 w-3.5" />;
  if (event.status === 'succeeded') return <CheckCircle2 className="h-3.5 w-3.5" />;
  if (event.status === 'failed' || event.status === 'cancelled') return <XCircle className="h-3.5 w-3.5" />;
  return <Clock3 className="h-3.5 w-3.5" />;
}

export function SettingsDebugPanel() {
  const [snapshot, setSnapshot] = useState<IntelligenceSchedulerSnapshot>(EMPTY_SNAPSHOT);
  const [error, setError] = useState('');
  const [startedScenario, setStartedScenario] = useState('');

  const runDemo = (scenario: 'fifo' | 'parallel' | 'cancel' | 'fallback', label: string) => {
    invoke('run_intelligence_scheduler_demo', { scenario })
      .then(() => {
        setStartedScenario(label);
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

      {import.meta.env.DEV && (
        <section className="theme-surface rounded-2xl border p-5 space-y-3">
          <div>
            <h3 className="theme-title text-sm font-bold">Test scheduler</h3>
            <p className="theme-text-muted mt-1 text-xs leading-relaxed">
              Safe simulations use the real scheduler without contacting providers, consuming tokens, or changing clips.
            </p>
          </div>
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
          {startedScenario && <p className="theme-status-success rounded-lg border px-3 py-2 text-[10px]" role="status">Started: {startedScenario}</p>}
        </section>
      )}

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
            <div key={event.sequence} className="theme-surface border-b px-3 py-2.5 last:border-b-0">
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
