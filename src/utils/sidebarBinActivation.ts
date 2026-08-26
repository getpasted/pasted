import type { KeyboardEvent } from 'react';

export function activateSidebarBin(
  id: number,
  activeDragBinId: number | null,
  consumeDragClick: () => boolean,
  onSelect: (id: number) => void,
) {
  if (!consumeDragClick() && activeDragBinId === null) onSelect(id);
}

export function activateSidebarBinFromKeyboard(
  event: KeyboardEvent<HTMLDivElement>,
  id: number,
  activeDragBinId: number | null,
  onSelect: (id: number) => void,
) {
  if (event.target !== event.currentTarget || !['Enter', ' '].includes(event.key)) return;
  event.preventDefault();
  if (activeDragBinId === null) onSelect(id);
}
