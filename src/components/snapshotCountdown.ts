export function snapshotCountdownParts(target: string | null, nowMs: number) {
  if (!target) return null;
  const targetMs = Date.parse(target);
  if (Number.isNaN(targetMs)) return null;
  const totalSeconds = Math.max(0, Math.ceil((targetMs - nowMs) / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  return { hours, minutes, seconds };
}
