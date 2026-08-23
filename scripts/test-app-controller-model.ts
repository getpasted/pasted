import assert from 'node:assert/strict';
import type { ClipItem } from '../src/types.ts';
import {
  findDraggedPreviewClip,
  selectionHasRestrictedClip,
} from '../src/hooks/appControllerModel.ts';

const clip = (id: number, isTrashed = false) => ({
  id,
  is_trashed: isTrashed,
} as ClipItem);

const active = clip(1);
const trashed = clip(2, true);
const isRestricted = (item: ClipItem) => Boolean(item.is_trashed);
assert.equal(selectionHasRestrictedClip(new Set([1]), [active, trashed], isRestricted), false);
assert.equal(selectionHasRestrictedClip(new Set([1, 2]), [active, trashed], isRestricted), true);
assert.equal(selectionHasRestrictedClip(new Set([99]), [active], isRestricted), false,
  'unloaded selections must not invent a restricted policy');

assert.equal(findDraggedPreviewClip({ clipId: 1 }, [active], [trashed]), active);
assert.equal(findDraggedPreviewClip({ clipId: 2 }, [active], [trashed]), trashed);
assert.equal(findDraggedPreviewClip(null, [active], [trashed]), undefined);

console.log('App controller model tests passed.');
