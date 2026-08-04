import { useEffect, useState } from 'react';
import type { IntelligenceSchedulerSnapshot } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';

export type IntelligenceRequestPhase = 'starting' | 'queued' | 'running';

export interface IntelligenceRequestStatus {
  phase: IntelligenceRequestPhase;
  connectionName: string | null;
  didFallback: boolean;
}

const INITIAL_STATUS: IntelligenceRequestStatus = {
  phase: 'starting',
  connectionName: null,
  didFallback: false,
};

export function useIntelligenceRequestStatus(clientRequestId: string | null) {
  const [status, setStatus] = useState<IntelligenceRequestStatus>(INITIAL_STATUS);

  useEffect(() => {
    if (!clientRequestId) {
      setStatus(INITIAL_STATUS);
      return;
    }

    let cancelled = false;
    const refresh = () => {
      invoke<IntelligenceSchedulerSnapshot>('get_intelligence_scheduler_snapshot')
        .then((snapshot) => {
          if (cancelled) return;
          const job = snapshot.jobs.find((candidate) => candidate.clientRequestId === clientRequestId);
          const requestEvents = snapshot.recentEvents.filter((event) => event.clientRequestId === clientRequestId);
          const providers = new Set(requestEvents.map((event) => event.connectionName));
          setStatus((current) => ({
            phase: job?.status ?? current.phase,
            connectionName: job?.connectionName ?? current.connectionName,
            didFallback: providers.size > 1,
          }));
        })
        .catch(() => {
          // The request itself owns user-facing errors. Scheduler diagnostics are supplemental.
        });
    };

    setStatus(INITIAL_STATUS);
    refresh();
    const timer = window.setInterval(refresh, 200);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [clientRequestId]);

  return status;
}
