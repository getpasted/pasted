import {
  CircleDot,
  Compass,
  Feather,
  Flame,
  Shield,
  Waves,
  type LucideIcon,
} from 'lucide-react';
import type { AppSettings } from '../types';
import { translate, type TranslationKey } from '../localization/runtime';
import { MenuSelect } from './MenuSelect';
import { SettingsAccentTile } from './SettingsAccentTile';
import { SettingsSwitch } from './SettingsSwitch';

const supportedBrowsers: Array<{ browser: string; modeKey: TranslationKey; Icon: LucideIcon }> = [
  { browser: 'Safari', modeKey: 'component.settingsBlacklistPanel.safariPrivateBrowsing', Icon: Compass },
  { browser: 'Chrome', modeKey: 'component.settingsBlacklistPanel.chromeIncognito', Icon: CircleDot },
  { browser: 'Edge', modeKey: 'component.settingsBlacklistPanel.edgeInPrivate', Icon: Waves },
  { browser: 'Firefox', modeKey: 'component.settingsBlacklistPanel.firefoxPrivateBrowsing', Icon: Flame },
  { browser: 'DuckDuckGo', modeKey: 'component.settingsBlacklistPanel.duckDuckGoFireWindow', Icon: Feather },
  { browser: 'Brave', modeKey: 'component.settingsBlacklistPanel.bravePrivateWindows', Icon: Shield },
];

export function PrivateBrowserExclusionSection({
  settings,
  onUpdateSettings,
}: {
  settings: AppSettings;
  onUpdateSettings: (updates: Partial<AppSettings>) => void;
}) {
  const toggleLabel = translate('component.settingsBlacklistPanel.allPrivateIncognitoWebBrowsers');
  const fallbackOptions = [
    { value: 'capture', label: translate('component.settingsBlacklistPanel.continueCapturing') },
    { value: 'exclude_browser', label: translate('component.settingsBlacklistPanel.excludeTheBrowser') },
  ];

  return (
    <section className="space-y-3" aria-labelledby="private-browser-exclusions-heading">
      <div>
        <h3 id="private-browser-exclusions-heading" className="theme-text-main text-sm font-bold">
          {translate('component.settingsBlacklistPanel.privateBrowserWindows')}
        </h3>
        <p className="theme-text-muted mt-1 text-[11px] leading-relaxed">
          {translate('component.settingsBlacklistPanel.skipClipsCopiedFromSupportedPrivateBrowsingWindows')}
        </p>
      </div>

      <div className="theme-surface rounded-xl border p-4">
        <div className="flex items-center justify-between gap-4">
          <div className="flex min-w-0 items-center gap-3">
            <SettingsAccentTile size="compact"><Shield className="h-4 w-4" /></SettingsAccentTile>
            <div className="min-w-0">
              <div className="flex flex-wrap items-center gap-2">
                <span className="theme-text-main font-semibold">{toggleLabel}</span>
                <span className="theme-badge rounded-full border px-2 py-0.5 text-[9px] font-bold uppercase tracking-wide">
                  {translate('component.settingsBlacklistPanel.builtIn')}
                </span>
              </div>
            </div>
          </div>
          <SettingsSwitch
            checked={settings.excludePrivateBrowserWindows}
            label={toggleLabel}
            onClick={() => onUpdateSettings({
              excludePrivateBrowserWindows: !settings.excludePrivateBrowserWindows,
            })}
          />
        </div>

        <div className="theme-divider mt-4 border-t pt-4">
          <div className="theme-text-subtle mb-2 text-[10px] font-bold uppercase tracking-wider">
            {translate('component.settingsBlacklistPanel.supportedBrowsers')}
          </div>
          <ul className="grid gap-2 sm:grid-cols-2">
            {supportedBrowsers.map(({ browser, modeKey, Icon }) => (
              <li key={browser} className="flex items-start gap-2 text-[11px]">
                <Icon className="theme-text-subtle mt-0.5 h-3.5 w-3.5 shrink-0" aria-hidden="true" />
                <span className="min-w-0">
                  <span className="theme-text-main block font-bold">{browser}</span>
                  <span className="theme-text-muted mt-0.5 block leading-snug">{translate(modeKey)}</span>
                </span>
              </li>
            ))}
          </ul>
        </div>

        <div className="theme-divider mt-4 flex items-center justify-between gap-4 border-t pt-4">
          <label className="theme-text-main min-w-0 font-semibold">
            {translate('component.settingsBlacklistPanel.ifDetectionIsUnavailable')}
          </label>
          <MenuSelect
            value={settings.privateBrowserUnavailablePolicy}
            options={fallbackOptions}
            label={translate('component.settingsBlacklistPanel.ifDetectionIsUnavailable')}
            className="w-44 shrink-0"
            disabled={!settings.excludePrivateBrowserWindows}
            onChange={(value) => onUpdateSettings({
              privateBrowserUnavailablePolicy: value as AppSettings['privateBrowserUnavailablePolicy'],
            })}
          />
        </div>
        <p className="platform-linux-only theme-text-subtle mt-3 text-[10px] leading-relaxed">
          {translate('component.settingsBlacklistPanel.nativeWaylandMayNotExposeTheFocusedBrowser')}
        </p>
      </div>
    </section>
  );
}
