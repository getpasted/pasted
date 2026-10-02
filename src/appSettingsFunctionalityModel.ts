import type { FunctionalitySettings } from './appSettingsTypes/functionality';
import { settingDefault } from './settingsContract.ts';
import { FEATURE_SETTING_KEYS } from './utils/features';

export const DEFAULT_FUNCTIONALITY_SETTINGS = Object.fromEntries(
  FEATURE_SETTING_KEYS.map((key) => [key, settingDefault(key)]),
) as unknown as FunctionalitySettings;

export function savedFunctionalitySettings(saved: Record<string, string>): Partial<FunctionalitySettings> {
  return Object.fromEntries(
    FEATURE_SETTING_KEYS.filter((key) => saved[key] !== undefined)
      .map((key) => [key, saved[key] === 'true']),
  ) as Partial<FunctionalitySettings>;
}
