import { settingDefault, type AppSettings } from './settingsContract.ts';
type CapturePolicySettings = Pick<
  AppSettings,
  'alwaysPastePlainText' | 'excludePrivateBrowserWindows' | 'privateBrowserUnavailablePolicy'
>;
type PrivateBrowserSettings = Pick<AppSettings, 'excludePrivateBrowserWindows' | 'privateBrowserUnavailablePolicy'>;

export const DEFAULT_PRIVATE_BROWSER_SETTINGS: PrivateBrowserSettings = {
  excludePrivateBrowserWindows: settingDefault('excludePrivateBrowserWindows'),
  privateBrowserUnavailablePolicy: settingDefault('privateBrowserUnavailablePolicy'),
};

export const DEFAULT_CAPTURE_POLICY_SETTINGS: CapturePolicySettings = {
  alwaysPastePlainText: settingDefault('alwaysPastePlainText'),
  ...DEFAULT_PRIVATE_BROWSER_SETTINGS,
};

export function savedCapturePolicySettings(saved: Record<string, string>): CapturePolicySettings {
  return {
    alwaysPastePlainText: saved.alwaysPastePlainText === undefined
      ? DEFAULT_CAPTURE_POLICY_SETTINGS.alwaysPastePlainText
      : saved.alwaysPastePlainText === 'true',
    excludePrivateBrowserWindows: saved.excludePrivateBrowserWindows === undefined
      ? DEFAULT_CAPTURE_POLICY_SETTINGS.excludePrivateBrowserWindows
      : saved.excludePrivateBrowserWindows === 'true',
    privateBrowserUnavailablePolicy: ['capture', 'exclude_browser'].includes(saved.privateBrowserUnavailablePolicy)
      ? saved.privateBrowserUnavailablePolicy as AppSettings['privateBrowserUnavailablePolicy']
      : DEFAULT_CAPTURE_POLICY_SETTINGS.privateBrowserUnavailablePolicy,
  };
}
