export function appendUniqueSearchPage<T extends { id: number }>(
  current: T[],
  page: T[],
): T[] {
  const existing = new Set(current.map((item) => item.id));
  return [...current, ...page.filter((item) => !existing.has(item.id))];
}
