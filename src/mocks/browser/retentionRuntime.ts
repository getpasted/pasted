import { unhandledValue } from './result';

export async function invokeRetentionBrowserMock<T>(
  command: string,
): Promise<T | typeof unhandledValue> {
  switch (command) {
    case 'enforce_activity_retention':
    case 'enforce_analysis_attempt_retention':
    case 'enforce_clip_retention':
    case 'enforce_revision_retention':
    case 'enforce_search_history_retention':
    case 'enforce_trash_retention':
      return undefined as T;
    default:
      return unhandledValue;
  }
}
