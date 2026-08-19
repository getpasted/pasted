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
const clipSearch = read('src/utils/clipSearch.ts');
const clipSearchGrammar = read('src/utils/clipSearchGrammar.ts');
const historySearchDocs = read('docs/wiki/History-and-Search.md');
const database = read('src-tauri/src/db.rs');
const cli = read('src-tauri/src/bin/pasted_cli.rs');
const clipTypes = read('src/types.ts');
const appData = read('src/hooks/useAppData.ts');
const foundationCss = read('src/styles/foundation.css');

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
assert.match(sidebar, /useLocalization\(\)/, 'Memoized sidebar navigation must subscribe to locale changes');
assert.match(sidebar, /getClipCollection\('bin', b\)/, 'Bins must inherit collection capabilities in the sidebar');
assert.match(sidebar, /id: 'clipTypes'[\s\S]{0,500}id: 'types'/, 'Clip Types must appear before semantic Content Types');
assert.match(sidebar, /clipFacetRoute\('clip_type', value\)/, 'Clip Type navigation must use stable structural routes');
assert.match(sidebar, /clipFacetRoute\('content_type', value\)/, 'Content Type navigation must use stable calculated-collection routes');
assert.match(sidebar, /clipFacetRoute\('source', value\)/, 'Source navigation must use stable calculated-collection routes');
assert.match(read('src/hooks/useClipViews.ts'), /parseClipFacetRoute\(currentTab\)/, 'Type and Source views must share calculated collection filtering');
assert.match(sidebar, /missingSources[\s\S]*get_source_icons/, 'Source icons must request only newly observed applications');
assert.match(sidebar, /\[features\.sources, sourceIconSignature\]/, 'Clip count and ordering changes must not retrigger source icon extraction');
assert.match(sidebar, /sourceFallbackIcon\(item\.value\)/, 'Unresolvable system sources must retain semantic cross-platform icons');
assert.match(nativeCommands, /SOURCE_ICON_CACHE/, 'Resolved native application icons must be cached across frontend requests');
assert.match(nativeCommands, /pub async fn get_source_icons/, 'Native icon extraction must not block synchronous IPC dispatch');
assert.match(nativeCommands, /macos_application_icon_data_url[\s\S]{0,8000}spawn_blocking/, 'macOS icon conversion must stay off the UI thread');
assert.match(clipSearchGrammar, /startsWith\('source:'\)/, 'Source filtering must use the canonical source: search operator');
assert.match(clipSearchGrammar, /startsWith\('clip:'\)/, 'Clip Type filtering must use the canonical clip: search operator');
assert.match(clipSearchGrammar, /startsWith\('content:'\)/, 'Content Type filtering must use the canonical content: search operator');
assert.match(clipSearchGrammar, /startsWith\('format:'\)/, 'File Format filtering must use the canonical format: search operator');
assert.doesNotMatch(clipSearchGrammar, /startsWith\('type:'\)/, 'The ambiguous type: search operator must not remain');
assert.doesNotMatch(clipSearchGrammar, /startsWith\('app:'\)/, 'The pre-1.0 app: search operator must not remain as an alias');
assert.match(clipSearchGrammar, /sources:\s*string\[\]/, 'Parsed search plans must expose canonical source terminology');
assert.doesNotMatch(clipSearchGrammar, /apps:\s*string\[\]/, 'Parsed search plans must not expose the removed app terminology');
assert.match(sidebar, /prefix:\s*'source:'/, 'Search helpers must advertise the canonical source: operator');
assert.match(sidebar, /prefix:\s*'clip:'/, 'Search helpers must advertise the canonical clip: operator');
assert.match(sidebar, /prefix:\s*'content:'/, 'Search helpers must advertise the canonical content: operator');
assert.match(sidebar, /prefix:\s*'format:'/, 'Search helpers must advertise the canonical format: operator');
assert.doesNotMatch(sidebar, /prefix:\s*'type:'/, 'Search helpers must not advertise the ambiguous type: operator');
assert.match(historySearchDocs, /`source:` — capture source/, 'Search documentation must use the canonical source: operator');
assert.match(historySearchDocs, /`clip:` — structural Clip Type/, 'Search documentation must describe structural Clip Type filtering');
assert.match(historySearchDocs, /`content:` — current Content Type/, 'Search documentation must describe semantic Content Type filtering');
assert.match(historySearchDocs, /`format:` — verified File Format/, 'Search documentation must describe verified File Format filtering');
assert.doesNotMatch(historySearchDocs, /`app:`/, 'Search documentation must not advertise the removed pre-1.0 app: operator');
assert.match(database, /pub source: String/, 'The shared clip contract must expose canonical source terminology');
assert.match(database, /ALTER TABLE clips RENAME COLUMN source_app TO source/, 'Existing RC libraries must migrate source_app invisibly');
assert.match(clipTypes, /source:\s*string;/, 'Frontend clip contracts must expose the canonical source field');
assert.doesNotMatch(clipTypes, /source_app/, 'Frontend clip contracts must not retain the pre-1.0 source_app field');
assert.match(cli, /"source": source/, 'CLI structured search output must expose the canonical source field');
assert.match(database, /id,content_type,source,is_pinned/, 'CSV exports must expose the canonical source header');
assert.match(appData, /record\.source_app[\s\S]*source_app:\s*_legacySource/, 'Pre-1.0 cached and IPC clip summaries must migrate source_app without retaining it');
assert.match(sidebar, /source\?\.trim\(\)\.toLowerCase\(\)\s*\?\?\s*''/, 'Source icon rendering must tolerate stale or incomplete cached metadata');
assert.match(clipViews, /getClipCollection\(currentTab, selectedBin\)/, 'Clip filtering must resolve the active collection');
assert.match(clipViews, /facet\?\.kind === 'clip_type'[\s\S]{0,160}clip\.content_type === facet\.value/, 'Clip Type routes must filter structural identity only');
assert.match(clipViews, /facet\?\.kind === 'content_type'[\s\S]{0,180}clip\.content_types/, 'Content Type routes must filter Classifier results only');
assert.match(emptyState, /collection\?\.emptyTitle/, 'Empty states must come from the collection descriptor');
assert.match(viewPolicy, /collection\?\.membership/, 'Interaction policy must use collection membership');
assert.match(app, /currentCollection\?\.title/, 'The clip-list heading must use the collection descriptor');
assert.match(app, /currentTab\.startsWith\('clip_type-'\)[\s\S]{0,180}currentTab\.startsWith\('file_format-'\)/, 'Search escape must remember every collection-axis route');
assert.match(app, /\[bins, currentTab, locale, selectedBinId\]/, 'The active collection heading must recompute when the locale changes');
assert.doesNotMatch(dragHook, /export type ClipDropAction/, 'Drop actions must be owned by the collection contract');
assert.match(database, /pub fn get_clips_page[\s\S]*LIMIT \? OFFSET \?/, 'Active clips must support bounded server pagination');
assert.match(database, /pub fn get_trashed_clips_page[\s\S]*LIMIT \? OFFSET \?/, 'Trash must support bounded server pagination');
assert.match(database, /pub fn get_clip_collection_summary/, 'Sidebar collection counts must come from an exact server summary');
assert.match(appData, /const CLIP_PAGE_SIZE = 250;/, 'The GUI must fetch clip collections in bounded pages');
assert.match(appData, /loadMoreClips[\s\S]*loadMoreTrashedClips/, 'Active clips and Trash must both support incremental loading');
for (const field of ['clipTypeCounts', 'typeCounts', 'sourceCounts']) {
  assert.match(sidebar, new RegExp(`clipCollectionSummary\\.${field}`), `${field} badges must come from the exact server summary`);
}
assert.match(foundationCss, /\.clip-card[\s\S]*content-visibility:\s*auto/, 'Offscreen clip cards must retain browser-native rendering virtualization');

console.log('Clip collection contract audit passed.');
