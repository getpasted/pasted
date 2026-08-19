import assert from 'node:assert/strict';
import fs from 'node:fs';
import { APP_LOCK_ACTIVITY_EVENTS, createIdleDeadline } from '../src/utils/idleDeadline.ts';
import { appLockAuthErrorKey, authToggleDisabled } from '../src/utils/appLockPolicy.ts';

const rootSource = fs.readFileSync('src/main.tsx', 'utf8');
const hudWindowSource = fs.readFileSync('src-tauri/src/hud_window.rs', 'utf8');
const commandsSource = fs.readFileSync('src-tauri/src/commands.rs', 'utf8');
const hotkeySource = fs.readFileSync('src-tauri/src/hotkey_manager.rs', 'utf8');
const liveAppSource = fs.readFileSync('src-tauri/src/live_app.rs', 'utf8');
const nativeRootSource = fs.readFileSync('src-tauri/src/lib.rs', 'utf8');

assert.match(rootSource, /rootView === "hud" && appLock\.status\.locked\) return null/,
  'the HUD webview must never render the app-lock modal');
assert.match(rootSource, /useAppLock\(\{ animateUnlock: rootView !== "hud" \}\)/,
  'the HUD must accept unlock state immediately instead of waiting for the main-window animation');
assert.match(rootSource, /useAppLock\(\{ animateUnlock: false \}\)/,
  'capture feedback must not wait for a lock-screen animation that its window never renders');
assert.match(hudWindowSource, /pub fn require_unlocked[\s\S]*?state\.is_locked\(\)[\s\S]*?hide\(app\)[\s\S]*?Pasted is locked\./,
  'the shared HUD window boundary must reject and hide locked invocations');
assert.match(hudWindowSource, /pub fn reveal[\s\S]*?app_lock::status[\s\S]*?emit\("app-lock-changed", &lock_status\)[\s\S]*?window\.show\(\)/,
  'every HUD reveal must synchronize current lock state before showing the hidden webview');
assert.match(commandsSource, /lock_app_with_state[\s\S]*?hud_window::hide\(app\)/,
  'locking from the GUI must hide the HUD');
assert.match(hotkeySource, /app_lock::lock_enabled[\s\S]*?hud_window::hide\(app\)/,
  'locking from a hotkey must hide the HUD');
assert.match(liveAppSource, /LiveAppAction::AppLockLock[\s\S]*?hud_window::hide\(app\)/,
  'locking from the CLI live-app path must hide the HUD');
assert.match(nativeRootSource, /RunEvent::Resumed[\s\S]*?state\.lock\(\);[\s\S]*?hud_window::hide\(app\)/,
  'locking after system sleep must hide the HUD');

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
