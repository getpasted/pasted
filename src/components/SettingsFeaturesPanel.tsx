import {
  Activity,
  BarChart3,
  Cable,
  Command,
  ScanSearch,
  Folder,
  HelpCircle,
  LayoutGrid,
  ListOrdered,
  Bell,
  History,
  Pin,
  Shield,
  ScanText,
  Sparkles,
  StickyNote,
  Trash2,
  Shapes,
  AppWindow,
  AudioLines,
  LockKeyhole,
  EyeOff,
  Layers3,
  FileType2,
  FilePenLine,
  Search,
} from 'lucide-react';
import type { AppSettings } from '../types';
import {
  FEATURE_DEFINITIONS,
  FEATURE_GROUPS,
  activeFeaturePreset,
  featureUpdatesForPreset,
  type FeatureId,
  type FeatureGroupId,
  type FeaturePreset,
} from '../utils/features';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsSectionHeading } from './SettingsSectionHeading';
import { SettingsSwitch } from './SettingsSwitch';
import { InfoPopover } from './InfoPopover';
import { SettingsPanelNote } from './SettingsPanelNote';
import { translate, type TranslationKey } from '../localization/runtime';
import { useLocalization } from '../localization/LocalizationProvider';

const FEATURE_GROUP_KEYS: Record<FeatureGroupId, { label: TranslationKey; description: TranslationKey }> = {
  library: { label: 'feature.group.library.label', description: 'feature.group.library.description' },
  discovery: { label: 'feature.group.discovery.label', description: 'feature.group.discovery.description' },
  workflow: { label: 'feature.group.workflow.label', description: 'feature.group.workflow.description' },
  app: { label: 'feature.group.app.label', description: 'feature.group.app.description' },
};

const FEATURE_KEYS: Record<FeatureId, { label: TranslationKey; description: TranslationKey; caution?: TranslationKey }> = {
  analytics: { label: 'feature.analytics.label', description: 'feature.analytics.description' },
  bins: { label: 'feature.bins.label', description: 'feature.bins.description' },
  clipTypes: { label: 'feature.clipTypes.label', description: 'feature.clipTypes.description' },
  fileFormats: { label: 'feature.fileFormats.label', description: 'feature.fileFormats.description' },
  contentClassification: { label: 'feature.contentClassification.label', description: 'feature.contentClassification.description' },
  concealment: { label: 'feature.concealment.label', description: 'feature.concealment.description' },
  naming: { label: 'feature.naming.label', description: 'feature.naming.description' },
  notes: { label: 'feature.notes.label', description: 'feature.notes.description' },
  notifications: { label: 'feature.notifications.label', description: 'feature.notifications.description' },
  appLock: { label: 'feature.appLock.label', description: 'feature.appLock.description', caution: 'feature.appLock.caution' },
  ocr: { label: 'feature.ocr.label', description: 'feature.ocr.description' },
  transcriptions: { label: 'feature.transcriptions.label', description: 'feature.transcriptions.description' },
  pinning: { label: 'feature.pinning.label', description: 'feature.pinning.description' },
  protection: { label: 'feature.protection.label', description: 'feature.protection.description', caution: 'feature.protection.caution' },
  queue: { label: 'feature.queue.label', description: 'feature.queue.description' },
  revisions: { label: 'feature.revisions.label', description: 'feature.revisions.description', caution: 'feature.revisions.caution' },
  hud: { label: 'feature.hud.label', description: 'feature.hud.description' },
  hotkeys: { label: 'feature.hotkeys.label', description: 'feature.hotkeys.description' },
  trash: { label: 'feature.trash.label', description: 'feature.trash.description', caution: 'feature.trash.caution' },
  transformations: { label: 'feature.transformations.label', description: 'feature.transformations.description' },
  activityLog: { label: 'feature.activityLog.label', description: 'feature.activityLog.description' },
  types: { label: 'feature.types.label', description: 'feature.types.description' },
  sources: { label: 'feature.sources.label', description: 'feature.sources.description' },
  search: { label: 'feature.search.label', description: 'feature.search.description' },
  cli: { label: 'feature.cli.label', description: 'feature.cli.description' },
  help: { label: 'feature.help.label', description: 'feature.help.description' },
};

const FEATURE_PRESET_KEYS: Record<FeaturePreset, TranslationKey> = {
  simple: 'feature.preset.simple',
  full: 'feature.preset.full',
  custom: 'feature.preset.custom',
};

const FEATURE_ICONS = {
  analytics: BarChart3,
  bins: Folder,
  clipTypes: Layers3,
  fileFormats: FileType2,
  contentClassification: ScanSearch,
  concealment: EyeOff,
  naming: FilePenLine,
  notes: StickyNote,
  notifications: Bell,
  appLock: LockKeyhole,
  ocr: ScanText,
  transcriptions: AudioLines,
  pinning: Pin,
  protection: Shield,
  queue: ListOrdered,
  revisions: History,
  hud: LayoutGrid,
  hotkeys: Command,
  trash: Trash2,
  transformations: Sparkles,
  activityLog: Activity,
  types: Shapes,
  sources: AppWindow,
  search: Search,
  cli: Command,
  help: HelpCircle,
} satisfies Record<FeatureId, typeof Activity>;

interface SettingsFeaturesPanelProps {
  settings: AppSettings;
  onUpdateSettings: (updates: Partial<AppSettings>) => void;
}

export function SettingsFeaturesPanel({ settings, onUpdateSettings }: SettingsFeaturesPanelProps) {
  useLocalization();
  const activePreset = activeFeaturePreset(settings);
  const visiblePresets = activePreset === 'custom'
    ? (['custom', 'simple', 'full'] as const)
    : (['simple', 'full'] as const);

  return (
    <div className="space-y-5 text-xs">
      <SettingsPanelHeader
        icon={Cable}
        title={translate('component.settingsFeaturesPanel.functionality')}
        description={translate('component.settingsFeaturesPanel.chooseWhichFeaturesAreAvailable')}
        actions={(
          <div className="theme-surface flex rounded-xl border p-1" aria-label={translate('component.settingsFeaturesPanel.featurePresets')}>
            {visiblePresets.map((preset) => (
              <button
                key={preset}
                type="button"
                aria-pressed={activePreset === preset}
                onClick={() => {
                  if (preset !== 'custom') onUpdateSettings(featureUpdatesForPreset(preset));
                }}
                className={`settings-feature-preset rounded-lg px-3 py-1.5 font-semibold capitalize ${activePreset === preset ? 'is-active' : ''}`}
              >
                {translate(FEATURE_PRESET_KEYS[preset])}
              </button>
            ))}
          </div>
        )}
      />

      <div className="space-y-6">
        {FEATURE_GROUPS.map((group) => (
          <section key={group.id} className="space-y-3">
            <SettingsSectionHeading
              title={translate(FEATURE_GROUP_KEYS[group.id].label)}
              description={translate(FEATURE_GROUP_KEYS[group.id].description)}
            />
            <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
              {FEATURE_DEFINITIONS.filter((feature) => feature.group === group.id).map((feature) => {
                const Icon = FEATURE_ICONS[feature.id];
                const enabled = settings[feature.settingKey];
                const keys = FEATURE_KEYS[feature.id];
                const label = translate(keys.label);
                return (
                  <div
                    key={feature.id}
                    className={`settings-feature-card theme-card-idle border p-4 ${enabled ? 'is-enabled' : ''}`}
                    onClick={(event) => {
                      if ((event.target as HTMLElement).closest('button, a, input, select, textarea')) return;
                      onUpdateSettings({ [feature.settingKey]: !enabled });
                    }}
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div className="flex min-w-0 items-start gap-3">
                        <span className="settings-feature-icon flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border">
                          <Icon className={`h-4 w-4 ${feature.id === 'naming' ? 'theme-named-text' : ''}`} />
                        </span>
                        <div className="min-w-0">
                          <div className="relative w-fit max-w-[calc(100%-1.5rem)]">
                            <h3 className="theme-title font-semibold">{label}</h3>
                            {feature.caution && !enabled && (
                              <span className="settings-feature-caution">
                                <InfoPopover label={translate('component.settingsFeaturesPanel.labelDisabledWarning', { label })} tone="danger">
                                  {keys.caution ? translate(keys.caution) : feature.caution}
                                </InfoPopover>
                              </span>
                            )}
                          </div>
                          <p className="theme-text-muted mt-1 text-[11px] leading-relaxed">{translate(keys.description)}</p>
                        </div>
                      </div>
                      <SettingsSwitch
                        checked={enabled}
                        label={label}
                        onClick={() => onUpdateSettings({ [feature.settingKey]: !enabled })}
                      />
                    </div>
                  </div>
                );
              })}
            </div>
          </section>
        ))}
      </div>

      <SettingsPanelNote>{translate('component.settingsFeaturesPanel.simpleEnablesEssentialClipboardToolsFullEnablesEveryFeatureDisablingAFeature')}</SettingsPanelNote>
    </div>
  );
}
