import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { parseClipSearch } from '../src/utils/clipSearchGrammar.ts';
import { appendUniqueSearchPage } from '../src/utils/searchPagination.ts';

interface GrammarFixture {
  query: string;
  sources: string[];
  clipTypes: string[];
  contentTypes: string[];
  fileFormats: string[];
  terms: string[];
  requiresNote: boolean;
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
    sources: plan.sources,
    clipTypes: plan.clipTypes,
    contentTypes: plan.contentTypes,
    fileFormats: plan.formats,
    terms: plan.terms,
    requiresNote: plan.requiresNote,
    requiresPinned: plan.requiresPinned,
    requiresProtected: plan.requiresProtected,
    requiresTrashed: plan.requiresTrashed,
    incomplete: plan.hasIncompleteFilter,
    regex: plan.regex?.source ?? null,
    regexFallback: plan.regexFallback,
  }, {
    sources: fixture.sources,
    clipTypes: fixture.clipTypes,
    contentTypes: fixture.contentTypes,
    fileFormats: fixture.fileFormats,
    terms: fixture.terms,
    requiresNote: fixture.requiresNote,
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

console.log(`Clip search grammar tests passed (${fixtures.length} shared fixtures).`);
