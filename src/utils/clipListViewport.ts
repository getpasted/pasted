export function clipCardScrollTop(element: HTMLElement, card: HTMLElement): number {
  const listPaddingTop = Number.parseFloat(window.getComputedStyle(element).paddingTop) || 0;
  const cardMarginTop = Number.parseFloat(window.getComputedStyle(card).marginTop) || 0;
  return Math.max(0, Math.min(
    element.scrollTop + card.getBoundingClientRect().top - element.getBoundingClientRect().top
      - listPaddingTop - cardMarginTop,
    element.scrollHeight - element.clientHeight,
  ));
}
