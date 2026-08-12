export function parseDbDate(timeStr: string): Date {
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

export function formatClipTime(timeStr: string): string {
  try {
    const d = parseDbDate(timeStr);
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  } catch {
    return timeStr;
  }
}

export function formatClipRelativeTime(timeStr: string, nowMs = Date.now()): string {
  const date = parseDbDate(timeStr);
  const timestampMs = date.getTime();
  if (Number.isNaN(timestampMs)) return timeStr;

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

export function formatClipDateTime(timeStr: string): string {
  try {
    const d = parseDbDate(timeStr);
    return d.toLocaleString([], {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return timeStr;
  }
}

export function formatClipFullDateTime(timeStr: string): string {
  try {
    const d = parseDbDate(timeStr);
    return d.toLocaleString([], {
      weekday: 'long',
      year: 'numeric',
      month: 'long',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return timeStr;
  }
}

export function clipDateTimeAttribute(timeStr: string): string {
  const date = parseDbDate(timeStr);
  return Number.isNaN(date.getTime()) ? timeStr : date.toISOString();
}
