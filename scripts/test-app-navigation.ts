import assert from 'node:assert/strict';
import { resolveAppNavigationTarget } from '../src/utils/appNavigation.ts';

assert.deepEqual(resolveAppNavigationTarget('settings:security'), {
  tab: 'settings',
  settingsTab: 'security',
});
assert.deepEqual(resolveAppNavigationTarget('settings:search-history'), {
  tab: 'settings',
  settingsTab: 'search-history',
});
assert.deepEqual(resolveAppNavigationTarget('help:privacy-capture'), {
  tab: 'help',
  helpTopic: 'privacy-capture',
});
assert.deepEqual(resolveAppNavigationTarget('transformations:playground'), {
  tab: 'transformations',
  transformWorkspace: 'playground',
});
assert.deepEqual(resolveAppNavigationTarget('settings:not-a-tab'), { tab: 'settings' });
assert.deepEqual(resolveAppNavigationTarget('search'), { tab: 'search' });

console.log('App navigation controller tests passed.');
