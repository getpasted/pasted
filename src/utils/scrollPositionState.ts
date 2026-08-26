export const SCROLL_POSITION_STATE_KEY = 'pasted_scroll_positions';

export interface PersistedScrollPosition {
  scrollTop: number;
  anchorClipId?: number | null;
  anchorOffset?: number;
}

interface PersistedScrollState {
  version: 1;
  positions: Record<string, PersistedScrollPosition>;
}

const MAX_POSITIONS = 96;
const MAX_SCROLL_TOP = 100_000_000;
const pendingPositions = new Map<string, PersistedScrollPosition>();
let persistenceTimer: ReturnType<typeof setTimeout> | undefined;

function normalizePosition(value: unknown): PersistedScrollPosition | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  if (typeof record.scrollTop !== 'number' || !Number.isFinite(record.scrollTop)) return null;
  const position: PersistedScrollPosition = {
    scrollTop: Math.min(Math.max(0, record.scrollTop), MAX_SCROLL_TOP),
  };
  if (record.anchorClipId === null) position.anchorClipId = null;
  else if (typeof record.anchorClipId === 'number' && Number.isSafeInteger(record.anchorClipId)) {
    position.anchorClipId = record.anchorClipId;
  }
  if (typeof record.anchorOffset === 'number' && Number.isFinite(record.anchorOffset)) {
    position.anchorOffset = Math.min(Math.max(record.anchorOffset, -10_000), 10_000);
  }
  return position;
}

export function parseScrollPositionState(value: unknown): PersistedScrollState {
  const empty: PersistedScrollState = { version: 1, positions: {} };
  if (!value || typeof value !== 'object' || Array.isArray(value)) return empty;
  const record = value as Record<string, unknown>;
  if (record.version !== 1 || !record.positions || typeof record.positions !== 'object' || Array.isArray(record.positions)) return empty;
  const positions: Record<string, PersistedScrollPosition> = {};
  for (const [key, candidate] of Object.entries(record.positions).slice(-MAX_POSITIONS)) {
    if (!key || key.length > 768) continue;
    const position = normalizePosition(candidate);
    if (position) positions[key] = position;
  }
  return { version: 1, positions };
}

function readState(): PersistedScrollState {
  try {
    const saved = localStorage.getItem(SCROLL_POSITION_STATE_KEY);
    return saved ? parseScrollPositionState(JSON.parse(saved)) : { version: 1, positions: {} };
  } catch {
    return { version: 1, positions: {} };
  }
}

export function readPersistedScrollPosition(key: string): PersistedScrollPosition {
  return pendingPositions.get(key) ?? readState().positions[key] ?? { scrollTop: 0 };
}

export function flushPendingScrollPositionPersistence() {
  if (persistenceTimer) clearTimeout(persistenceTimer);
  persistenceTimer = undefined;
  if (pendingPositions.size === 0) return;
  try {
    const state = readState();
    for (const [key, position] of pendingPositions) {
      delete state.positions[key];
      state.positions[key] = position;
    }
    const positions = Object.fromEntries(Object.entries(state.positions).slice(-MAX_POSITIONS));
    localStorage.setItem(SCROLL_POSITION_STATE_KEY, JSON.stringify({ version: 1, positions }));
    pendingPositions.clear();
  } catch {
    // Scroll continuity is best-effort and never blocks library work.
  }
}

export function scheduleScrollPositionPersistence(key: string, position: PersistedScrollPosition) {
  const normalized = normalizePosition(position);
  if (!key || key.length > 768 || !normalized) return;
  pendingPositions.set(key, normalized);
  if (persistenceTimer) clearTimeout(persistenceTimer);
  persistenceTimer = setTimeout(flushPendingScrollPositionPersistence, 120);
}

export function discardPendingScrollPositionPersistence() {
  if (persistenceTimer) clearTimeout(persistenceTimer);
  persistenceTimer = undefined;
  pendingPositions.clear();
}
