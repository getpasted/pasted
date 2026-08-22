import { invokeContentBrowserMock } from './contentRuntime';
import { invokeIntelligenceBrowserMock } from './intelligenceRuntime';
import { getMockClips, invokeLibraryBrowserMock } from './libraryRuntime';
import { unhandledValue } from './result';
import { invokeSystemBrowserMock } from './systemRuntime';

type BrowserHandler = <T>(
  command: string,
  args: Record<string, unknown> | undefined,
  clips: ReturnType<typeof getMockClips>,
) => Promise<T | typeof unhandledValue>;

const handlers: BrowserHandler[] = [
  invokeContentBrowserMock,
  invokeIntelligenceBrowserMock,
  invokeSystemBrowserMock,
  (command, args) => invokeLibraryBrowserMock(command, args),
];

export async function invokeBrowserMock<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  console.warn(`[safeInvoke mock] ${command}`);
  for (const handler of handlers) {
    const result = await handler<T>(command, args, getMockClips());
    if (result !== unhandledValue) return result;
  }
  throw new Error(`Unsupported browser IPC command: ${command}`);
}
