import assert from 'node:assert/strict';
import {
  dateTimeAttribute,
  formatFullDateTime,
  formatRelativeTime,
  parseDbDate,
} from '../src/utils/date.ts';

const instant = '2026-08-17T12:34:56Z';
const canonicalInstant = '2026-08-17T12:34:56.000Z';
assert.equal(parseDbDate('2026-08-17 12:34:56').toISOString(), canonicalInstant,
  'Legacy SQLite timestamps must remain UTC.');
assert.equal(parseDbDate('2026-08-17T07:34:56-05:00').toISOString(), canonicalInstant,
  'Offset timestamps must normalize to the same instant.');
assert.equal(dateTimeAttribute('2026-08-17 12:34:56'), canonicalInstant,
  'Machine-readable time attributes must stay canonical UTC.');

const now = Date.parse('2026-08-17T12:40:56Z');
assert.equal(formatRelativeTime(instant, now, 'en-US'), '6m ago');
assert.equal(formatRelativeTime(now, now, 'en-US'), 'now');
assert.equal(
  formatRelativeTime('2026-08-16T10:40:56Z', now, 'en-US'),
  '1d ago',
  'Elapsed days must not use calendar terms such as "yesterday".',
);
assert.equal(
  formatRelativeTime('2026-08-09T12:40:56Z', now, 'en-US'),
  '8d ago',
  'Elapsed days must not be rounded to calendar terms such as "last week".',
);
assert.equal(
  formatRelativeTime('2026-07-18T12:40:56Z', now, 'en-US'),
  '1mo ago',
  'Older units must remain numeric rather than using calendar terms.',
);
assert.notEqual(formatRelativeTime(instant, now, 'de-DE'), formatRelativeTime(instant, now, 'en-US'),
  'Relative-time presentation must honor the selected locale.');

const english = formatFullDateTime(instant, 'en-US');
const german = formatFullDateTime(instant, 'de-DE');
assert.notEqual(english, german, 'Full timestamp presentation must honor the selected locale.');

console.log('Locale-aware timestamp tests passed.');
