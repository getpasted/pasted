import {
  Activity,
  BarChart3,
  Cable,
  Command,
  ScanSearch,
  Folder,
  HelpCircle,
  LayoutGrid,
  Stethoscope,
  ListOrdered,
  Bell,
  History,
  Pin,
  Shield,
  ScanText,
  Sparkles,
  StickyNote,
  Trash2,
} from 'lucide-react';
import type { AppSettings } from '../types';
import {
  FEATURE_DEFINITIONS,
  activeFeaturePreset,
  featureUpdatesForPreset,
  type FeatureId,
} from '../utils/features';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { InfoPopover } from './InfoPopover';

const FEATURE_ICONS = {
  analytics: BarChart3,
  bins: Folder,
  contentDetection: ScanSearch,
  diagnostics: Stethoscope,
  notes: StickyNote,
  notifications: Bell,
  ocr: ScanText,
  pinning: Pin,
  protection: Shield,
  queue: ListOrdered,
  revisions: History,
  hud: LayoutGrid,
  trash: Trash2,
  transformations: Sparkles,
  activityLog: Activity,
  cli: Command,
  help: HelpCircle,
} satisfies Record<FeatureId, typeof Activity>;

interface SettingsFeaturesPanelProps {
  settings: AppSettings;
  onUpdateSettings: (updates: Partial<AppSettings>) => void;
}

export function SettingsFeaturesPanel({ settings, onUpdateSettings }: SettingsFeaturesPanelProps) {
  const activePreset = activeFeaturePreset(settings);

  return (
    <div className="space-y-5 text-xs">
      <SettingsPanelHeader
        icon={Cable}
        title="Features"
        description="Make Pasted as focused or as powerful as you want. Hidden features keep their existing data."
        actions={(
          <div className="theme-surface flex rounded-xl border p-1" aria-label="Feature presets">
            {(['simple', 'full'] as const).map((preset) => (
              <button
                key={preset}
                type="button"
                aria-pressed={activePreset === preset}
                onClick={() => onUpdateSettings(featureUpdatesForPreset(preset))}
                className={`settings-feature-preset rounded-lg px-3 py-1.5 font-semibold capitalize ${activePreset === preset ? 'is-active' : ''}`}
              >
                {preset}
              </button>
            ))}
          </div>
        )}
      />

      {activePreset === 'custom' && (
        <p className="theme-status-info rounded-xl border px-3 py-2 text-[11px]">
          Custom setup — your individual choices differ from the Simple and Full presets.
        </p>
      )}

      <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
        {FEATURE_DEFINITIONS.map((feature) => {
          const Icon = FEATURE_ICONS[feature.id];
          const enabled = settings[feature.settingKey];
          return (
            <section key={feature.id} className={`settings-feature-card theme-card-idle border p-4 ${enabled ? 'is-enabled' : ''}`}>
              <div className="flex items-start justify-between gap-3">
                <div className="flex min-w-0 items-start gap-3">
                  <span className="settings-feature-icon flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border">
                    <Icon className="h-4 w-4" />
                  </span>
                  <div className="min-w-0">
                    <div className="relative w-fit max-w-[calc(100%-1.5rem)]">
                      <h3 className="theme-title font-semibold">{feature.label}</h3>
                      {feature.caution && !enabled && (
                        <span className="settings-feature-caution">
                          <InfoPopover label={`${feature.label} disabled warning`} tone="danger">
                            {feature.caution}
                          </InfoPopover>
                        </span>
                      )}
                    </div>
                    <p className="theme-text-muted mt-1 text-[11px] leading-relaxed">{feature.description}</p>
                  </div>
                </div>
                <button
                  type="button"
                  role="switch"
                  aria-checked={enabled}
                  aria-label={`${enabled ? 'Disable' : 'Enable'} ${feature.label}`}
                  onClick={() => onUpdateSettings({ [feature.settingKey]: !enabled })}
                  className={`settings-switch relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent ${enabled ? 'is-on' : ''}`}
                >
                  <span className={`settings-switch-thumb pointer-events-none inline-block h-4 w-4 rounded-full shadow transition-transform ${enabled ? 'translate-x-4' : 'translate-x-0'}`} />
                </button>
              </div>
            </section>
          );
        })}
      </div>
    </div>
  );
}
