const COMPLETION_PHASE = 0.68;
const HOLD_END_PHASE = 0.78;
const SETTLE_DURATION_MS = 140;
const FADE_DURATION_MS = 320;

export function dismissStartupSplash(splash: HTMLElement): () => void {
  let frame = 0;
  let alignmentTimer = 0;
  let settleTimer = 0;
  let fadeAnimation: Animation | undefined;
  let cancelled = false;

  const fade = () => {
    splash.classList.add('is-ready');
    fadeAnimation = splash.animate(
      [{ opacity: 1 }, { opacity: 0 }],
      {
        duration: FADE_DURATION_MS,
        easing: 'cubic-bezier(0.22, 1, 0.36, 1)',
        fill: 'forwards',
      },
    );
    void fadeAnimation.finished.then(() => {
      if (!cancelled) splash.remove();
    }).catch(() => undefined);
  };

  const settle = () => {
    splash.classList.add('is-dismiss-ready');
    frame = window.requestAnimationFrame(() => {
      settleTimer = window.setTimeout(() => {
        frame = window.requestAnimationFrame(fade);
      }, SETTLE_DURATION_MS);
    });
  };

  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    frame = window.requestAnimationFrame(fade);
    return () => {
      cancelled = true;
      window.cancelAnimationFrame(frame);
      fadeAnimation?.cancel();
    };
  }

  const outline = splash.querySelector<SVGPathElement>('.startup-copycat-outline');
  const animation = outline?.getAnimations()[0];
  const duration = Number(animation?.effect?.getComputedTiming().duration);
  const currentTime = Number(animation?.currentTime);

  if (!animation || !Number.isFinite(duration) || duration <= 0 || !Number.isFinite(currentTime)) {
    frame = window.requestAnimationFrame(settle);
  } else {
    const phase = (currentTime % duration) / duration;
    const remaining = phase >= COMPLETION_PHASE && phase <= HOLD_END_PHASE
      ? 0
      : phase < COMPLETION_PHASE
        ? COMPLETION_PHASE - phase
        : 1 - phase + COMPLETION_PHASE;
    alignmentTimer = window.setTimeout(() => {
      frame = window.requestAnimationFrame(settle);
    }, remaining * duration);
  }

  return () => {
    cancelled = true;
    window.cancelAnimationFrame(frame);
    window.clearTimeout(alignmentTimer);
    window.clearTimeout(settleTimer);
    fadeAnimation?.cancel();
  };
}
