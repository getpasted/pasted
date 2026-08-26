export function sortFacetItemsByPopularity<T extends { count: number; label: string }>(items: T[]) {
  return items.sort((left, right) => right.count - left.count || left.label.localeCompare(right.label));
}
