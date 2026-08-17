import { createElement, type FocusEvent, type HTMLAttributes, type MouseEvent } from 'react';

type OverflowTextElement = 'div' | 'h2' | 'h3' | 'h4' | 'p' | 'span' | 'strong';

interface OverflowTextProps extends Omit<HTMLAttributes<HTMLElement>, 'children' | 'title'> {
  as?: OverflowTextElement;
  text: string;
}

/**
 * Exposes clipped text without adding a permanent tooltip to values that fit.
 * Measuring on interaction keeps long virtualized lists free of ResizeObservers.
 */
export function OverflowText({ as = 'span', text, onMouseEnter, onFocus, ...props }: OverflowTextProps) {
  const updateTitle = (element: HTMLElement) => {
    const isClipped = element.scrollWidth > element.clientWidth + 1
      || element.scrollHeight > element.clientHeight + 1;
    if (isClipped) element.setAttribute('title', text);
    else element.removeAttribute('title');
  };

  const handleMouseEnter = (event: MouseEvent<HTMLElement>) => {
    updateTitle(event.currentTarget);
    onMouseEnter?.(event);
  };

  const handleFocus = (event: FocusEvent<HTMLElement>) => {
    updateTitle(event.currentTarget);
    onFocus?.(event);
  };

  return createElement(as, { ...props, dir: props.dir ?? 'auto', onMouseEnter: handleMouseEnter, onFocus: handleFocus }, text);
}
