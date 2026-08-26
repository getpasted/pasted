import { Building2, Coffee, Droplet, Drum, Laptop, Moon, Pizza, Snowflake, Zap } from 'lucide-react';
import { useLocalization } from '../localization/LocalizationProvider';
import { translate } from '../localization/runtime';
import type { AppSettings } from '../types';
import { MenuSelect } from './MenuSelect';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
const appearanceModes = [
  { value: 'system', get label() { return translate('common.system'); }, Icon: Laptop },
  { value: 'dark', get label() { return translate('component.settingsGeneralPanel.dark'); }, Icon: Moon },
  { value: 'cool', get label() { return translate('component.settingsGeneralPanel.cool'); }, Icon: Snowflake },
  { value: 'warm', get label() { return translate('component.settingsGeneralPanel.warm'); }, Icon: Coffee },
  { value: '2894', label: '2894', Icon: Building2 },
  { value: 'sauced', get label() { return translate('component.settingsGeneralPanel.sauced'); }, Icon: Pizza },
  { value: 'vampire', get label() { return translate('component.settingsGeneralPanel.vampire'); }, Icon: Droplet },
  { value: 'flux', get label() { return translate('component.settingsGeneralPanel.flux'); }, Icon: Zap },
  { value: '808', label: '808', Icon: Drum },
] as const;

const appearanceGroups = [
  { get label() { return translate('common.system'); }, values: ['system'] },
  { get label() { return translate('component.settingsGeneralPanel.darkSchemes'); }, values: ['dark', 'vampire', 'flux', '808'] },
  { get label() { return translate('component.settingsGeneralPanel.lightSchemes'); }, values: ['cool', 'warm', '2894', 'sauced'] },
] as const;

interface SettingsGeneralAppearanceSectionProps {
  settings: AppSettings;
  onUpdateSettings: (settings: Partial<AppSettings>) => void;
}

export function SettingsGeneralAppearanceSection({ settings, onUpdateSettings }: SettingsGeneralAppearanceSectionProps) {
  const { t, locales } = useLocalization();

  return <div className="space-y-4">
    <SettingsSubsectionHeader
      title={translate('component.settingsGeneralPanel.appearance')}
      description={translate('component.settingsGeneralPanel.chooseAColorSchemeAndDisplayScale')}
    />
    <div className="flex items-center justify-between pb-1">
      <span className="font-medium">
        {translate('component.settingsGeneralPanel.colorScheme')} <strong className="theme-text-muted ms-1">{appearanceModes.find(({ value }) => value === (settings.themeMode || 'system'))?.label}</strong>
      </span>
      <div className="theme-surface appearance-picker ui-card-radius flex items-center gap-1 border p-1" role="group" aria-label={translate('component.settingsGeneralPanel.appearanceScheme')}>
        {appearanceGroups.map((group) => (
          <div key={group.label} className="appearance-picker-group flex items-center gap-1" role="group" aria-label={group.label}>
            {group.values.map((value) => {
              const mode = appearanceModes.find((candidate) => candidate.value === value)!;
              const isActive = (settings.themeMode || 'system') === value;
              return <button
                key={value}
                type="button"
                title={mode.label}
                aria-label={translate('component.settingsGeneralPanel.labelAppearance', { label: mode.label })}
                aria-pressed={isActive}
                onClick={() => onUpdateSettings({ themeMode: value })}
                className={`appearance-mode-button ui-control-radius flex h-8 w-8 items-center justify-center border border-transparent transition-colors ${isActive ? 'is-active' : ''}`}
              >
                <mode.Icon className="w-3.5 h-3.5" />
              </button>;
            })}
          </div>
        ))}
      </div>
    </div>
    <div className="flex items-start justify-between gap-4">
      <div className="min-w-0 flex-1 pe-4">
        <span className="font-semibold theme-text-main block">{t('settings.general.language.label')}</span>
        <p className="text-[11px] theme-text-muted leading-normal mt-0.5">{t('settings.general.language.description')}</p>
      </div>
      <MenuSelect
        value={settings.language}
        options={[
          { value: 'system', label: t('common.automatic') },
          ...locales.map(({ code, nativeName }, index) => ({ value: code, label: nativeName, dividerBefore: index === 0 })),
        ]}
        onChange={(value) => onUpdateSettings({ language: value })}
        label={t('settings.general.language.ariaLabel')}
        className="settings-menu-select shrink-0"
      />
    </div>
  </div>;
}
