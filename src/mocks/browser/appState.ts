import { handled, unhandled, type BrowserMockResult } from './result';

const platformDescription = typeof navigator === 'undefined'
  ? ''
  : `${navigator.platform} ${navigator.userAgent}`;
const systemAuthLabel = /Mac|iPhone|iPad/i.test(platformDescription)
  ? 'Touch ID'
  : /Win/i.test(platformDescription)
    ? 'Windows Hello'
    : 'System authentication';

let appLockStatus = {
  enabled: false,
  locked: false,
  systemAuthEnabled: false,
  systemAuthAvailable: false,
  systemAuthLabel,
  appleWatchEnabled: false,
  appleWatchAvailable: false,
  idleMinutes: 5,
  lockOnSleep: true,
  lockOnRestart: true,
  captureWhileLocked: true,
};
let settings: Record<string, string> = {};
let clipboardPaused = false;

const lockSnapshot = () => ({ ...appLockStatus });

export function handleAppStateBrowserMock(
  command: string,
  args: Record<string, unknown> | undefined,
): BrowserMockResult {
  switch (command) {
    case 'is_clipboard_paused':
      return handled(clipboardPaused);
    case 'toggle_clipboard_pause':
      clipboardPaused = !clipboardPaused;
      return handled(clipboardPaused);
    case 'get_all_app_settings':
      return handled({ ...settings });
    case 'save_app_setting':
      settings[String(args?.key ?? '')] = String(args?.value ?? '');
      return handled(undefined);
    case 'save_app_settings':
    case 'register_app_setting_hotkeys': {
      const values = args?.values as Record<string, string> | undefined;
      if (values) settings = { ...settings, ...values };
      return handled(undefined);
    }
    case 'register_app_setting_hotkey':
    case 'register_hud_hotkey': {
      const key = String(args?.key ?? 'hudHotkey');
      settings[key] = String(args?.value ?? args?.hotkey ?? '');
      return handled(undefined);
    }
    case 'get_app_lock_status':
      return handled(lockSnapshot());
    case 'configure_app_lock':
      appLockStatus = { ...appLockStatus, enabled: true, locked: false };
      return handled(lockSnapshot());
    case 'disable_app_lock':
      appLockStatus = { ...appLockStatus, enabled: false, locked: false, systemAuthEnabled: false, appleWatchEnabled: false };
      return handled(lockSnapshot());
    case 'lock_app':
      appLockStatus = { ...appLockStatus, locked: true };
      return handled(lockSnapshot());
    case 'unlock_app':
      appLockStatus = { ...appLockStatus, locked: false };
      return handled(lockSnapshot());
    case 'set_app_lock_system_auth':
      appLockStatus = { ...appLockStatus, systemAuthEnabled: Boolean(args?.enabled) };
      return handled(lockSnapshot());
    case 'set_app_lock_apple_watch':
      appLockStatus = { ...appLockStatus, appleWatchEnabled: Boolean(args?.enabled) };
      return handled(lockSnapshot());
    case 'set_app_lock_idle_minutes':
      appLockStatus = { ...appLockStatus, idleMinutes: Number(args?.minutes ?? 5) };
      return handled(lockSnapshot());
    case 'set_app_lock_lock_on_sleep':
      appLockStatus = { ...appLockStatus, lockOnSleep: Boolean(args?.enabled) };
      return handled(lockSnapshot());
    case 'set_app_lock_lock_on_restart':
      appLockStatus = { ...appLockStatus, lockOnRestart: Boolean(args?.enabled) };
      return handled(lockSnapshot());
    case 'set_app_lock_capture_while_locked':
      appLockStatus = { ...appLockStatus, captureWhileLocked: Boolean(args?.enabled) };
      return handled(lockSnapshot());
    case 'reset_app_lock_policy':
      appLockStatus = {
        ...appLockStatus,
        systemAuthEnabled: false,
        appleWatchEnabled: false,
        idleMinutes: 5,
        lockOnSleep: true,
        lockOnRestart: true,
        captureWhileLocked: true,
      };
      return handled(lockSnapshot());
    default:
      return unhandled;
  }
}
