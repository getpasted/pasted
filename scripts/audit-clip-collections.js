import assert from 'node:assert/strict';
import fs from 'node:fs';

const read = (path) => fs.readFileSync(path, 'utf8');
const registry = read('src/utils/clipCollections.ts');
const sidebar = read('src/components/Sidebar.tsx');
const clipViews = read('src/hooks/useClipViews.ts');
const emptyState = read('src/components/EmptyClipList.tsx');
const viewPolicy = read('src/utils/clipViewPolicy.ts');
const app = read('src/App.tsx');
const dragHook = read('src/hooks/useClipBinDrag.ts');

for (const tab of ['all', 'sequential', 'pinned', 'protected', 'notes', 'trash']) {
  assert.match(registry, new RegExp(`tab:\\s*'${tab}'`), `${tab} must be registered as a system clip collection`);
}

for (const field of [
  'acceptsClipDrop',
  'canReorder',
  'allowsDuplicateMembership',
  'isCalculated',
  'isReadOnly',
  'emptyTitle',
  'emptyDescription',
]) {
  assert.match(registry, new RegExp(`\\b${field}\\b`), `The collection contract must retain ${field}`);
}

assert.match(sidebar, /getSystemClipCollections\(features\)/, 'Sidebar navigation must come from the shared collection registry');
assert.match(sidebar, /getClipCollection\('bin', b\)/, 'Bins must inherit collection capabilities in the sidebar');
assert.match(clipViews, /getClipCollection\(currentTab, selectedBin\)/, 'Clip filtering must resolve the active collection');
assert.match(emptyState, /collection\?\.emptyTitle/, 'Empty states must come from the collection descriptor');
assert.match(viewPolicy, /collection\?\.membership/, 'Interaction policy must use collection membership');
assert.match(app, /currentCollection\?\.title/, 'The clip-list heading must use the collection descriptor');
assert.doesNotMatch(dragHook, /export type ClipDropAction/, 'Drop actions must be owned by the collection contract');

console.log('Clip collection contract audit passed.');
