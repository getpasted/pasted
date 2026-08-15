import type { ReactNode } from 'react';
import { Bell } from 'lucide-react';
import type { AppSettings } from '../types';
import { MenuSelect } from './MenuSelect';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsPanelNote } from './SettingsPanelNote';
import { SettingsSwitch } from './SettingsSwitch';

interface SettingsNotificationsPanelProps {
  settings: AppSettings;
  onUpdateSettings: (updates: Partial<AppSettings>) => void;
}

interface SettingRowProps {
  disabled?: boolean;
  label: string;
  description: string;
  action: ReactNode;
}

function SettingRow({ disabled = false, label, description, action }: SettingRowProps) {
  return (
    <div className={`flex items-start justify-between gap-4 ${disabled ? 'settings-disabled-row' : ''}`}>
      <div className="min-w-0 flex-1">
        <span className="theme-text-main block font-semibold">{label}</span>
        <p className="theme-text-muted mt-0.5 text-[11px] leading-normal">{description}</p>
      </div>
      {action}
    </div>
  );
}

const POSITION_OPTIONS = [
  { value: 'top-left', label: 'Top left' },
  { value: 'top-right', label: 'Top right' },
  { value: 'bottom-left', label: 'Bottom left' },
  { value: 'bottom-right', label: 'Bottom right' },
];

const DISMISS_OPTIONS = [
  { value: '3', label: '3 seconds' },
  { value: '5', label: '5 seconds' },
  { value: '7', label: '7 seconds' },
  { value: '10', label: '10 seconds' },
  { value: '15', label: '15 seconds' },
  { value: '30', label: '30 seconds' },
  { value: '0', label: 'Never' },
];

export function SettingsNotificationsPanel({ settings, onUpdateSettings }: SettingsNotificationsPanelProps) {
  return (
    <div className="space-y-5 text-xs">
      <SettingsPanelHeader
        icon={Bell}
        title="Notifications"
        description="Get a quiet confirmation when clipboard capture succeeds or fails."
      />

      <div className="space-y-4">
        <SettingRow
          label="Capture feedback"
          description="Briefly confirms successful captures without taking focus from the current app."
          action={<SettingsSwitch
            checked={settings.captureFeedback}
            label="Capture feedback"
            onClick={() => onUpdateSettings({ captureFeedback: !settings.captureFeedback })}
          />}
        />
        <SettingRow
          disabled={!settings.captureFeedback}
          label="Show skipped captures"
          description="Also acknowledge clipboard items intentionally left alone."
          action={<SettingsSwitch
            checked={settings.captureFeedbackIgnored}
            label="Show skipped captures"
            disabled={!settings.captureFeedback}
            onClick={() => onUpdateSettings({ captureFeedbackIgnored: !settings.captureFeedbackIgnored })}
          />}
        />
        <SettingRow
          disabled={!settings.captureFeedback}
          label="Show clip preview"
          description="Show the captured item with quick actions."
          action={<SettingsSwitch
            checked={settings.captureFeedbackPreview}
            label="Show clip preview"
            disabled={!settings.captureFeedback}
            onClick={() => onUpdateSettings({ captureFeedbackPreview: !settings.captureFeedbackPreview })}
          />}
        />
        <SettingRow
          disabled={!settings.captureFeedback || !settings.captureFeedbackPreview}
          label="Dismiss preview after"
          description="The countdown pauses while the pointer is over a preview."
          action={<MenuSelect
            value={String(settings.captureFeedbackDismissSeconds)}
            options={DISMISS_OPTIONS}
            disabled={!settings.captureFeedback || !settings.captureFeedbackPreview}
            onChange={(value) => onUpdateSettings({ captureFeedbackDismissSeconds: Number(value) })}
            label="Preview dismissal delay"
            className="settings-menu-select"
          />}
        />
        <SettingRow
          label="Screen position"
          description="Uses this corner on whichever display currently contains the pointer."
          action={<MenuSelect
            value={settings.captureFeedbackPosition}
            options={POSITION_OPTIONS}
            onChange={(value) => onUpdateSettings({
              captureFeedbackPosition: value as AppSettings['captureFeedbackPosition'],
            })}
            label="Capture feedback position"
            className="settings-menu-select"
          />}
        />
      </div>
      <SettingsPanelNote>
        Capture feedback stays on-device and never exposes copied text, images, file names, or paths to system notifications.
      </SettingsPanelNote>
    </div>
  );
}
