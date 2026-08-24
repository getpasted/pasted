import { notificationDefaultUpdates } from './appSettingsSectionDefaults';
import { translate } from './localization/runtime';
import type { AppSettings } from './types';
import { resetBooleanLabel, type SettingsResetChange } from './components/SettingsResetChanges';

export const POSITION_OPTIONS = [
  { value: 'top-left', get label() { return translate('component.settingsNotificationsPanel.topLeft'); } },
  { value: 'top-right', get label() { return translate('component.settingsNotificationsPanel.topRight'); } },
  { value: 'bottom-left', get label() { return translate('component.settingsNotificationsPanel.bottomLeft'); } },
  { value: 'bottom-right', get label() { return translate('component.settingsNotificationsPanel.bottomRight'); } },
];

export const DISMISS_OPTIONS = [3, 5, 7, 10, 15, 30].map((seconds) => ({
  value: String(seconds),
  get label() { return translate(`component.settingsNotificationsPanel.value${seconds}Seconds` as Parameters<typeof translate>[0]); },
})).concat([{ value: '0', get label() { return translate('component.settingsNotificationsPanel.never'); } }]);

export function notificationResetChanges(settings: AppSettings): {
  changes: SettingsResetChange[];
  defaults: Partial<AppSettings>;
} {
  const defaults = notificationDefaultUpdates();
  const changes = [
    { label: translate('component.settingsNotificationsPanel.captureFeedback'), before: resetBooleanLabel(settings.captureFeedback), after: resetBooleanLabel(defaults.captureFeedback!) },
    { label: translate('component.settingsNotificationsPanel.showSkippedCaptures'), before: resetBooleanLabel(settings.captureFeedbackIgnored), after: resetBooleanLabel(defaults.captureFeedbackIgnored!) },
    { label: translate('component.settingsNotificationsPanel.showClipPreview'), before: resetBooleanLabel(settings.captureFeedbackPreview), after: resetBooleanLabel(defaults.captureFeedbackPreview!) },
    { label: translate('component.settingsNotificationsPanel.screenPosition'), before: optionLabel(POSITION_OPTIONS, settings.captureFeedbackPosition), after: optionLabel(POSITION_OPTIONS, String(defaults.captureFeedbackPosition)) },
    { label: translate('component.settingsNotificationsPanel.dismissPreviewAfter'), before: optionLabel(DISMISS_OPTIONS, String(settings.captureFeedbackDismissSeconds)), after: optionLabel(DISMISS_OPTIONS, String(defaults.captureFeedbackDismissSeconds)) },
  ].filter(({ before, after }) => before !== after);
  return { changes, defaults };
}

function optionLabel(options: Array<{ value: string; label: string }>, value: string) {
  return options.find((option) => option.value === value)?.label ?? value;
}
