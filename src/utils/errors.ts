export function errorMessage(reason: unknown): string {
  if (reason instanceof Error && reason.message.trim()) return reason.message;
  if (typeof reason === 'string') return reason;
  if (reason && typeof reason === 'object' && 'message' in reason) {
    const message = reason.message;
    if (typeof message === 'string') return message;
    if (message !== undefined && message !== null) return String(message);
  }
  try {
    const serialized = JSON.stringify(reason);
    if (serialized !== undefined) return serialized;
  } catch {
    // Fall through for cyclic or otherwise unserializable values.
  }
  return String(reason);
}
