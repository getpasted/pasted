import { defaultAppExclusions } from './appExclusionModel';
import { DEFAULT_PRIVATE_BROWSER_SETTINGS } from './appSettingsCapturePolicyModel';
import { translate } from './localization/runtime';
import type { AppSettings, BlacklistApp } from './types';
import { resetBooleanLabel, type SettingsResetChange } from './components/SettingsResetChanges';

function ruleSummary(app: BlacklistApp, locale: string) {
  const rules = [
    app.ignoreText ? translate('component.settingsBlacklistPanel.text') : null,
    app.ignoreImages ? translate('component.settingsBlacklistPanel.images') : null,
    app.ignoreFiles ? translate('component.settingsBlacklistPanel.files') : null,
    app.ignoreHotkeys ? translate('component.settingsBlacklistPanel.hotkeys') : null,
  ].filter((rule): rule is string => rule !== null);
  return rules.length > 0
    ? new Intl.ListFormat(locale, { style: 'short', type: 'conjunction' }).format(rules)
    : translate('component.settingsBlacklistPanel.nothing');
}

export function appExclusionResetChanges(apps: BlacklistApp[], settings: AppSettings, locale: string): SettingsResetChange[] {
  const defaults = defaultAppExclusions();
  const currentByName = new Map(apps.map((app) => [app.name, app]));
  const defaultByName = new Map(defaults.map((app) => [app.name, app]));
  const appChanges: SettingsResetChange[] = [];
  for (const app of apps) {
    const next = defaultByName.get(app.name);
    if (!next) appChanges.push({ label: app.name, before: ruleSummary(app, locale), after: null });
    else if (ruleSummary(app, locale) !== ruleSummary(next, locale)) {
      appChanges.push({ label: app.name, before: ruleSummary(app, locale), after: ruleSummary(next, locale) });
    }
  }
  for (const app of defaults) {
    if (!currentByName.has(app.name)) appChanges.push({ label: app.name, before: null, after: ruleSummary(app, locale) });
  }
  const policyChanges = [
    {
      label: translate('component.settingsBlacklistPanel.allPrivateIncognitoWebBrowsers'),
      before: resetBooleanLabel(settings.excludePrivateBrowserWindows),
      after: resetBooleanLabel(DEFAULT_PRIVATE_BROWSER_SETTINGS.excludePrivateBrowserWindows),
    },
    {
      label: translate('component.settingsBlacklistPanel.ifDetectionIsUnavailable'),
      before: fallbackLabel(settings.privateBrowserUnavailablePolicy),
      after: fallbackLabel(DEFAULT_PRIVATE_BROWSER_SETTINGS.privateBrowserUnavailablePolicy),
    },
  ].filter(({ before, after }) => before !== after);
  return [...appChanges, ...policyChanges];
}

function fallbackLabel(policy: AppSettings['privateBrowserUnavailablePolicy']) {
  return translate(policy === 'capture'
    ? 'component.settingsBlacklistPanel.continueCapturing'
    : 'component.settingsBlacklistPanel.excludeTheBrowser');
}
