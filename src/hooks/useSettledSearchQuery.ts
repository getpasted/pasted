import { useDeferredValue, useEffect, useState } from 'react';

export function useSettledSearchQuery(query: string, active: boolean, delayMs = 1000) {
  const deferredQuery = useDeferredValue(query);
  const [settledQuery, setSettledQuery] = useState('');
  useEffect(() => {
    if (!active || !deferredQuery.trim()) {
      setSettledQuery('');
      return undefined;
    }
    const timer = window.setTimeout(() => setSettledQuery(deferredQuery), delayMs);
    return () => window.clearTimeout(timer);
  }, [active, deferredQuery, delayMs]);
  return settledQuery;
}
