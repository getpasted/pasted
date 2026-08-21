export interface ClipListScrollPosition {
  scrollTop: number;
  anchorClipId: number | null;
  anchorOffset: number;
}

const TOP_POSITION: ClipListScrollPosition = Object.freeze({
  scrollTop: 0,
  anchorClipId: null,
  anchorOffset: 0,
});

export class ClipListScrollMemory {
  private readonly positions = new Map<string, ClipListScrollPosition>();

  remember(viewKey: string, position: ClipListScrollPosition) {
    if (!Number.isFinite(position.scrollTop) || !Number.isFinite(position.anchorOffset)) return;
    this.positions.set(viewKey, { ...position, scrollTop: Math.max(0, position.scrollTop) });
  }

  recall(viewKey: string): ClipListScrollPosition {
    return this.positions.get(viewKey) ?? TOP_POSITION;
  }
}
