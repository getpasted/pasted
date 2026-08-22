import { invoke as tauriInvoke } from '@tauri-apps/api/core';

import { invokeBrowserMock } from '../mocks/browser/runtime';

export async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (typeof window !== 'undefined' && (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) {
    return tauriInvoke<T>(cmd, args);
  }
  return invokeBrowserMock<T>(cmd, args);
}
