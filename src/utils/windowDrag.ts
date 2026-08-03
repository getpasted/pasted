import type { MouseEvent as ReactMouseEvent } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';

const INTERACTIVE_SELECTOR = 'button, a, input, select, textarea, [role="button"], [contenteditable="true"]';

/**
 * Starts a native window drag only on a deliberate primary-button press over
 * non-interactive title-bar space. This avoids WebKit's continuously active
 * drag-region hit testing, which can fight the normal pointer cursor.
 */
export function startWindowDrag(event: ReactMouseEvent<HTMLElement>) {
  if (event.button !== 0) return;
  const target = event.target as HTMLElement | null;
  if (target?.closest(INTERACTIVE_SELECTOR)) return;
  if (typeof window === 'undefined' || !(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) return;

  event.preventDefault();
  getCurrentWindow().startDragging().catch((error) => {
    console.error('Failed to start window drag:', error);
  });
}
