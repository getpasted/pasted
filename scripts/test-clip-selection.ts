import assert from 'node:assert/strict';
import { clipCollectionViewKey, clipIdsForSelectAll, isSelectAllShortcut, pendingClipFocusId, selectionIdsForContextMenu } from '../src/utils/clipSelection.ts';
import { concealedClipMask } from '../src/utils/concealedClipMask.ts';
import { ClipListScrollMemory } from '../src/utils/clipListScrollMemory.ts';
import { orderClipsForStableReorder } from '../src/utils/clipListViewport.ts';

assert.equal(clipCollectionViewKey('all', null), 'section:all');
assert.equal(clipCollectionViewKey('bin', 7), 'bin:7');
const scrollMemory = new ClipListScrollMemory();
scrollMemory.remember('section:all', { scrollTop: 480, anchorClipId: 42, anchorOffset: -12 });
scrollMemory.remember('bin:7', { scrollTop: 125, anchorClipId: 9, anchorOffset: 4 });
assert.deepEqual(scrollMemory.recall('section:all'), {
  scrollTop: 480, anchorClipId: 42, anchorOffset: -12,
}, 'History must retain its visible clip anchor as well as its raw scroll position');
assert.equal(scrollMemory.recall('bin:7').scrollTop, 125, 'Each Bin must retain an independent scroll position');
assert.equal(scrollMemory.recall('section:trash').scrollTop, 0, 'A newly visited collection must start at the top');

const clips = [{ id: 1 }, { id: 2 }, { id: 3 }];
assert.deepEqual(
  orderClipsForStableReorder(clips, ['3', '1', '2'], (clip) => String(clip.id)).map((clip) => clip.id),
  [3, 1, 2],
  'the optimistic render order must match the committed drag order',
);
assert.strictEqual(orderClipsForStableReorder(clips, null, (clip) => String(clip.id)), clips,
  'settled lists must retain their canonical array identity');

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
assert.deepEqual(clipIdsForSelectAll([{ id: 7 }, { id: 3 }, { id: 11 }]), new Set([7, 3, 11]));
assert.equal(isSelectAllShortcut({ key: 'a', metaKey: true, ctrlKey: false, altKey: false }), true);
assert.equal(isSelectAllShortcut({ key: 'A', metaKey: false, ctrlKey: true, altKey: false }), true);
assert.equal(isSelectAllShortcut({ key: 'a', metaKey: true, ctrlKey: false, altKey: true }), false);

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
