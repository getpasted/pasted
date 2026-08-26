export function clipCardScrollTop(element: HTMLElement, card: HTMLElement): number {
  const listPaddingTop = Number.parseFloat(window.getComputedStyle(element).paddingTop) || 0;
  const cardMarginTop = Number.parseFloat(window.getComputedStyle(card).marginTop) || 0;
  return Math.max(0, Math.min(
    element.scrollTop + card.getBoundingClientRect().top - element.getBoundingClientRect().top
      - listPaddingTop - cardMarginTop,
    element.scrollHeight - element.clientHeight,
  ));
}

export function orderClipsForStableReorder<T>(
  items: T[],
  orderedIds: string[] | null,
  idForItem: (item: T) => string,
): T[] {
  if (!orderedIds) return items;
  const itemById = new Map(items.map((item) => [idForItem(item), item]));
  const ordered = orderedIds.flatMap((id) => {
    const item = itemById.get(id);
    if (!item) return [];
    itemById.delete(id);
    return [item];
  });
  return [...ordered, ...itemById.values()];
}
