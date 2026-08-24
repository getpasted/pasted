import { invoke as tauriInvoke } from '@tauri-apps/api/core';

export async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (typeof window !== 'undefined' && (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) {
    return tauriInvoke<T>(cmd, args);
  }
  const { invokeBrowserMock } = await import('../mocks/browser/runtime');
  return invokeBrowserMock<T>(cmd, args);
}
