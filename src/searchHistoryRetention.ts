export const MAX_SEARCH_HISTORY_AGE_DAYS = 36_500;

export function storedSearchHistoryAgeDays(saved: Record<string, string>, fallback: number): number {
  const value = Number(saved.searchHistoryAgeDays);
  const normalized = Number.isFinite(value) ? value : fallback;
  return Math.max(0, Math.min(MAX_SEARCH_HISTORY_AGE_DAYS, normalized));
}
