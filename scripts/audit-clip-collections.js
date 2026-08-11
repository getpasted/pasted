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
const nativeCommands = read('src-tauri/src/commands.rs');

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
assert.match(sidebar, /clipFacetRoute\('type', value\)/, 'Type navigation must use stable calculated-collection routes');
assert.match(sidebar, /clipFacetRoute\('source', value\)/, 'Source navigation must use stable calculated-collection routes');
assert.match(read('src/hooks/useClipViews.ts'), /parseClipFacetRoute\(currentTab\)/, 'Type and Source views must share calculated collection filtering');
assert.match(sidebar, /missingSources[\s\S]*get_source_app_icons/, 'Source icons must request only newly observed applications');
assert.match(sidebar, /\[features\.sources, sourceIconSignature\]/, 'Clip count and ordering changes must not retrigger source icon extraction');
assert.match(sidebar, /sourceFallbackIcon\(item\.value\)/, 'Unresolvable system sources must retain semantic cross-platform icons');
assert.match(nativeCommands, /SOURCE_APP_ICON_CACHE/, 'Resolved native application icons must be cached across frontend requests');
assert.match(nativeCommands, /pub async fn get_source_app_icons/, 'Native icon extraction must not block synchronous IPC dispatch');
assert.match(nativeCommands, /macos_application_icon_data_url[\s\S]{0,8000}spawn_blocking/, 'macOS icon conversion must stay off the UI thread');
assert.match(clipViews, /getClipCollection\(currentTab, selectedBin\)/, 'Clip filtering must resolve the active collection');
assert.match(emptyState, /collection\?\.emptyTitle/, 'Empty states must come from the collection descriptor');
assert.match(viewPolicy, /collection\?\.membership/, 'Interaction policy must use collection membership');
assert.match(app, /currentCollection\?\.title/, 'The clip-list heading must use the collection descriptor');
assert.doesNotMatch(dragHook, /export type ClipDropAction/, 'Drop actions must be owned by the collection contract');

console.log('Clip collection contract audit passed.');
