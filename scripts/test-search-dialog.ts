import assert from 'node:assert/strict';
import {
  closeSearchDialog,
  commitSearchDialog,
  openSearchDialog,
  updateSearchDraft,
} from '../src/utils/searchDialog.ts';

const committedQuery = 'source:Safari';
const opened = openSearchDialog(committedQuery);
assert.deepEqual(opened, { isOpen: true, draft: committedQuery });

const edited = updateSearchDraft(opened, 'type:image');
assert.deepEqual(edited, { isOpen: true, draft: 'type:image' });
assert.equal(committedQuery, 'source:Safari', 'editing must not mutate the committed query');

assert.deepEqual(closeSearchDialog(), { isOpen: false, draft: '' });
assert.deepEqual(
  openSearchDialog(committedQuery),
  opened,
  'reopening after cancellation must restore the committed query',
);

assert.equal(commitSearchDialog(edited), 'type:image');

console.log('Search dialog model tests passed.');
