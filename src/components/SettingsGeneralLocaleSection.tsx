import { useLocalization } from '../localization/LocalizationProvider';
import { translate } from '../localization/runtime';
import type { AppSettings } from '../types';
import { MenuSelect } from './MenuSelect';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';

interface SettingsGeneralLocaleSectionProps {
  settings: AppSettings;
  onUpdateSettings: (settings: Partial<AppSettings>) => void;
}

export function SettingsGeneralLocaleSection({ settings, onUpdateSettings }: SettingsGeneralLocaleSectionProps) {
  const { t, locales } = useLocalization();
  return <section className="space-y-4" aria-labelledby="general-language-region-title">
    <SettingsSubsectionHeader
      id="general-language-region-title"
      title={translate('component.settingsGeneralPanel.languageAndRegion')}
      description={translate('component.settingsGeneralPanel.chooseTheLanguageAndRegionalFormats')}
    />
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
  </section>;
}
