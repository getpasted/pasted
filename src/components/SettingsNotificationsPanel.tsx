import { useState, type ReactNode } from 'react';
import { Bell } from 'lucide-react';
import type { AppSettings } from '../types';
import { MenuSelect } from './MenuSelect';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsSwitch } from './SettingsSwitch';
import { translate } from '../localization/runtime';
import { ConfirmationDialog, type ConfirmationDialogRequest } from './ConfirmationDialog';
import { SettingsPanelResetNote } from './SettingsPanelResetNote';
import { SettingsResetChanges } from './SettingsResetChanges';
import { DISMISS_OPTIONS, notificationResetChanges, POSITION_OPTIONS } from '../notificationSettingsModel';

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

export function SettingsNotificationsPanel({ settings, onUpdateSettings }: SettingsNotificationsPanelProps) {
  const [confirmation, setConfirmation] = useState<ConfirmationDialogRequest | null>(null);
  const requestReset = () => {
    const { changes, defaults } = notificationResetChanges(settings);
    setConfirmation({
      title: translate('component.settingsNotificationsPanel.resetNotifications'),
      description: translate('component.settingsResetChanges.description'),
      details: <SettingsResetChanges changes={changes} />,
      confirmLabel: translate('common.reset'),
      confirmDisabled: changes.length === 0,
      onConfirm: () => {
        onUpdateSettings(defaults);
        setConfirmation(null);
      },
    });
  };

  return (
    <div className="space-y-5 text-xs">
      <SettingsPanelHeader
        icon={Bell}
        title={translate('component.settingsNotificationsPanel.notifications')}
        description={translate('component.settingsNotificationsPanel.getAQuietConfirmationWhenClipboardCaptureSucceedsOrFails')}
      />

      <div className="space-y-4">
        <SettingRow
          label={translate('component.settingsNotificationsPanel.captureFeedback')}
          description={translate('component.settingsNotificationsPanel.brieflyConfirmsSuccessfulCapturesWithoutTakingFocusFromTheCurrentApp')}
          action={<SettingsSwitch
            checked={settings.captureFeedback}
            label={translate('component.settingsNotificationsPanel.captureFeedback')}
            onClick={() => onUpdateSettings({ captureFeedback: !settings.captureFeedback })}
          />}
        />
        <SettingRow
          disabled={!settings.captureFeedback}
          label={translate('component.settingsNotificationsPanel.showSkippedCaptures')}
          description={translate('component.settingsNotificationsPanel.alsoAcknowledgeClipboardItemsIntentionallyLeftAlone')}
          action={<SettingsSwitch
            checked={settings.captureFeedbackIgnored}
            label={translate('component.settingsNotificationsPanel.showSkippedCaptures')}
            disabled={!settings.captureFeedback}
            onClick={() => onUpdateSettings({ captureFeedbackIgnored: !settings.captureFeedbackIgnored })}
          />}
        />
        <SettingRow
          disabled={!settings.captureFeedback}
          label={translate('component.settingsNotificationsPanel.showClipPreview')}
          description={translate('component.settingsNotificationsPanel.showTheCapturedItemWithQuickActions')}
          action={<SettingsSwitch
            checked={settings.captureFeedbackPreview}
            label={translate('component.settingsNotificationsPanel.showClipPreview')}
            disabled={!settings.captureFeedback}
            onClick={() => onUpdateSettings({ captureFeedbackPreview: !settings.captureFeedbackPreview })}
          />}
        />
        <SettingRow
          disabled={!settings.captureFeedback || !settings.captureFeedbackPreview}
          label={translate('component.settingsNotificationsPanel.dismissPreviewAfter')}
          description={translate('component.settingsNotificationsPanel.theCountdownPausesWhileThePointerIsOverAPreview')}
          action={<MenuSelect
            value={String(settings.captureFeedbackDismissSeconds)}
            options={DISMISS_OPTIONS}
            disabled={!settings.captureFeedback || !settings.captureFeedbackPreview}
            onChange={(value) => onUpdateSettings({ captureFeedbackDismissSeconds: Number(value) })}
            label={translate('component.settingsNotificationsPanel.previewDismissalDelay')}
            className="settings-menu-select"
          />}
        />
        <SettingRow
          label={translate('component.settingsNotificationsPanel.screenPosition')}
          description={translate('component.settingsNotificationsPanel.usesThisCornerOnWhicheverDisplayCurrentlyContainsThePointer')}
          action={<MenuSelect
            value={settings.captureFeedbackPosition}
            options={POSITION_OPTIONS}
            onChange={(value) => onUpdateSettings({
              captureFeedbackPosition: value as AppSettings['captureFeedbackPosition'],
            })}
            label={translate('component.settingsNotificationsPanel.captureFeedbackPosition')}
            className="settings-menu-select"
          />}
        />
      </div>
      <SettingsPanelResetNote onReset={requestReset}>
        {translate('component.settingsNotificationsPanel.captureFeedbackStaysOnDeviceAndNeverExposesCopiedTextImagesFile')}
      </SettingsPanelResetNote>
      <ConfirmationDialog request={confirmation} onCancel={() => setConfirmation(null)} />
    </div>
  );
}
