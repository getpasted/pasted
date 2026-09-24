import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  createVirtualClipLayout,
  estimatedClipCardHeight,
  virtualClipIndexes,
} from '../src/utils/virtualClipList.ts';
import { clipCollectionIsReady } from '../src/utils/clipListViewport.ts';
import {
  cacheRecentClipCollection,
  clearRecentClipCollections,
  getRecentClipCollection,
} from '../src/utils/recentClipCollectionCache.ts';
import {
  cachePreview,
  getCachedPreview,
  previewCacheWeightForTests,
} from '../src/utils/previewMemoryCache.ts';

assert.equal(clipCollectionIsReady(true, 10, 10), false);
assert.equal(clipCollectionIsReady(false, 1, 10), true);
assert.equal(clipCollectionIsReady(false, 0, 0), true);

const cacheChunk = 'x'.repeat(10 * 1024 * 1024);
cachePreview('preview-a', cacheChunk);
cachePreview('preview-b', cacheChunk);
cachePreview('preview-c', cacheChunk);
assert.equal(getCachedPreview('preview-a'), undefined, 'the shared preview cache must evict its oldest entry');
assert.ok(previewCacheWeightForTests() <= 48 * 1024 * 1024, 'preview cache weight must remain bounded');

clearRecentClipCollections();
for (let index = 0; index < 13; index += 1) {
  cacheRecentClipCollection(String(index), { items: [], totalCount: index });
}
assert.equal(getRecentClipCollection('0'), undefined, 'recent collection pages must use bounded LRU eviction');
assert.equal(getRecentClipCollection('12')?.totalCount, 12);

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
assert.deepEqual(
  virtualClipIndexes(layout, 10_000, 100, 0),
  [4],
  'a stale scroll offset beyond a shrunken layout must still render the visible end of the list',
);
assert.ok(estimatedClipCardHeight('small') < estimatedClipCardHeight('medium'));
assert.ok(estimatedClipCardHeight('medium') < estimatedClipCardHeight('large'));
const viewportSource = readFileSync(new URL('../src/hooks/useVirtualClipViewport.ts', import.meta.url), 'utf8');
const listSource = readFileSync(new URL('../src/components/VirtualClipList.tsx', import.meta.url), 'utf8');
assert.match(
  viewportSource,
  /\[disabled, layoutSize, scrollRef\]/,
  'the viewport must resync after measurements or loaded batches clamp the real scroll offset',
);
assert.match(
  viewportSource,
  /useEffect\(\(\) => \{[\s\S]*if \(frame !== 0\) return;[\s\S]*frame = 0;[\s\S]*addEventListener\('scroll', update/,
  'the viewport listener must bind after commit and coalesce without starving scroll updates',
);
assert.match(
  listSource,
  /measurementFrameRef[\s\S]*requestAnimationFrame/,
  'clip measurements must batch layout updates outside ResizeObserver delivery',
);
assert.match(
  listSource,
  /disabled \|\| layout\.totalSize <= viewport\.height \+ OVERSCAN_PX/,
  'short clip collections must remain in stable document flow',
);
assert.match(
  listSource,
  /viewportIndexes[\s\S]*offscreenForcedIndexes[\s\S]*viewportIndexes\.map/,
  'the visible virtual window must use flow layout while forced offscreen clips stay positioned',
);
assert.match(listSource, /Array\.from\(new Set\(forcedClipIds\.flatMap/,
  'a selected pinned clip must not mount twice as a forced virtual row');

console.log('Virtual clip list tests passed.');
