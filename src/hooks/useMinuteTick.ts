import { useSyncExternalStore } from 'react';

const listeners = new Set<() => void>();
let intervalId: number | null = null;
let snapshot = Date.now();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);

  if (intervalId === null) {
    snapshot = Date.now();
    intervalId = window.setInterval(() => {
      snapshot = Date.now();
      listeners.forEach((notify) => notify());
    }, 30_000);
  }

  return () => {
    listeners.delete(listener);
    if (listeners.size === 0 && intervalId !== null) {
      window.clearInterval(intervalId);
      intervalId = null;
    }
  };
}

function getSnapshot(): number {
  return snapshot;
}

export function useMinuteTick(): number {
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}
