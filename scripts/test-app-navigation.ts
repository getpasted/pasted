import assert from 'node:assert/strict';
import {
  isClipCollectionRoute,
  resolveAppNavigationTarget,
  resolveSearchExit,
} from '../src/utils/appNavigation.ts';

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

for (const route of [
  'all', 'sequential', 'pinned', 'protected', 'notes', 'trash', 'bin',
  'clip_type-text', 'content_type-code', 'file_format-json', 'source-Finder',
]) {
  assert.equal(isClipCollectionRoute(route), true, `${route} should be remembered as a clip view`);
}
for (const route of ['search', 'settings', 'help', 'analytics', 'transformations', 'activity']) {
  assert.equal(isClipCollectionRoute(route), false, `${route} should not replace the previous clip view`);
}

assert.deepEqual(resolveSearchExit({ tab: 'bin', binId: 7 }, new Set([7])), { tab: 'bin', binId: 7 });
assert.deepEqual(resolveSearchExit({ tab: 'bin', binId: 7 }, new Set([8])), { tab: 'all', binId: null });
assert.deepEqual(resolveSearchExit({ tab: 'protected', binId: null }, new Set()), {
  tab: 'protected',
  binId: null,
});

console.log('App navigation controller tests passed.');
