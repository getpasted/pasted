import assert from 'node:assert/strict';
import fs from 'node:fs';
import { readRustModuleTree } from './audit-source-trees.js';

const read = (path) => fs.readFileSync(path, 'utf8');
const registry = read('src/utils/clipCollections.ts');
const propertyAssociations = read('src/utils/clipPropertyAssociations.ts');
const sidebar = [
  'src/components/Sidebar.tsx',
  'src/components/CollapsedSidebar.tsx',
  'src/components/SidebarBinsSection.tsx',
  'src/components/SidebarClipSection.tsx',
  'src/components/SidebarFacetSections.tsx',
  'src/components/SidebarSearchFooter.tsx',
  'src/components/sidebarNavigationModel.tsx',
  'src/hooks/useSidebarFacets.ts',
].map(read).join('\n');
const clipViews = read('src/hooks/useClipViews.ts');
const searchPagination = read('src/utils/searchPagination.ts');
const clipsApi = read('src/api/clips.ts');
const emptyState = read('src/components/EmptyClipList.tsx');
const viewPolicy = read('src/utils/clipViewPolicy.ts');
const clipCard = [
  'src/components/ClipCard.tsx',
  'src/components/ClipCardActions.tsx',
  'src/components/clipCardModel.ts',
].map(read).join('\n');
const app = [
  read('src/App.tsx'),
  read('src/hooks/useAppController.ts'),
  read('src/hooks/useSettledSearchQuery.ts'),
  read('src/components/AppShellView.tsx'),
  read('src/components/ClipListContent.tsx'),
].join('\n');
const clipListHeader = read('src/components/ClipListHeader.tsx');
const appNavigation = read('src/utils/appNavigation.ts');
const dragHook = read('src/hooks/useClipBinDrag.ts');
const nativeCommands = read('src-tauri/src/commands/source_apps.rs');
const clipSearch = read('src/utils/clipSearch.ts');
const clipSearchGrammar = read('src/utils/clipSearchGrammar.ts');
const historySearchDocs = read('docs/wiki/History-and-Search.md');
const database = readRustModuleTree('src-tauri/src/db.rs', 'src-tauri/src/db');
const nativeClipSearch = read('src-tauri/src/db/clip_search.rs');
const nativeClipSearchTermFields = read('src-tauri/src/db/clip_search/term_fields.rs');
const cli = readRustModuleTree('src-tauri/src/bin/pasted.rs', 'src-tauri/src/cli');
const clipTypes = read('src/types.ts');
const appData = read('src/hooks/useAppData.ts');
const foundationCss = read('src/styles/foundation.css');

for (const tab of ['all', 'sequential', 'pinned', 'protected', 'notes', 'trash']) {
  assert.match(registry, new RegExp(`tab:\\s*'${tab}'`), `${tab} must be registered as a system clip collection`);
}

for (const [id, membership, action] of [
  ['pin', 'pinned', 'pin'],
  ['protect', 'protected', 'protect'],
  ['conceal', 'concealed', 'conceal'],
]) {
  assert.match(propertyAssociations, new RegExp(`id:\\s*'${id}'[\\s\\S]{0,100}membership:\\s*'${membership}'[\\s\\S]{0,100}dropAction:\\s*'${action}'`), `${membership} must use the shared clip-property association contract`);
  assert.match(registry, new RegExp(`association:\\s*'${id}'`), `${membership} collection must reference its property association`);
}
assert.match(propertyAssociations, /id:\s*'name'[\s\S]{0,100}membership:\s*'named'/, 'Named must use the shared clip-property association contract');
assert.match(registry, /association:\s*'name'/, 'Named collection must reference its property association');
assert.match(clipViews, /getClipPropertyAssociation\(collection\?\.association\)/, 'Property collection filtering must use the shared association contract');
assert.match(dragHook, /CLIP_PROPERTY_ASSOCIATIONS/, 'Property drop eligibility must use the shared association contract');
assert.match(registry, /key:\s*'system:queue'[\s\S]{0,300}acceptsClipDrop:\s*true[\s\S]{0,100}dropAction:\s*'queue'/,
  'Queued must remain a registered text Clip drop destination');
assert.match(dragHook, /content_type === 'file' \|\| !clip\.text_content[\s\S]{0,40}disabled\.push\('queue'\)/,
  'Queue drops must fail closed for Clips without a text payload');

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
assert.match(sidebar, /getClipCollection\('bin', (?:bin|b)\)/, 'Bins must inherit collection capabilities in the sidebar');
assert.match(sidebar, /id: 'clipTypes'[\s\S]{0,500}id: 'types'/, 'Clip Types must appear before semantic Content Types');
assert.match(sidebar, /clipFacetRoute\('clip_type', value\)/, 'Clip Type navigation must use stable structural routes');
assert.match(sidebar, /clipFacetRoute\('content_type', value\)/, 'Content Type navigation must use stable calculated-collection routes');
assert.match(sidebar, /clipFacetRoute\('source', value\)/, 'Source navigation must use stable calculated-collection routes');
assert.match(read('src/hooks/useClipViews.ts'), /parseClipFacetRoute\(currentTab\)/, 'Type and Source views must share calculated collection filtering');
assert.match(sidebar, /missingSources[\s\S]*get_source_icons/, 'Source icons must request only newly observed applications');
assert.match(sidebar, /\[sourcesEnabled, sourceIconSignature\]/, 'Clip count and ordering changes must not retrigger source icon extraction');
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
assert.match(database, /pub struct ClipItem[\s\S]{0,700}pub source: String/, 'Shared structured Search output must expose the canonical source field');
assert.match(database, /id,content_type,source,is_pinned/, 'CSV exports must expose the canonical source header');
assert.match(appData, /record\.source_app[\s\S]*source_app:\s*_legacySource/, 'Pre-1.0 cached and IPC clip summaries must migrate source_app without retaining it');
assert.match(sidebar, /source\?\.trim\(\)\.toLowerCase\(\)\s*\?\?\s*''/, 'Source icon rendering must tolerate stale or incomplete cached metadata');
assert.match(clipViews, /getClipCollection\(currentTab, selectedBin\)/, 'Clip filtering must resolve the active collection');
assert.match(clipViews, /clipsApi\.search\(/, 'GUI Search must use the centralized Clips client');
assert.match(
  clipViews,
  /setSearchResult\(\(current\) => \(\{ \.\.\.current, loading: true, failed: false \}\)\)/,
  'Search refreshes must retain settled results until their replacement is ready',
);
assert.match(
  app,
  /isLoadingCurrentCollection && currentCollection\?\.membership !== 'search'/,
  'Search must not reuse the History pagination loading interstitial',
);
assert.match(app, /useDeferredValue\((?:searchQuery|query)\)/,
  'Search result rendering must not compete with controlled input updates');
assert.match(app, /setTimeout\(\(\) => setSettledQuery\(deferredQuery\), delayMs\)/,
  'Search requests must wait for the explicit settled-query delay');
assert.match(clipViews, /startTransition\(\(\) => \{[\s\S]{0,120}setSearchResult/,
  'Authoritative Search results must commit at transition priority');
assert.doesNotMatch(clipViews, /setTimeout\([\s\S]{0,300}clipsApi\.search/,
  'Search must rely on deferred rendering instead of a fixed debounce');
assert.match(clipViews, /resolveSearchDisplayItems\([\s\S]{0,120}normalizedSearchQuery[\s\S]{0,120}searchResult\.query/,
  'Search display state must pass through the blank-query guard');
assert.match(searchPagination, /return normalizedQuery && resultQuery \? resultItems : \[\];/,
  'Blank and first-pending Search states must never fall back to History clips');
assert.match(app, /currentCollection\?\.membership === 'search' && Boolean\(searchDisplayQuery\)/,
  'Search must preserve a settled empty state while its replacement query runs');
assert.match(app, /searchQuery=\{currentTab === 'search' \? searchDisplayQuery : searchQuery\}/,
  'A preserved Search empty state must retain its settled query until replacement');
assert.match(nativeClipSearch, /indexed_fts_like[\s\S]{0,300}term_fields::base\(fts_like\)/,
  'Ordinary Search must pass its FTS5 trigram-optimized LIKE policy to term fields');
assert.match(nativeClipSearchTermFields, /WHERE text_content \{fts_like\}/,
  'Ordinary Search term fields must retain the FTS5 text-content path');
assert.match(clipsApi, /invoke<ClipSearchResult>\('search_clips'/, 'The Clips client must use the authoritative shared Search service');
assert.doesNotMatch(clipViews, /search_clip_searchable_text_ids/, 'GUI Search must not intersect extracted-text IDs with loaded pages');
assert.match(database, /LOWER\(clips\.content_type\) LIKE \? ESCAPE/, 'Collection-axis Search filters must use fuzzy case-insensitive matching');
assert.match(clipViews, /facet\?\.kind === 'clip_type'[\s\S]{0,160}clip\.content_type === facet\.value/, 'Clip Type routes must filter structural identity only');
assert.match(clipViews, /facet\?\.kind === 'content_type'[\s\S]{0,180}clip\.content_types/, 'Content Type routes must filter Classifier results only');
assert.match(emptyState, /collection\?\.emptyTitle/, 'Empty states must come from the collection descriptor');
assert.match(viewPolicy, /collection\?\.membership/, 'Interaction policy must use collection membership');
assert.match(
  clipCard,
  /features\.concealment && viewPolicy\.canOrganize && onToggleConcealed/,
  'Clip cards must honor collection policy before exposing concealment actions',
);
assert.match(clipListHeader, /collection\?\.title/, 'The clip-list heading must use the collection descriptor');
assert.match(appNavigation, /tab\.startsWith\('clip_type-'\)[\s\S]{0,180}tab\.startsWith\('file_format-'\)/, 'Search escape must remember every collection-axis route');
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
