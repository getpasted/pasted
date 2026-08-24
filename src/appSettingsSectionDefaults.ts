import type { AppSettings } from './types';
import { settingDefault } from './settingsContract.ts';

export const DEFAULT_NOTIFICATION_SETTINGS = {
  captureFeedback: settingDefault('captureFeedback'),
  captureFeedbackIgnored: settingDefault('captureFeedbackIgnored'),
  captureFeedbackPreview: settingDefault('captureFeedbackPreview'),
  captureFeedbackPosition: settingDefault('captureFeedbackPosition'),
  captureFeedbackDismissSeconds: settingDefault('captureFeedbackDismissSeconds'),
} as const satisfies Partial<AppSettings>;

export function notificationDefaultUpdates(): Partial<AppSettings> {
  return { ...DEFAULT_NOTIFICATION_SETTINGS };
}
