import type { MouseEvent as ReactMouseEvent } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { safeInvoke as invoke } from './tauri';

const INTERACTIVE_SELECTOR = [
  'button',
  'a',
  'input',
  'select',
  'textarea',
  'label',
  'summary',
  '[role="button"]',
  '[role="link"]',
  '[role="menuitem"]',
  '[contenteditable="true"]',
  '.titlebar-no-drag',
].join(', ');

export function isInteractiveTitlebarTarget(target: EventTarget | null) {
  const candidate = target as { closest?: (selector: string) => Element | null } | null;
  return typeof candidate?.closest === 'function' && candidate.closest(INTERACTIVE_SELECTOR) !== null;
}

/**
 * Starts a native window drag only on a deliberate primary-button press over
 * non-interactive title-bar space. This avoids WebKit's continuously active
 * drag-region hit testing, which can fight the normal pointer cursor.
 */
export function startWindowDrag(event: ReactMouseEvent<HTMLElement>) {
  if (event.button !== 0) return;
  if (isInteractiveTitlebarTarget(event.target)) return;
  if (typeof window === 'undefined' || !(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) return;

  event.preventDefault();

  getCurrentWindow().startDragging().catch((error) => {
    console.error('Failed to start window drag:', error);
  });
}

/**
 * Applies the configured macOS title-bar action after WebKit has confirmed a
 * double-click. Handling this during the second mouse-down makes a click-drag
 * resize the window before macOS can recognize that the pointer is moving.
 */
export function handleWindowDragDoubleClick(event: ReactMouseEvent<HTMLElement>) {
  if (event.button !== 0) return;
  if (isInteractiveTitlebarTarget(event.target)) return;
  if (document.documentElement.dataset.platform !== 'macos') return;
  if (typeof window === 'undefined' || !(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) return;

  event.preventDefault();
  invoke<void>('perform_titlebar_double_click').catch((error) => {
    console.error('Failed to perform titlebar double-click action:', error);
  });
}
