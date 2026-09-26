interface DismissalItem {
  id: number;
  exiting?: boolean;
  clip: { isPinned: boolean } | null;
}

type Phase = 'visible' | 'fading' | 'collapsing';
interface DismissalOptions {
  items: () => DismissalItem[];
  delaySeconds: () => number;
  setPhase: (id: number, phase: Phase) => void;
  finish: (id: number) => void;
  setTimeout: (callback: () => void, delay: number) => number;
  clearTimeout: (timer: number) => void;
}

// Interaction holds the entire stack steady, including notices arriving while
// the pointer is stationary. DOM hover, native hover, and focus are independent.
export function createCaptureFeedbackDismissal(options: DismissalOptions) {
  const timers = new Map<number, number>();
  const phases = new Map<number, Phase>();
  const interactions = new Set<string>();

  const pause = (id: number) => {
    const timer = timers.get(id);
    if (timer !== undefined) options.clearTimeout(timer);
    timers.delete(id);
    if (phases.has(id)) options.setPhase(id, 'visible');
    phases.delete(id);
  };
  const schedule = (id: number) => {
    pause(id);
    const item = options.items().find((candidate) => candidate.id === id);
    if (!item || item.exiting || interactions.size > 0) return;
    if (item.clip?.isPinned) return;
    const delay = item.clip ? options.delaySeconds() * 1_000 : 1_800;
    if (delay <= 0) return;
    const after = (milliseconds: number, callback: () => void) => {
      timers.set(id, options.setTimeout(() => {
        timers.delete(id);
        if (interactions.size > 0) { pause(id); return; }
        callback();
      }, milliseconds));
    };
    after(delay, () => {
      phases.set(id, 'fading');
      options.setPhase(id, 'fading');
      after(1_000, () => {
        phases.set(id, 'collapsing');
        options.setPhase(id, 'collapsing');
        after(160, () => {
          phases.delete(id);
          options.finish(id);
        });
      });
    });
  };
  const setInteraction = (source: string, active: boolean) => {
    const wasActive = interactions.size > 0;
    if (active) interactions.add(source);
    else interactions.delete(source);
    if (active) {
      options.items().forEach(({ id }) => pause(id));
    } else if (wasActive && interactions.size === 0) {
      options.items().forEach(({ id }) => schedule(id));
    }
  };
  const clear = () => {
    timers.forEach(options.clearTimeout);
    timers.clear();
    phases.clear();
    interactions.clear();
  };
  return { schedule, pause, setInteraction, clear };
}
