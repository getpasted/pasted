import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parseClipSearch } from '../src/utils/clipSearchGrammar.ts';
import { appendUniqueSearchPage, resolveSearchDisplayItems } from '../src/utils/searchPagination.ts';
import { searchHistoryRequestQuery } from '../src/utils/searchHistory.ts';

interface GrammarFixture {
  clipIds?: number[];
  query: string;
  sources: string[];
  clipTypes: string[];
  contentTypes: string[];
  fileFormats: string[];
  terms: string[];
  requiresNote: boolean;
  requiresNamed: boolean;
  requiresPinned: boolean;
  requiresProtected: boolean;
  requiresTrashed: boolean;
  incomplete: boolean;
  regex: string | null;
  regexFallback: string | null;
}

const fixtures = JSON.parse(readFileSync(
  new URL('../contracts/search/v1/grammar.json', import.meta.url),
  'utf8',
)) as GrammarFixture[];

for (const fixture of fixtures) {
  const plan = parseClipSearch(fixture.query);
  assert.deepEqual({
    clipIds: plan.clipIds,
    sources: plan.sources,
    clipTypes: plan.clipTypes,
    contentTypes: plan.contentTypes,
    fileFormats: plan.formats,
    terms: plan.terms,
    requiresNote: plan.requiresNote,
    requiresNamed: plan.requiresNamed,
    requiresPinned: plan.requiresPinned,
    requiresProtected: plan.requiresProtected,
    requiresTrashed: plan.requiresTrashed,
    incomplete: plan.hasIncompleteFilter,
    regex: plan.regex?.source ?? null,
    regexFallback: plan.regexFallback,
  }, {
    clipIds: fixture.clipIds ?? [],
    sources: fixture.sources,
    clipTypes: fixture.clipTypes,
    contentTypes: fixture.contentTypes,
    fileFormats: fixture.fileFormats,
    terms: fixture.terms,
    requiresNote: fixture.requiresNote,
    requiresNamed: fixture.requiresNamed,
    requiresPinned: fixture.requiresPinned,
    requiresProtected: fixture.requiresProtected,
    requiresTrashed: fixture.requiresTrashed,
    incomplete: fixture.incomplete,
    regex: fixture.regex,
    regexFallback: fixture.regexFallback,
  }, `Search grammar drifted for ${fixture.query}`);
}

assert.deepEqual(
  appendUniqueSearchPage([{ id: 3 }, { id: 2 }], [{ id: 2 }, { id: 1 }]),
  [{ id: 3 }, { id: 2 }, { id: 1 }],
  'paginated Search results must retain unloaded items without duplicating page boundaries',
);

const historyFallback = [{ id: 1 }, { id: 2 }];
assert.deepEqual(
  resolveSearchDisplayItems('', '', historyFallback),
  [],
  'an empty Search must not display the History collection behind a zero result count',
);
assert.deepEqual(
  resolveSearchDisplayItems('invoice', '', historyFallback),
  [],
  'a Search awaiting its first authoritative result must not display History as provisional results',
);
assert.deepEqual(
  resolveSearchDisplayItems('invoice', 'invoice', historyFallback),
  historyFallback,
  'an authoritative Search result must remain visible',
);

assert.equal(searchHistoryRequestQuery({
  query: 'invoice',
  clipIds: [4, 9],
  clipTypes: ['text'],
  contentTypes: ['financial record'],
  fileFormats: ['pdf'],
  sources: ['Acme Browser'],
  trash: true,
}), 'invoice id:4,9 clip:text content:"financial record" format:pdf source:"Acme Browser" is:trashed');
assert.equal(
  searchHistoryRequestQuery({ query: 'regex:invoice.*', sources: ['Finder'] }),
  null,
  'regex searches with separate filters must not be rerun with silently changed semantics',
);
assert.equal(
  searchHistoryRequestQuery({ query: 'invoice', sources: [`both "double" and 'single'`] }),
  null,
  'unrepresentable filter values must disable rerun rather than dropping a filter',
);
assert.equal(searchHistoryRequestQuery({ query: 'invoice', sources: ['say"hello'] }), 'invoice source:\'say"hello\'');
assert.equal(searchHistoryRequestQuery({ query: '"unfinished', sources: ['Finder'] }), null);

console.log(`Clip search grammar tests passed (${fixtures.length} shared fixtures).`);
