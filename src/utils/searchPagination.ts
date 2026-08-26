export function appendUniqueSearchPage<T extends { id: number }>(
  current: T[],
  page: T[],
): T[] {
  const existing = new Set(current.map((item) => item.id));
  return [...current, ...page.filter((item) => !existing.has(item.id))];
}

export function resolveSearchDisplayItems<T>(
  normalizedQuery: string,
  resultQuery: string,
  resultItems: T[],
): T[] {
  return normalizedQuery && resultQuery ? resultItems : [];
}
