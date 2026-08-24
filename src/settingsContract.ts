import contractJson from '../shared/settings-contract.json' with { type: 'json' };
import type { AppSettings } from './types';
export type { AppSettings } from './types';

export type SettingsPageId = typeof contractJson.pages[number]['id'];

const settings = contractJson.settings as Array<{
  key: string;
  owner: string;
  reset: string;
  default?: unknown;
}>;
const definitions = new Map(settings.map((definition) => [definition.key, definition]));

export function settingDefault<K extends keyof AppSettings>(key: K): AppSettings[K] {
  const definition = definitions.get(key);
  if (!definition || definition.default === undefined) {
    throw new Error(`Missing default for ${key}`);
  }
  return definition.default as AppSettings[K];
}

export function rawSettingDefault<T>(key: string): T {
  const definition = definitions.get(key);
  if (!definition || definition.default === undefined) {
    throw new Error(`Missing default for ${key}`);
  }
  return definition.default as T;
}

export function resettableSettingKeys(page: SettingsPageId): string[] {
  return settings
    .filter((definition) => definition.owner === page && definition.reset === 'default')
    .map(({ key }) => key);
}

export const SETTINGS_CONTRACT_VERSION = contractJson.version;
