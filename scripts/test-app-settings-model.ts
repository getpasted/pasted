import assert from 'node:assert/strict';
import { welcomeSetupSteps, visibleWelcomeStep } from '../src/components/welcomeSetupModel.ts';
import { hasStorageFeatures } from '../src/utils/features.ts';
import {
  DEFAULT_ANALYSIS_ATTEMPTS_PER_CLIP,
  DEFAULT_REVISION_HISTORY_LIMIT,
  isAnalysisFunctionalityEnabled,
  storedRetentionNumber,
} from '../src/appSettingsRetentionModel.ts';
import { storedSearchHistoryAgeDays } from '../src/searchHistoryRetention.ts';
import { DEFAULT_PRIVATE_BROWSER_SETTINGS, savedCapturePolicySettings } from '../src/appSettingsCapturePolicyModel.ts';
import { defaultAppExclusions, normalizeAppExclusions } from '../src/appExclusionModel.ts';
import { notificationDefaultUpdates } from '../src/appSettingsSectionDefaults.ts';
import {
  DEFAULT_SNAPSHOT_INTERVAL_MINUTES,
  DEFAULT_SNAPSHOT_KEEP_COUNT,
  storedSnapshotIntervalMinutes,
  storedSnapshotKeepCount,
} from '../src/appSettingsStorageModel.ts';

assert.equal(DEFAULT_REVISION_HISTORY_LIMIT, 10);
assert.equal(DEFAULT_ANALYSIS_ATTEMPTS_PER_CLIP, 10);
assert.equal(storedRetentionNumber({}, 'analysisAttemptsPerClip', DEFAULT_ANALYSIS_ATTEMPTS_PER_CLIP), 10);
assert.equal(storedRetentionNumber({ analysisAttemptsPerClip: '25' }, 'analysisAttemptsPerClip', 10), 25,
  'an existing configured Analysis limit must be preserved');
assert.equal(isAnalysisFunctionalityEnabled({ enableOcr: false, enableTranscriptions: false }), false);
assert.equal(isAnalysisFunctionalityEnabled({ enableOcr: true, enableTranscriptions: false }), true);
assert.equal(isAnalysisFunctionalityEnabled({ enableOcr: false, enableTranscriptions: true }), true);
assert.equal(storedSearchHistoryAgeDays({}, 0), 0);
assert.equal(storedSearchHistoryAgeDays({ searchHistoryAgeDays: '365' }, 0), 365);
assert.equal(storedSearchHistoryAgeDays({ searchHistoryAgeDays: '-1' }, 0), 0);
assert.equal(storedSearchHistoryAgeDays({ searchHistoryAgeDays: '50000' }, 0), 36_500);
assert.deepEqual(savedCapturePolicySettings({}), {
  alwaysPastePlainText: false,
  excludePrivateBrowserWindows: false,
  privateBrowserUnavailablePolicy: 'capture',
});
assert.equal(savedCapturePolicySettings({ excludePrivateBrowserWindows: 'true' }).excludePrivateBrowserWindows, true);
assert.equal(savedCapturePolicySettings({ privateBrowserUnavailablePolicy: 'exclude_browser' }).privateBrowserUnavailablePolicy, 'exclude_browser');
assert.equal(savedCapturePolicySettings({ privateBrowserUnavailablePolicy: 'guess' }).privateBrowserUnavailablePolicy, 'capture');
assert.deepEqual(notificationDefaultUpdates(), {
  captureFeedback: true,
  captureFeedbackIgnored: false,
  captureFeedbackPreview: false,
  captureFeedbackPosition: 'top-right',
  captureFeedbackDismissSeconds: 7,
});
assert.deepEqual(DEFAULT_PRIVATE_BROWSER_SETTINGS, {
  excludePrivateBrowserWindows: false,
  privateBrowserUnavailablePolicy: 'capture',
});
assert.equal(DEFAULT_SNAPSHOT_INTERVAL_MINUTES, 1440);
assert.equal(DEFAULT_SNAPSHOT_KEEP_COUNT, 5);
assert.equal(storedSnapshotIntervalMinutes({ snapshotIntervalMinutes: '15' }), 15);
assert.equal(storedSnapshotIntervalMinutes({ snapshotIntervalMinutes: '-1' }), 1);
assert.equal(storedSnapshotKeepCount({ snapshotKeepCount: '6' }), 6);
assert.equal(storedSnapshotKeepCount({ snapshotKeepCount: '20000' }), 10000);
assert.deepEqual(defaultAppExclusions().map(({ name }) => name), [
  '1Password', 'Passwords', 'Keychain Access', 'Bitwarden', 'Dashlane', 'Enpass', 'KeePassXC',
]);
assert.ok(defaultAppExclusions().every((app) => (
  app.ignoreText && app.ignoreImages && app.ignoreFiles && !app.ignoreHotkeys
)));
assert.deepEqual(normalizeAppExclusions([]), [], 'an intentional empty exclusion list must remain empty');
console.log('App settings model tests passed.');

for (let mask = 0; mask < 16; mask += 1) {
  assert.equal(hasStorageFeatures({
    enableLibraryMove: Boolean(mask & 1), enableBackups: Boolean(mask & 2),
    enableSnapshots: Boolean(mask & 4), enableFactoryReset: Boolean(mask & 8),
  }), mask !== 0, 'Storage is visible when any one of its capabilities is enabled');
}

for (const enableBackups of [false, true]) {
  for (const enableHotkeys of [false, true]) {
    const features = { enableBackups, enableHotkeys };
    const steps = welcomeSetupSteps(features);
    assert.equal(steps.includes('migration'), enableBackups);
    assert.equal(steps.includes('hotkey'), enableHotkeys);
    assert.equal(steps.length, 3 + Number(enableBackups) + Number(enableHotkeys));
    for (const step of ['welcome', 'migration', 'privacy', 'hotkey', 'ready'] as const) {
      assert.ok(steps.includes(visibleWelcomeStep(step, features)), 'active steps always belong to the visible sequence');
    }
    assert.equal(steps[steps.indexOf('welcome') + 1], enableBackups ? 'migration' : 'privacy');
    assert.equal(steps[steps.indexOf('privacy') - 1], enableBackups ? 'migration' : 'welcome');
  }
}
