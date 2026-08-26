import assert from 'node:assert/strict';
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
assert.deepEqual(defaultAppExclusions().map(({ name }) => name), [
  '1Password', 'Passwords', 'Keychain Access', 'Bitwarden', 'Dashlane', 'Enpass', 'KeePassXC',
]);
assert.ok(defaultAppExclusions().every((app) => (
  app.ignoreText && app.ignoreImages && app.ignoreFiles && !app.ignoreHotkeys
)));
assert.deepEqual(normalizeAppExclusions([]), [], 'an intentional empty exclusion list must remain empty');
console.log('App settings model tests passed.');
