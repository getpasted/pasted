export const APP_ZOOM_STEPS = [14, 16, 18, 20] as const;
export const ACTUAL_SIZE = 16;

export function clampAppZoom(size: number) {
  return APP_ZOOM_STEPS.reduce((closest, candidate) => (
    Math.abs(candidate - size) < Math.abs(closest - size) ? candidate : closest
  ), ACTUAL_SIZE as number);
}

export function stepAppZoom(size: number, direction: -1 | 1) {
  const current = clampAppZoom(size);
  const index = APP_ZOOM_STEPS.indexOf(current as (typeof APP_ZOOM_STEPS)[number]);
  const nextIndex = Math.max(0, Math.min(APP_ZOOM_STEPS.length - 1, index + direction));
  return APP_ZOOM_STEPS[nextIndex];
}

export function appZoomPercent(size: number) {
  return Math.round((clampAppZoom(size) / ACTUAL_SIZE) * 100);
}
