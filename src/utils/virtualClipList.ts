export interface VirtualClipPosition {
  index: number;
  start: number;
  size: number;
}

export interface VirtualClipLayout {
  positions: VirtualClipPosition[];
  totalSize: number;
}

export function estimatedClipCardHeight(rowHeight: 'small' | 'medium' | 'large'): number {
  if (rowHeight === 'small') return 76;
  if (rowHeight === 'large') return 196;
  return 112;
}

export function createVirtualClipLayout(
  itemIds: number[],
  measuredSizes: ReadonlyMap<number, number>,
  estimatedSize: number,
  gap: number,
): VirtualClipLayout {
  let start = 0;
  const positions = itemIds.map((id, index) => {
    const size = measuredSizes.get(id) ?? estimatedSize;
    const position = { index, start, size };
    start += size + gap;
    return position;
  });
  return {
    positions,
    totalSize: Math.max(0, start - (positions.length > 0 ? gap : 0)),
  };
}

function firstItemEndingAfter(positions: VirtualClipPosition[], offset: number): number {
  let low = 0;
  let high = positions.length;
  while (low < high) {
    const middle = Math.floor((low + high) / 2);
    const item = positions[middle];
    if (item.start + item.size < offset) low = middle + 1;
    else high = middle;
  }
  return low;
}

export function virtualClipIndexes(
  layout: VirtualClipLayout,
  scrollTop: number,
  viewportHeight: number,
  overscan: number,
  forcedIndexes: number[] = [],
): number[] {
  const { positions } = layout;
  if (positions.length === 0) return [];
  const boundedScrollTop = Math.min(Math.max(0, scrollTop), Math.max(0, layout.totalSize - viewportHeight));
  const start = firstItemEndingAfter(positions, Math.max(0, boundedScrollTop - overscan));
  const endOffset = boundedScrollTop + viewportHeight + overscan;
  let end = start;
  while (end < positions.length && positions[end].start <= endOffset) end += 1;
  const indexes = new Set<number>();
  for (let index = start; index < end; index += 1) indexes.add(index);
  for (const index of forcedIndexes) {
    if (index >= 0 && index < positions.length) indexes.add(index);
  }
  return Array.from(indexes).sort((left, right) => left - right);
}
