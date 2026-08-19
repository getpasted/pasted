import assert from 'node:assert/strict';
import { parseClipSearch } from '../src/utils/clipSearchGrammar.ts';

const plan = parseClipSearch('clip:file content:email format:pdf source:finder');
assert.deepEqual(plan.clipTypes, ['file']);
assert.deepEqual(plan.contentTypes, ['email']);
assert.deepEqual(plan.formats, ['pdf']);
assert.deepEqual(plan.sources, ['finder']);
assert.equal(parseClipSearch('content:').hasIncompleteFilter, true);
assert.equal(parseClipSearch('format:').hasIncompleteFilter, true);
assert.deepEqual(parseClipSearch('type:email').terms, ['type:email']);

console.log('Clip search grammar tests passed.');
