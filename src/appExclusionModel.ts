import type { BlacklistApp } from './types';
import { rawSettingDefault } from './settingsContract.ts';

const DEFAULT_APP_EXCLUSIONS = rawSettingDefault<ReadonlyArray<Readonly<BlacklistApp>>>('blacklistApps');

export function defaultAppExclusions(): BlacklistApp[] {
  return DEFAULT_APP_EXCLUSIONS.map((app) => ({ ...app }));
}

export function normalizeAppExclusions(value: unknown): BlacklistApp[] {
  if (!Array.isArray(value)) return defaultAppExclusions();
  return value.flatMap((entry, index) => {
    if (typeof entry === 'string' && entry.trim()) {
      return [{ id: `legacy-${index}`, name: entry, icon: 'Lock', ignoreText: true, ignoreImages: true, ignoreFiles: true, ignoreHotkeys: false }];
    }
    if (!entry || typeof entry !== 'object') return [];
    const rule = entry as Partial<BlacklistApp>;
    if (typeof rule.name !== 'string' || !rule.name.trim()) return [];
    return [{
      id: typeof rule.id === 'string' ? rule.id : `legacy-${index}`,
      name: rule.name,
      icon: typeof rule.icon === 'string' ? rule.icon : 'Lock',
      ignoreText: rule.ignoreText !== false,
      ignoreImages: rule.ignoreImages !== false,
      ignoreFiles: rule.ignoreFiles !== false,
      ignoreHotkeys: rule.ignoreHotkeys === true,
    }];
  });
}
