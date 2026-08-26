import assert from 'node:assert/strict';
import {
  APP_UI_STATE_KEY,
  DEFAULT_APP_UI_STATE,
  parseAppUiState,
  resetPastedClientStorage,
} from '../src/utils/appUiStateCodec.ts';
import { parseScrollPositionState } from '../src/utils/scrollPositionState.ts';

const restored = parseAppUiState({
  version: 2,
  currentTab: 'settings',
  settingsTab: 'analysis',
  helpTopic: 'deletion-recovery',
  transformWorkspace: 'playground',
  selectedClipId: 42,
  isSidebarCollapsed: true,
  sidebarSections: { clips: false, bins: true, types: false, sources: true, tools: false },
});
assert.equal(restored.currentTab, 'settings');
assert.equal(restored.settingsTab, 'analysis');
assert.equal(restored.helpTopic, 'deletion-recovery');
assert.equal(restored.transformWorkspace, 'playground');
assert.equal(restored.selectedClipId, 42);
assert.equal(restored.isSidebarCollapsed, true);
assert.equal(restored.sidebarSections.types, false);
assert.equal(restored.sidebarSections.clipTypes, true);

const legacy = parseAppUiState({ version: 1, currentTab: 'help' });
assert.equal(legacy.currentTab, 'help');
assert.equal(legacy.settingsTab, 'general');
assert.equal(legacy.helpTopic, 'getting-started');
assert.equal(legacy.transformWorkspace, 'transforms');

const invalid = parseAppUiState({
  currentTab: 'settings:security',
  settingsTab: 'hidden',
  helpTopic: '../outside',
  transformWorkspace: 'unknown',
});
assert.equal(invalid.currentTab, 'all');
assert.equal(invalid.settingsTab, 'general');
assert.equal(invalid.helpTopic, 'getting-started');
assert.equal(invalid.transformWorkspace, 'transforms');

const invalidBin = parseAppUiState({ currentTab: 'bin', selectedBinId: -1 });
assert.equal(invalidBin.currentTab, 'all');
assert.equal(invalidBin.selectedBinId, null);

assert.equal(parseAppUiState({ currentTab: 'content_type-email' }).currentTab, 'content_type-email');
assert.equal(parseAppUiState({ currentTab: 'file_format-pdf' }).currentTab, 'file_format-pdf');
assert.equal(parseAppUiState({ currentTab: 'type-email' }).currentTab, 'all');

const storedValues = new Map<string, string>([
  [APP_UI_STATE_KEY, JSON.stringify({ currentTab: 'settings', settingsTab: 'storage' })],
  ['pasted_sidebar_width', '330'],
  ['unrelated_state', 'preserved'],
]);
resetPastedClientStorage({
  get length() { return storedValues.size; },
  key: (index) => [...storedValues.keys()][index] ?? null,
  removeItem: (key) => { storedValues.delete(key); },
  setItem: (key, value) => { storedValues.set(key, value); },
});
assert.deepEqual(JSON.parse(storedValues.get(APP_UI_STATE_KEY) ?? ''), DEFAULT_APP_UI_STATE);
assert.equal(storedValues.has('pasted_sidebar_width'), false);
assert.equal(storedValues.get('unrelated_state'), 'preserved');

const scrollState = parseScrollPositionState({
  version: 1,
  positions: {
    'clips:section:all': { scrollTop: 480, anchorClipId: 42, anchorOffset: -12 },
    'settings:storage': { scrollTop: 920 },
    invalid: { scrollTop: 'far' },
  },
});
assert.deepEqual(scrollState.positions['clips:section:all'], {
  scrollTop: 480,
  anchorClipId: 42,
  anchorOffset: -12,
});
assert.equal(scrollState.positions['settings:storage'].scrollTop, 920);
assert.equal(scrollState.positions.invalid, undefined);
assert.equal(parseScrollPositionState({ version: 2, positions: {} }).version, 1);

console.log('App UI state route and subpage tests passed.');
