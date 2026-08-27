import assert from 'node:assert/strict';

import {
  createVirtualClipLayout,
  estimatedClipCardHeight,
  virtualClipIndexes,
} from '../src/utils/virtualClipList.ts';

const layout = createVirtualClipLayout(
  [1, 2, 3, 4, 5],
  new Map([[2, 200]]),
  100,
  10,
);
assert.deepEqual(layout.positions.map(({ start, size }) => ({ start, size })), [
  { start: 0, size: 100 },
  { start: 110, size: 200 },
  { start: 320, size: 100 },
  { start: 430, size: 100 },
  { start: 540, size: 100 },
]);
assert.equal(layout.totalSize, 640, 'the final item must not add a trailing gap');
assert.deepEqual(
  virtualClipIndexes(layout, 300, 100, 0),
  [1, 2],
  'the virtual window must include cards intersecting the viewport',
);
assert.deepEqual(
  virtualClipIndexes(layout, 300, 100, 0, [0, 4]),
  [0, 1, 2, 4],
  'selected and pinned cards may remain mounted outside the viewport',
);
assert.ok(estimatedClipCardHeight('small') < estimatedClipCardHeight('medium'));
assert.ok(estimatedClipCardHeight('medium') < estimatedClipCardHeight('large'));

console.log('Virtual clip list tests passed.');
