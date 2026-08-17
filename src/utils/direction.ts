export type InterfaceDirection = 'ltr' | 'rtl';

/** Converts a physical horizontal pointer delta into growth toward inline-end. */
export function inlineResizeDelta(startX: number, currentX: number, direction: InterfaceDirection): number {
  const physicalDelta = currentX - startX;
  return direction === 'rtl' ? -physicalDelta : physicalDelta;
}
