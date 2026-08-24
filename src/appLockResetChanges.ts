import type { AppLockStatus } from './hooks/useAppLock';
import { translate } from './localization/runtime';
import { resetBooleanLabel, type SettingsResetChange } from './components/SettingsResetChanges';

export const DEFAULT_APP_LOCK_POLICY = {
  systemAuthEnabled: false,
  appleWatchEnabled: false,
  idleMinutes: 5,
  lockOnSleep: true,
  lockOnRestart: true,
  captureWhileLocked: true,
} as const;

export function appLockResetChanges(status: AppLockStatus): SettingsResetChange[] {
  const changes = [
    { label: translate('component.settingsSecurityPanel.unlockUsingMethod', { method: status.systemAuthLabel }), before: resetBooleanLabel(status.systemAuthEnabled), after: resetBooleanLabel(DEFAULT_APP_LOCK_POLICY.systemAuthEnabled) },
    { label: translate('component.settingsSecurityPanel.unlockUsingAppleWatch'), before: resetBooleanLabel(status.appleWatchEnabled), after: resetBooleanLabel(DEFAULT_APP_LOCK_POLICY.appleWatchEnabled) },
    { label: translate('component.settingsSecurityPanel.lockAfterRestart'), before: resetBooleanLabel(status.lockOnRestart), after: resetBooleanLabel(DEFAULT_APP_LOCK_POLICY.lockOnRestart) },
    { label: translate('component.settingsSecurityPanel.lockWhenTheDeviceSleeps'), before: resetBooleanLabel(status.lockOnSleep), after: resetBooleanLabel(DEFAULT_APP_LOCK_POLICY.lockOnSleep) },
    { label: translate('component.settingsSecurityPanel.lockAfterInactivity'), before: idleLabel(status.idleMinutes), after: idleLabel(DEFAULT_APP_LOCK_POLICY.idleMinutes) },
    { label: translate('component.settingsSecurityPanel.captureWhileLocked'), before: resetBooleanLabel(status.captureWhileLocked), after: resetBooleanLabel(DEFAULT_APP_LOCK_POLICY.captureWhileLocked) },
  ];
  return changes.filter(({ before, after }) => before !== after);
}

function idleLabel(minutes: number) {
  if (minutes === 0) return translate('component.settingsSecurityPanel.never');
  if (minutes === 1) return translate('component.settingsSecurityPanel.value1Minute');
  if (minutes === 5) return translate('component.settingsSecurityPanel.value5Minutes');
  if (minutes === 60) return translate('component.settingsSecurityPanel.value1Hour');
  if (minutes === 480) return translate('component.settingsSecurityPanel.value8Hours');
  return translate('component.settingsResetChanges.minutes', { count: minutes });
}
