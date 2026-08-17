import assert from 'node:assert/strict';
import { isolateBidi, isRtlLocale } from '../src/localization/bidi.ts';
import { inlineResizeDelta } from '../src/utils/direction.ts';

assert.equal(isolateBidi('pasted registry list --json'), '\u2068pasted registry list --json\u2069');
assert.equal(isRtlLocale('ar', ['ar', 'he']), true);
assert.equal(isRtlLocale('en', ['ar', 'he']), false);
assert.equal(inlineResizeDelta(100, 125, 'ltr'), 25);
assert.equal(inlineResizeDelta(100, 75, 'rtl'), 25);

console.log('Bidirectional text isolation tests passed.');
