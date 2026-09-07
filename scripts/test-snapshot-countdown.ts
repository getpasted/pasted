import assert from 'node:assert/strict';
import { snapshotCountdownParts } from '../src/components/snapshotCountdown.ts';

const now = Date.parse('2026-09-06T12:00:00Z');
assert.deepEqual(snapshotCountdownParts('2026-09-06T12:01:01Z', now), {
  hours: 0,
  minutes: 1,
  seconds: 1,
});
assert.deepEqual(snapshotCountdownParts('2026-09-06T14:01:01Z', now), {
  hours: 2,
  minutes: 1,
  seconds: 1,
});
assert.deepEqual(snapshotCountdownParts('2026-09-06T11:59:00Z', now), {
  hours: 0,
  minutes: 0,
  seconds: 0,
});
assert.equal(snapshotCountdownParts('not-a-date', now), null);

console.log('Snapshot countdown tests passed.');
