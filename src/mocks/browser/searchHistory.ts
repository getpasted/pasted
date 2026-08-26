import type { ClipSearchRequest, SearchHistoryEntry } from '../../types';
import { handled, unhandled, type BrowserMockResult } from './result';

let nextId = 1;
let entries: SearchHistoryEntry[] = [];

export function resetMockSearchHistory() {
  entries = [];
}

export function handleSearchHistoryBrowserMock(
  command: string,
  args?: Record<string, unknown>,
): BrowserMockResult {
  if (command === 'record_search_history') {
    const incoming = (args?.request ?? { query: '' }) as ClipSearchRequest;
    const { limit: _limit, offset: _offset, ...request } = incoming;
    const key = JSON.stringify(request);
    const existing = entries.find((entry) => JSON.stringify(entry.request) === key);
    if (existing) {
      existing.resultCount = Number(args?.resultCount ?? 0);
      existing.useCount += 1;
      existing.lastUsedAt = new Date().toISOString();
      entries = [existing, ...entries.filter((entry) => entry.id !== existing.id)];
    } else {
      entries.unshift({
        id: nextId++, request, resultCount: Number(args?.resultCount ?? 0),
        useCount: 1, lastUsedAt: new Date().toISOString(),
      });
    }
    return handled(undefined);
  }
  if (command === 'list_search_history') {
    const offset = Math.max(0, Number(args?.offset ?? 0));
    const limit = Math.min(500, Math.max(1, Number(args?.limit ?? 50)));
    return handled({ items: entries.slice(offset, offset + limit), totalCount: entries.length, limit, offset });
  }
  if (command === 'delete_search_history') {
    entries = entries.filter((entry) => entry.id !== Number(args?.id));
    return handled(undefined);
  }
  if (command === 'clear_search_history') {
    resetMockSearchHistory();
    return handled(undefined);
  }
  return unhandled;
}
