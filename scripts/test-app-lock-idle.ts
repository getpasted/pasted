import assert from 'node:assert/strict';
import { APP_LOCK_ACTIVITY_EVENTS, createIdleDeadline } from '../src/utils/idleDeadline.ts';
import { appLockAuthErrorKey, authToggleDisabled } from '../src/utils/appLockPolicy.ts';

let now = 0;
let nextTimer = 1;
let scheduleCount = 0;
const timers = new Map<number, { callback: () => void; dueAt: number }>();

const schedule = (callback: () => void, delayMs: number) => {
  const timer = nextTimer++;
  scheduleCount += 1;
  timers.set(timer, { callback, dueAt: now + delayMs });
  return timer;
};

const cancel = (timer: number) => {
  timers.delete(timer);
};

const advanceTo = (target: number) => {
  while (true) {
    const pending = [...timers.entries()]
      .filter(([, value]) => value.dueAt <= target)
      .sort((left, right) => left[1].dueAt - right[1].dueAt)[0];
    if (!pending) break;
    const [timer, value] = pending;
    timers.delete(timer);
    now = value.dueAt;
    value.callback();
  }
  now = target;
};

assert(APP_LOCK_ACTIVITY_EVENTS.includes('pointermove'));
assert(APP_LOCK_ACTIVITY_EVENTS.includes('resize'));
assert.equal(authToggleDisabled({ pending: false, appLockEnabled: true, methodConfigured: true, methodAvailable: false }), false,
  'an unavailable configured method must remain possible to disable');
assert.equal(authToggleDisabled({ pending: false, appLockEnabled: true, methodConfigured: false, methodAvailable: false }), true,
  'an unavailable unconfigured method must not be enabled');
assert.equal(appLockAuthErrorKey('app_lock_auth_watch_unavailable'), 'component.appLockScreen.appleWatchIsNotAvailable');
assert.equal(appLockAuthErrorKey('app_lock_auth_watch_failed'), 'component.appLockScreen.appleWatchAuthenticationFailed');
assert.equal(appLockAuthErrorKey('unknown_error'), null, 'unknown native errors must remain available to existing error handling');

let elapsedCount = 0;
const deadline = createIdleDeadline({
  delayMs: 1_000,
  onElapsed: () => { elapsedCount += 1; },
  now: () => now,
  schedule,
  cancel,
});

advanceTo(900);
deadline.markActivity();
deadline.markActivity();
assert.equal(scheduleCount, 1, 'activity should not churn the active timer');

advanceTo(1_000);
assert.equal(elapsedCount, 0);
assert.equal(scheduleCount, 2, 'the original timer should reschedule only after it wakes');

advanceTo(1_899);
assert.equal(elapsedCount, 0);
advanceTo(1_900);
assert.equal(elapsedCount, 1);

deadline.markActivity();
advanceTo(3_000);
assert.equal(elapsedCount, 1, 'an elapsed deadline should fire only once');

let disposedElapsed = false;
const disposedDeadline = createIdleDeadline({
  delayMs: 1_000,
  onElapsed: () => { disposedElapsed = true; },
  now: () => now,
  schedule,
  cancel,
});
disposedDeadline.dispose();
advanceTo(4_000);
assert.equal(disposedElapsed, false);

console.log('App Lock idle deadline checks passed.');
