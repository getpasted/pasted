import {
  Children,
  Fragment,
  cloneElement,
  isValidElement,
  type ElementType,
  type ReactNode,
} from 'react';

function flattenMenuChildren(children: ReactNode, result: ReactNode[]) {
  Children.forEach(children, (child) => {
    if (isValidElement<{ children?: ReactNode }>(child) && child.type === Fragment) {
      flattenMenuChildren(child.props.children, result);
    } else if (child !== null && child !== undefined && typeof child !== 'boolean') {
      result.push(child);
    }
  });
}

export function normalizeMenuDividers(children: ReactNode, dividerType: ElementType): ReactNode[] {
  const flattened: ReactNode[] = [];
  flattenMenuChildren(children, flattened);
  const normalized: ReactNode[] = [];

  for (const child of flattened) {
    const isDivider = isValidElement(child) && child.type === dividerType;
    const previous = normalized[normalized.length - 1];
    if (isDivider && (previous === undefined
      || (isValidElement(previous) && previous.type === dividerType))) continue;
    normalized.push(child);
  }
  const finalChild = normalized[normalized.length - 1];
  if (isValidElement(finalChild) && finalChild.type === dividerType) normalized.pop();
  return normalized.map((child, index) => isValidElement(child) && child.key === null
    ? cloneElement(child, { key: `menu-child:${index}` })
    : child);
}
