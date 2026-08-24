import type { AppSettings } from '../types';
import { isAnalysisFunctionalityEnabled } from '../appSettingsRetentionModel';
import { translate } from '../localization/runtime';
import { MenuSelect } from './MenuSelect';

interface SettingsGeneralHistoryLimitsProps {
  settings: AppSettings;
  onUpdateSettings: (settings: Partial<AppSettings>) => void;
}

const revisionLimitOptions = [10, 25, 50, 100]
  .map((value) => ({ value: String(value), label: translate('component.settingsGeneralPanel.valueRevisions', { value }) }))
  .concat({ value: '0', get label() { return translate('component.settingsGeneralPanel.unlimited'); } });

const analysisLimitOptions = [10, 25, 50, 100]
  .map((value) => ({ value: String(value), label: translate('component.settingsGeneralPanel.valueAnalysisAttempts', { value }) }))
  .concat({ value: '0', get label() { return translate('component.settingsGeneralPanel.unlimited'); } });

function HistoryLimitRow({
  title,
  description,
  value,
  options,
  label,
  onChange,
}: {
  title: string;
  description: string;
  value: number;
  options: typeof revisionLimitOptions;
  label: string;
  onChange: (value: number) => void;
}) {
  return <div className="flex items-start justify-between gap-4">
    <div className="min-w-0 flex-1">
      <span className="theme-text-main block font-semibold">{title}</span>
      <p className="theme-text-muted mt-0.5 text-[11px] leading-normal">{description}</p>
    </div>
    <MenuSelect
      value={String(value)}
      options={options}
      onChange={(next) => onChange(Number(next))}
      label={label}
      className="settings-menu-select"
    />
  </div>;
}

export function SettingsGeneralHistoryLimits({ settings, onUpdateSettings }: SettingsGeneralHistoryLimitsProps) {
  const analysisEnabled = isAnalysisFunctionalityEnabled(settings);
  return <>
    {settings.enableRevisions && <HistoryLimitRow
      title={translate('component.settingsGeneralPanel.revisionsPerClip')}
      description={settings.revisionHistoryLimit === 0
        ? translate('component.settingsGeneralPanel.unlimitedRevisionHistoryDescription')
        : translate('component.settingsGeneralPanel.keepsCompleteTextSnapshotsForEditsOcrTransformsAndRestores')}
      value={settings.revisionHistoryLimit}
      options={revisionLimitOptions}
      label={translate('component.settingsGeneralPanel.revisionsRetainedPerClip')}
      onChange={(revisionHistoryLimit) => onUpdateSettings({ revisionHistoryLimit })}
    />}
    {analysisEnabled && <HistoryLimitRow
      title={translate('component.settingsGeneralPanel.analyzationsPerClip')}
      description={settings.analysisAttemptsPerClip === 0
        ? translate('component.settingsGeneralPanel.unlimitedAnalysisHistoryDescription')
        : translate('component.settingsGeneralPanel.keepsRecentAnalysisAttemptsForEachExtractor')}
      value={settings.analysisAttemptsPerClip}
      options={analysisLimitOptions}
      label={translate('component.settingsGeneralPanel.analyzationsRetainedPerClip')}
      onChange={(analysisAttemptsPerClip) => onUpdateSettings({ analysisAttemptsPerClip })}
    />}
  </>;
}
