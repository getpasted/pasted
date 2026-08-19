export function appLockAuthErrorKey(detail: string) {
  switch (detail) {
    case 'app_lock_auth_watch_unavailable':
      return 'component.appLockScreen.appleWatchIsNotAvailable' as const;
    case 'app_lock_auth_watch_failed':
      return 'component.appLockScreen.appleWatchAuthenticationFailed' as const;
    case 'app_lock_auth_timeout':
      return 'component.appLockScreen.systemAuthenticationTimedOut' as const;
    case 'app_lock_auth_failed':
      return 'component.appLockScreen.systemAuthenticationCouldNotBeCompleted' as const;
    default:
      return null;
  }
}

export function authToggleDisabled({
  pending,
  appLockEnabled,
  methodConfigured,
  methodAvailable,
}: {
  pending: boolean;
  appLockEnabled: boolean;
  methodConfigured: boolean;
  methodAvailable: boolean;
}): boolean {
  return pending || !appLockEnabled || (!methodConfigured && !methodAvailable);
}
