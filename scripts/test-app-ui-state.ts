import assert from 'node:assert/strict';
import { parseAppUiState } from '../src/utils/appUiStateCodec.ts';

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

console.log('App UI state route and subpage tests passed.');
