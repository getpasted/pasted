import assert from 'node:assert/strict';
import { sortFacetItemsByPopularity } from '../src/components/sidebarFacetModel.ts';

const sorted = sortFacetItemsByPopularity([
  { label: 'Beta', count: 2 },
  { label: 'Zulu', count: 8 },
  { label: 'Alpha', count: 2 },
  { label: 'Gamma', count: 5 },
]);

assert.deepEqual(sorted.map(({ label }) => label), ['Zulu', 'Gamma', 'Alpha', 'Beta']);
console.log('Sidebar facet model tests passed.');
