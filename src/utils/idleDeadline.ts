export const APP_LOCK_ACTIVITY_EVENTS: ReadonlyArray<keyof WindowEventMap> = [
  'pointerdown',
  'pointermove',
  'keydown',
  'wheel',
  'touchstart',
  'resize',
];

interface IdleDeadlineOptions {
  delayMs: number;
  onElapsed: () => void;
  now?: () => number;
  schedule?: (callback: () => void, delayMs: number) => number;
  cancel?: (timer: number) => void;
}

export function createIdleDeadline({
  delayMs,
  onElapsed,
  now = () => performance.now(),
  schedule = (callback, delay) => window.setTimeout(callback, delay),
  cancel = (timer) => window.clearTimeout(timer),
}: IdleDeadlineOptions) {
  let deadline = now() + delayMs;
  let timer: number | undefined;
  let disposed = false;

  const checkDeadline = () => {
    timer = undefined;
    if (disposed) return;

    const remaining = deadline - now();
    if (remaining > 0) {
      timer = schedule(checkDeadline, remaining);
      return;
    }

    disposed = true;
    onElapsed();
  };

  timer = schedule(checkDeadline, delayMs);

  return {
    markActivity() {
      if (!disposed) deadline = now() + delayMs;
    },
    dispose() {
      disposed = true;
      if (timer !== undefined) cancel(timer);
      timer = undefined;
    },
  };
}
