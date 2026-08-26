import type { AppSettings } from '../types';
import { useLocalization } from '../localization/LocalizationProvider';
import { translate } from '../localization/runtime';
import { MenuSelect } from './MenuSelect';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';

interface SettingsGeneralSearchHistorySectionProps {
  settings: AppSettings;
  onUpdateSettings: (settings: Partial<AppSettings>) => void;
}

const countPresets = [
  { value: '0', get label() { return translate('component.settingsGeneralPanel.unlimited'); } },
  ...[25, 50, 100, 250, 500, 1_000].map((value) => ({
    value: String(value),
    label: translate(value === 100
      ? 'component.settingsGeneralPanel.valueSearchesDefault'
      : 'component.settingsGeneralPanel.valueSearches', { value }),
  })),
];

const agePresets = [
  { value: '0', get label() { return translate('component.settingsGeneralPanel.forever'); } },
  { value: '1', get label() { return translate('component.settingsGeneralPanel.value1Day'); } },
  { value: '7', get label() { return translate('component.settingsGeneralPanel.value7Days'); } },
  { value: '30', get label() { return translate('component.settingsGeneralPanel.value30Days'); } },
  { value: '90', get label() { return translate('component.settingsGeneralPanel.value90Days'); } },
  { value: '365', get label() { return translate('component.settingsGeneralPanel.value1Year'); } },
];

export function SettingsGeneralSearchHistorySection({ settings, onUpdateSettings }: SettingsGeneralSearchHistorySectionProps) {
  const { t } = useLocalization();
  if (!settings.enableSearch) return null;
  const countOptions = countPresets.some(({ value }) => Number(value) === settings.searchHistoryLimit)
    ? countPresets
    : [
        ...countPresets.slice(0, 1),
        { value: String(settings.searchHistoryLimit), label: t('format.customValue', { value: t('component.settingsGeneralPanel.valueSearches', { value: settings.searchHistoryLimit }), custom: t('common.custom') }) },
        ...countPresets.slice(1),
      ];
  const ageOptions = agePresets.some(({ value }) => Number(value) === settings.searchHistoryAgeDays)
    ? agePresets
    : [
        ...agePresets.slice(0, 1),
        { value: String(settings.searchHistoryAgeDays), label: t('format.customValue', { value: t('format.dayCount', { count: settings.searchHistoryAgeDays }), custom: t('common.custom') }) },
        ...agePresets.slice(1),
      ];
  return <>
    <div className="theme-divider border-t" />
    <div className="space-y-4">
      <SettingsSubsectionHeader
        title={translate('component.settingsSearchHistoryPanel.searchHistory')}
        description={translate('component.settingsGeneralPanel.chooseHowMuchSuccessfulSearchHistoryToKeep')}
      />
      <div className="theme-surface overflow-hidden rounded-xl border">
        <div className="flex items-center justify-between gap-4 px-3 py-2.5">
          <div className="min-w-0">
            <span className="theme-text-main block font-semibold">{translate('component.settingsGeneralPanel.keepSearchesFor')}</span>
            <p className="theme-text-muted mt-0.5 text-[11px] leading-normal">{translate('component.settingsGeneralPanel.olderSearchesAreRemovedAutomatically')}</p>
          </div>
          <MenuSelect value={String(settings.searchHistoryAgeDays)} options={ageOptions} onChange={(value) => onUpdateSettings({ searchHistoryAgeDays: Number(value) })} label={translate('component.settingsGeneralPanel.maximumSearchAge')} className="settings-menu-select w-40 shrink-0" />
        </div>
        <div className="theme-divider flex items-center justify-between gap-4 border-t px-3 py-2.5">
          <div className="min-w-0">
            <span className="theme-text-main block font-semibold">{translate('component.settingsGeneralPanel.maximumSearches')}</span>
            <p className="theme-text-muted mt-0.5 text-[11px] leading-normal">{translate('component.settingsGeneralPanel.theOldestSearchesAreRemovedFirst')}</p>
          </div>
          <MenuSelect value={String(settings.searchHistoryLimit)} options={countOptions} onChange={(value) => onUpdateSettings({ searchHistoryLimit: Number(value) })} label={translate('component.settingsGeneralPanel.maximumSearchesRetained')} className="settings-menu-select w-40 shrink-0" />
        </div>
        <p className="theme-divider theme-text-subtle border-t px-3 py-2 text-[10px] leading-normal">{translate('component.settingsGeneralPanel.bothSearchLimitsApplyUnlimitedAndForeverDisableAutomaticRemoval')}</p>
      </div>
    </div>
  </>;
}
