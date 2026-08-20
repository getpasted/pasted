import assert from 'node:assert/strict';
import { selectionIdsForContextMenu } from '../src/utils/clipSelection.ts';

const multiSelection = new Set([2, 4, 6]);
assert.equal(
  selectionIdsForContextMenu(multiSelection, 4),
  multiSelection,
  'Right-clicking within a multi-selection must preserve the complete batch',
);
assert.deepEqual(
  selectionIdsForContextMenu(multiSelection, 9),
  new Set([9]),
  'Right-clicking outside a multi-selection must select only the target clip',
);
assert.deepEqual(
  selectionIdsForContextMenu(new Set(), 3),
  new Set([3]),
  'Right-clicking without a selection must select the target clip',
);

console.log('Clip selection controller tests passed.');
