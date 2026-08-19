import { translate } from '../localization/runtime';
import { errorMessage } from './errors';
import { appLockAuthErrorKey } from './appLockPolicy';

export function appLockAuthErrorMessage(reason: unknown): string {
  const detail = errorMessage(reason).replace(/^Error:\s*/, '');
  const key = appLockAuthErrorKey(detail);
  return key ? translate(key) : detail;
}
