import assert from 'node:assert/strict';
import { pendingClipFocusId, selectionIdsForContextMenu } from '../src/utils/clipSelection.ts';
import { concealedClipMask } from '../src/utils/concealedClipMask.ts';

const multiSelection = new Set([2, 4, 6]);
assert.equal(
  selectionIdsForContextMenu(multiSelection, 4),
  multiSelection,
  'Right-clicking within a multi-selection must preserve the complete batch',
);

const concealedClip = {
  content_type: 'text',
  content_types: ['payment_card'],
  text_content: '4111 1111 1111 1234',
};
assert.equal(
  concealedClipMask(concealedClip),
  '•••• •••• •••• 1234',
  'Payment Card concealment may expose only the final four characters',
);
assert.equal(
  concealedClipMask({ ...concealedClip, content_type: 'image', content_types: [], text_content: null }),
  '•••• ••••',
  'Concealed non-text clips must render a non-empty generic mask',
);
assert.deepEqual(
  selectionIdsForContextMenu(multiSelection, 9),
  new Set([9]),
  'Right-clicking outside a multi-selection must select only the target clip',
);

const historyFocusRequest = { clipId: 42, requestId: 3, viewKey: 'section:all' };
assert.equal(
  pendingClipFocusId(historyFocusRequest, 'section:all', null),
  42,
  'A fresh History focus request must preserve its target clip across navigation',
);
assert.equal(
  pendingClipFocusId(historyFocusRequest, 'section:pinned', null),
  null,
  'A focus request must not affect another collection',
);
assert.equal(
  pendingClipFocusId(historyFocusRequest, 'section:all', 3),
  null,
  'A handled focus request must not override later user selection',
);
assert.deepEqual(
  selectionIdsForContextMenu(new Set(), 3),
  new Set([3]),
  'Right-clicking without a selection must select the target clip',
);

console.log('Clip selection controller tests passed.');
