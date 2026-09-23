import assert from 'node:assert/strict';

import {
  mergePinnedClipSnapshots,
  pinnedClipIsConcealed,
  pinnedClipSummary,
} from '../src/components/pinnedClipShelfModel.ts';
import type { ClipContentType, ClipItem } from '../src/types.ts';

function clip(contentType: ClipContentType, textContent: string, contentTypes?: ClipContentType[]): ClipItem {
  return {
    id: 1,
    content_type: contentType,
    content_types: contentTypes,
    text_content: textContent,
    html_content: null,
    image_base64: null,
    content_hash: 'hash',
    source: 'Safari',
    is_pinned: true,
    is_concealed: true,
    bin_id: null,
    bin_ids: [],
    created_at: '2026-09-23T00:00:00Z',
  };
}

const credential = clip('text', 'credential contents');
credential.is_concealed = false;
credential.content_types = ['credential'];
assert.equal(pinnedClipIsConcealed(credential, [], [{ id: 'credential', concealClips: true }]), true,
  'the shelf must apply inherited Content Type concealment');
assert.equal(pinnedClipIsConcealed(
  { ...credential, content_types: ['text'], bin_ids: [7] },
  [{
    id: 7,
    name: 'Secrets',
    icon: 'folder',
    color: 'blue',
    conceal_clips: true,
    created_at: '2026-09-23T00:00:00Z',
  }],
  [],
), true, 'the shelf must apply inherited Bin concealment');
assert.equal(pinnedClipIsConcealed(
  { ...credential, is_explicitly_revealed: true },
  [],
  [{ id: 'credential', concealClips: true }],
), false, 'an explicit reveal must override inherited shelf concealment');
let visibleSummaryCalls = 0;
const visibleSummary = (item: ClipItem) => {
  visibleSummaryCalls += 1;
  return item.text_content ?? '';
};
assert.equal(pinnedClipSummary(credential, true, visibleSummary), '•••• ••••',
  'a concealed pinned clip must not expose its text in the shelf');
assert.equal(visibleSummaryCalls, 0,
  'concealed shelf summaries must not evaluate the visible summary path');
assert.equal(pinnedClipSummary(credential, false, visibleSummary), credential.text_content,
  'a revealed pinned clip should retain its compact text summary');
assert.equal(visibleSummaryCalls, 1);

const file = clip('file', '/Users/example/secret.txt');
assert.equal(pinnedClipSummary(file, true, visibleSummary), '•••• ••••',
  'a concealed file clip must not expose its path or name in the shelf');

const paymentCard = clip('text', '4111111111111111', ['payment_card']);
assert.equal(pinnedClipSummary(paymentCard, true, visibleSummary), '•••• •••• •••• 1111',
  'the shelf should use the shared payment-card mask');

const revealedCredential = { ...credential, is_explicitly_revealed: true };
const newPinnedClip = { ...credential, id: 2, text_content: 'new pinned clip' };
const leavingClip = { ...credential, id: 3, text_content: 'leaving clip' };
const refreshed = mergePinnedClipSnapshots(
  [revealedCredential, leavingClip],
  [credential, newPinnedClip],
);
assert.equal(refreshed[0], credential,
  'a displayed shelf clip must refresh when its concealment state changes');
assert.equal(refreshed[1], leavingClip,
  'a removed shelf clip must retain its snapshot for the exit animation');
assert.equal(refreshed[2], newPinnedClip,
  'a newly stacked clip should be added after retained shelf clips');
assert.equal(mergePinnedClipSnapshots([credential], [{ ...credential }])[0], credential,
  'equivalent parent snapshots must not trigger a shelf state update');

console.log('Pinned clip shelf model tests passed.');
