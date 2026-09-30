import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { AppSettings } from '../types';
import { settingsApi } from '../api/settings';
import { useLocalization } from '../localization/LocalizationProvider';
import { translate } from '../localization/runtime';
import { APP_EVENTS } from '../utils/appEvents';
import { MenuSelect } from './MenuSelect';

const countPresets = [
  { value: '0', get label() { return translate('component.settingsGeneralPanel.unlimited'); } },
  { value: '250', get label() { return translate('component.settingsGeneralPanel.value250Clips'); } },
  { value: '500', get label() { return translate('component.settingsGeneralPanel.value500Clips'); } },
  { value: '1000', get label() { return translate('component.settingsGeneralPanel.value1000ClipsDefault'); } },
  { value: '5000', get label() { return translate('component.settingsGeneralPanel.value5000Clips'); } },
  { value: '10000', get label() { return translate('component.settingsGeneralPanel.value10000Clips'); } },
  { value: '50000', get label() { return translate('component.settingsGeneralPanel.value50000Clips'); } },
];

interface Props {
  settings: AppSettings;
  ageOptions: { value: string; label: string }[];
  onUpdateSettings: (updates: Partial<AppSettings>) => void;
}

export function SettingsGeneralHistoryRetentionSection({ settings, ageOptions, onUpdateSettings }: Props) {
  const { t } = useLocalization();
  const [expandedTo, setExpandedTo] = useState<number | null>(null);

  useEffect(() => {
    let active = true;
    const refresh = () => {
      void settingsApi.load().then((saved) => {
        if (!active) return;
        const limit = Number(saved.keepClipCount);
        const expanded = Number(saved.historyLimitAutoExpanded);
        setExpandedTo(Number.isFinite(limit) && expanded === limit ? expanded : null);
      }).catch(console.error);
    };
    refresh();
    const listeners = Promise.all([
      listen(APP_EVENTS.clipAdded, refresh),
      listen(APP_EVENTS.appSettingChanged, refresh),
    ]);
    return () => {
      active = false;
      void listeners.then((unlisten) => unlisten.forEach((stop) => stop())).catch(console.error);
    };
  }, []);

  const count = settings.keepClipCount;
  const countOptions = countPresets.some(({ value }) => Number(value) === count)
    ? countPresets
    : [
        ...countPresets.slice(0, 1),
        { value: String(count), label: t('format.customValue', { value: t('format.clipCount', { count }), custom: t('common.custom') }) },
        ...countPresets.slice(1),
      ];

  return (
    <div className="theme-surface overflow-hidden rounded-xl border">
      <div className="flex items-center justify-between gap-4 px-3 py-2.5">
        <div className="min-w-0">
          <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.keepClipsFor')}</span>
          <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
            {translate('component.settingsGeneralPanel.eligibleClipsOlderThanThisMoveToTrashAutomatically')}
          </p>
        </div>
        <MenuSelect value={String(settings.keepClipAgeDays)} options={ageOptions}
          onChange={(value) => onUpdateSettings({ keepClipAgeDays: Number(value) })}
          label={translate('component.settingsGeneralPanel.maximumClipAge')}
          className="settings-menu-select w-40 shrink-0" />
      </div>
      <div className="theme-divider flex items-center justify-between gap-4 border-t px-3 py-2.5">
        <div className="min-w-0">
          <span className="font-semibold theme-text-main block">{translate('component.settingsGeneralPanel.maximumClips')}</span>
          <p className="text-[11px] theme-text-muted leading-normal mt-0.5">
            {translate('component.settingsGeneralPanel.theOldestEligibleClipsMoveToTrashFirst')}
          </p>
        </div>
        <MenuSelect value={String(count)} options={countOptions}
          onChange={(value) => { setExpandedTo(null); onUpdateSettings({ keepClipCount: Number(value) }); }}
          label={translate('component.settingsGeneralPanel.maximumClipsRetained')}
          className="settings-menu-select w-40 shrink-0" />
      </div>
      <p className="theme-divider theme-text-subtle border-t px-3 py-2 text-[10px] leading-normal">
        {translate('component.settingsGeneralPanel.bothLimitsApplyPinnedAndProtectedClipsNeverMoveToTrashAutomatically')}
      </p>
      {expandedTo === count && (
        <p className="theme-divider theme-text-muted border-t px-3 py-2 text-xs leading-normal">
          {translate('component.settingsGeneralPanel.historyLimitIncreasedForProtectedClips')}
        </p>
      )}
    </div>
  );
}
