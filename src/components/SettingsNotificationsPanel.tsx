import { Bell } from 'lucide-react';
import type { AppSettings } from '../types';
import { MenuSelect } from './MenuSelect';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsSwitch } from './SettingsSwitch';

interface SettingsNotificationsPanelProps {
  settings: AppSettings;
  onUpdateSettings: (updates: Partial<AppSettings>) => void;
}

interface ToggleRowProps {
  checked: boolean;
  disabled?: boolean;
  label: string;
  description: string;
  onChange: () => void;
}

function ToggleRow({ checked, disabled = false, label, description, onChange }: ToggleRowProps) {
  return (
    <div className={`flex items-start justify-between gap-4 ${disabled ? 'settings-disabled-row' : ''}`}>
      <div className="min-w-0">
        <span className="theme-text-main block font-semibold">{label}</span>
        <p className="theme-text-muted mt-0.5 text-[11px] leading-normal">{description}</p>
      </div>
      <SettingsSwitch
        checked={checked}
        label={label}
        disabled={disabled}
        onClick={onChange}
      />
    </div>
  );
}

const POSITION_OPTIONS = [
  { value: 'top-left', label: 'Top Left' },
  { value: 'top-right', label: 'Top Right' },
  { value: 'bottom-left', label: 'Bottom Left' },
  { value: 'bottom-right', label: 'Bottom Right' },
];

const DISMISS_OPTIONS = [
  { value: '3', label: '3 Seconds' },
  { value: '5', label: '5 Seconds' },
  { value: '7', label: '7 Seconds' },
  { value: '10', label: '10 Seconds' },
  { value: '15', label: '15 Seconds' },
  { value: '30', label: '30 Seconds' },
  { value: '0', label: 'Never' },
];

export function SettingsNotificationsPanel({ settings, onUpdateSettings }: SettingsNotificationsPanelProps) {
  return (
    <div className="space-y-6 text-xs">
      <SettingsPanelHeader
        icon={Bell}
        title="Notifications"
        description="Get a quiet confirmation when clipboard capture succeeds or fails."
      />

      <div className="space-y-4">
        <ToggleRow
          checked={settings.captureFeedback}
          label="Capture feedback"
          description="Briefly confirms successful captures without taking focus from the current app."
          onChange={() => onUpdateSettings({ captureFeedback: !settings.captureFeedback })}
        />
        <ToggleRow
          checked={settings.captureFeedbackIgnored}
          disabled={!settings.captureFeedback}
          label="Show skipped captures"
          description="Also acknowledge clipboard items intentionally left alone."
          onChange={() => onUpdateSettings({ captureFeedbackIgnored: !settings.captureFeedbackIgnored })}
        />
        <ToggleRow
          checked={settings.captureFeedbackPreview}
          disabled={!settings.captureFeedback}
          label="Show clip preview"
          description="Show the captured item with quick actions."
          onChange={() => onUpdateSettings({ captureFeedbackPreview: !settings.captureFeedbackPreview })}
        />
        <div className={`flex items-start justify-between gap-4 ${!settings.captureFeedback || !settings.captureFeedbackPreview ? 'opacity-45' : ''}`}>
          <div className="min-w-0 flex-1">
            <span className="theme-text-main block font-semibold">Dismiss Preview After</span>
            <p className="theme-text-muted mt-0.5 text-[11px] leading-normal">
              The countdown pauses while the pointer is over a preview.
            </p>
          </div>
          <MenuSelect
            value={String(settings.captureFeedbackDismissSeconds)}
            options={DISMISS_OPTIONS}
            disabled={!settings.captureFeedback || !settings.captureFeedbackPreview}
            onChange={(value) => onUpdateSettings({ captureFeedbackDismissSeconds: Number(value) })}
            label="Preview dismissal delay"
            className="settings-menu-select"
          />
        </div>
        <div className="flex items-start justify-between gap-4">
          <div className="min-w-0 flex-1">
            <span className="theme-text-main block font-semibold">Screen Position</span>
            <p className="theme-text-muted mt-0.5 text-[11px] leading-normal">
              Uses this corner on whichever display currently contains the pointer.
            </p>
          </div>
          <MenuSelect
            value={settings.captureFeedbackPosition}
            options={POSITION_OPTIONS}
            onChange={(value) => onUpdateSettings({
              captureFeedbackPosition: value as AppSettings['captureFeedbackPosition'],
            })}
            label="Capture feedback position"
            className="settings-menu-select"
          />
        </div>
      </div>
      <p className="theme-text-muted text-[11px] leading-normal">
        Capture feedback stays on-device and never exposes copied text, images, file names, or paths to system notifications.
      </p>
    </div>
  );
}
