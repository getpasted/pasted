export function parseDbDate(timeStr: string): Date {
  if (!timeStr) return new Date();
  let isoStr = timeStr.trim();
  // Convert SQLite space separator "YYYY-MM-DD HH:MM:SS" to ISO "YYYY-MM-DDTHH:MM:SS"
  if (isoStr.includes(' ') && !isoStr.includes('T')) {
    isoStr = isoStr.replace(' ', 'T');
  }
  // Ensure trailing Z for UTC interpretation if timezone offset is omitted
  if (!isoStr.endsWith('Z') && !isoStr.includes('+') && !isoStr.includes('-')) {
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
