import assert from 'node:assert/strict';
import fs from 'node:fs';

const read = (path) => fs.readFileSync(path, 'utf8');
const navigation = read('src/hooks/useAppNavigation.ts');
const controller = read('src/hooks/useSearchDialogController.ts');
const dialog = read('src/components/SearchDialog.tsx');
const sidebar = read('src/components/SidebarSearchFooter.tsx');
const collapsedSidebar = read('src/components/CollapsedSidebarSearchFooter.tsx');
const settings = read('src/components/SettingsDestination.tsx');
const appShell = read('src/components/AppShellView.tsx');
const clipViews = read('src/hooks/useClipViews.ts');
const pagination = read('src/utils/searchPagination.ts');
const emptyState = read('src/components/EmptyClipList.tsx');
const clipList = read('src/components/ClipListContent.tsx');
const css = read('src/styles/clips-sidebar.css');

assert.match(navigation, /if \(route === 'search'\) \{[\s\S]{0,100}openSearchDialog\(\);[\s\S]{0,40}return;/,
  'Search navigation must open the modal without replacing the active collection');
assert.doesNotMatch(`${appShell}\n${settings}`, /setSearchQuery/,
  'Committed Search state must not expose its raw setter outside the navigation owner');

assert.match(controller, /const runSearch = useCallback\([\s\S]{0,220}setCommittedQuery\(query\)[\s\S]{0,100}setSelectedBinId\(null\)[\s\S]{0,100}setCurrentTab\('search'\)/,
  'Every submitted Search must use one committed state transition');
assert.match(controller, /if \(!enabled\) \{[\s\S]{0,80}close\(\);[\s\S]{0,80}clear\(\);/,
  'Disabling Search must close its modal and clear its in-memory query');
assert.match(controller, /event\.key === 'Escape'[\s\S]{0,220}event\.defaultPrevented/,
  'Escape must defer to higher-priority interactions before clearing committed Search');
assert.match(controller, /if \(committedQuery\) clear\(\);[\s\S]{0,40}else exitSearch\(\);/,
  'Escape on an empty Search must restore the previous clip collection');
assert.match(controller, /event\.metaKey \|\| event\.ctrlKey[\s\S]{0,100}event\.key\.toLowerCase\(\) !== 'f'/,
  'The standard Search shortcut must remain cross-platform');

assert.match(dialog, /<AppDialog[\s\S]{0,900}data-search-dialog-input/,
  'Search input and helpers must live in the shared modal');
assert.match(dialog, /requestAnimationFrame[\s\S]{0,220}inputRef\.current\?\.focus\(\)[\s\S]{0,80}inputRef\.current\?\.select\(\)/,
  'The mounted Search modal must explicitly focus and select its input');
assert.doesNotMatch(dialog, /autoFocus/,
  'Search focus must wait until AppDialog records the launcher for restoration');
assert.match(dialog, /const helpers = getSearchHelpers\(features\)/,
  'Localized Search helpers must refresh whenever the dialog rerenders');
assert.match(sidebar, /readOnly[\s\S]{0,120}aria-haspopup="dialog"/,
  'The expanded sidebar Search surface must be a modal launcher');
assert.match(sidebar, /onMouseDown=\{\(event\) => event\.preventDefault\(\)\}/,
  'The expanded launcher must not steal focus from the modal input');
assert.match(collapsedSidebar, /aria-haspopup="dialog"/,
  'The collapsed Search control must advertise the same modal interaction');
assert.doesNotMatch(`${sidebar}\n${collapsedSidebar}`, /EmbeddedMenu|MenuItem|ChevronUp/,
  'The retired sidebar Search pop-up must not return');

assert.doesNotMatch(settings, /setSearchQuery|handleSidebarNavigate\('search'\)/,
  'Settings Search entry points must not reconstruct the old split state transition');
assert.ok((settings.match(/navigation\.runSearch\(/g) ?? []).length === 2,
  'OCR status and Search History must both use the shared Search operation');

assert.doesNotMatch(`${navigation}\n${clipViews}`, /useDeferredValue|useSettledSearchQuery|setTimeout/,
  'Explicitly submitted Search must not retain live-input delays');
assert.match(pagination, /resultQuery === normalizedQuery \? resultItems : \[\]/,
  'A replacement Search must never display another query’s clips');
assert.match(clipViews, /currentPageTotalCount:[\s\S]{0,120}searchResult\.query === normalizedSearchQuery[\s\S]{0,80}: displayedClips\.length/,
  'A replacement Search must never display another query’s result count');
assert.match(clipViews, /searchDisplayQuery: searchResult\.query === normalizedSearchQuery \? searchResult\.query : ''/,
  'A replacement Search must never retain another query’s highlights');

assert.match(clipList, /isSearching \|\| Boolean\(searchDisplayQuery\)/,
  'The pending Search state must render immediately while awaiting its first result');
assert.match(emptyState, /search-loading-mark/,
  'The pending Search state must animate its existing Search mark');
assert.match(emptyState, /collection\.searchingYourClips[\s\S]{0,180}collection\.searchingActiveAndTrashedClips/,
  'The pending Search state must describe the active operation');
assert.match(css, /@keyframes search-loading-figure-eight/,
  'Search loading must retain its figure-eight motion');
assert.match(css, /prefers-reduced-motion: reduce[\s\S]{0,100}\.search-loading-mark[\s\S]{0,80}animation: none/,
  'Search loading motion must honor reduced-motion preferences');

console.log('Search interface audit passed.');
