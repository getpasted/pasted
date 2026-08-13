export function parseDbDate(timeStr: string | number): Date {
  if (typeof timeStr === 'number') return new Date(timeStr);
  if (!timeStr) return new Date();
  let isoStr = timeStr.trim();
  // Convert SQLite space separator "YYYY-MM-DD HH:MM:SS" to ISO "YYYY-MM-DDTHH:MM:SS"
  if (isoStr.includes(' ') && !isoStr.includes('T')) {
    isoStr = isoStr.replace(' ', 'T');
  }
  // Ensure trailing Z for UTC interpretation if timezone offset is omitted
  const hasExplicitTimezone = /(?:Z|[+-]\d{2}:?\d{2})$/i.test(isoStr);
  if (!hasExplicitTimezone) {
    isoStr += 'Z';
  }
  const d = new Date(isoStr);
  return isNaN(d.getTime()) ? new Date(timeStr) : d;
}

export function formatRelativeTime(timeStr: string | number, nowMs = Date.now()): string {
  const date = parseDbDate(timeStr);
  const timestampMs = date.getTime();
  if (Number.isNaN(timestampMs)) return String(timeStr);

  const elapsedSeconds = Math.max(0, Math.floor((nowMs - timestampMs) / 1000));
  if (elapsedSeconds < 60) return 'now';

  const elapsedMinutes = Math.floor(elapsedSeconds / 60);
  if (elapsedMinutes < 60) return `${elapsedMinutes}m ago`;

  const elapsedHours = Math.floor(elapsedMinutes / 60);
  if (elapsedHours < 24) return `${elapsedHours}h ago`;

  const elapsedDays = Math.floor(elapsedHours / 24);
  if (elapsedDays < 7) return `${elapsedDays}d ago`;
  if (elapsedDays < 30) return `${Math.floor(elapsedDays / 7)}w ago`;
  if (elapsedDays < 365) return `${Math.floor(elapsedDays / 30)}mo ago`;
  return `${Math.floor(elapsedDays / 365)}y ago`;
}

export function formatFullDateTime(timeStr: string | number): string {
  try {
    const d = parseDbDate(timeStr);
    return d.toLocaleString([], {
      weekday: 'long',
      year: 'numeric',
      month: 'long',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      timeZoneName: 'short',
    });
  } catch {
    return String(timeStr);
  }
}

export function dateTimeAttribute(timeStr: string | number): string {
  const date = parseDbDate(timeStr);
  return Number.isNaN(date.getTime()) ? String(timeStr) : date.toISOString();
}
