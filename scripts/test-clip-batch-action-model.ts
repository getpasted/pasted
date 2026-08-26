import assert from 'node:assert/strict';

import { clipCollectionShowsGeneralPinActions, getClipBatchCollectionAction } from '../src/components/clipBatchActionModel.ts';

assert.equal(getClipBatchCollectionAction({ membership: 'pinned', association: 'pin' }), 'unpin');
assert.equal(getClipBatchCollectionAction({ membership: 'protected', association: 'protect' }), 'unprotect');
assert.equal(getClipBatchCollectionAction({ membership: 'concealed', association: 'conceal' }), 'reveal');
assert.equal(getClipBatchCollectionAction({ membership: 'trash' }), 'restore');
assert.equal(getClipBatchCollectionAction({ membership: 'all' }), undefined);
assert.equal(getClipBatchCollectionAction({ membership: 'bin' }), undefined);
assert.equal(getClipBatchCollectionAction({ membership: 'facet' }), undefined);
assert.equal(clipCollectionShowsGeneralPinActions({ membership: 'bin' }), true);
assert.equal(clipCollectionShowsGeneralPinActions({ membership: 'facet' }), true);
assert.equal(clipCollectionShowsGeneralPinActions({ membership: 'all' }), false);
assert.equal(clipCollectionShowsGeneralPinActions({ membership: 'named' }), false);
assert.equal(clipCollectionShowsGeneralPinActions({ membership: 'noted' }), false);
assert.equal(clipCollectionShowsGeneralPinActions({ membership: 'search' }), false);

console.log('Clip batch action model tests passed.');
