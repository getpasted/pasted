import { handled, unhandled, type BrowserMockResult } from './result';

export function handleClipVersionBrowserMock<T extends { id: number }>(
  command: string,
  args: Record<string, unknown> | undefined,
  clips: readonly T[],
): BrowserMockResult {
  if (command === 'get_clip_versions') return handled([]);
  if (command === 'get_clip_version_count') return handled(0);
  if (command === 'restore_clip_version') {
    const clip = clips.find((item) => item.id === Number(args?.clipId));
    return handled(clip ? { ...clip } : null);
  }
  if (command === 'delete_clip_version') return handled(null);
  return unhandled;
}
