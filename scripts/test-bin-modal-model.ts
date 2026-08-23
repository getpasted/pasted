import assert from 'node:assert/strict';

import { lastEmojiGrapheme } from '../src/components/binModalEmoji.ts';

assert.equal(lastEmojiGrapheme(''), null, 'Empty icon input must preserve the current icon');
assert.equal(lastEmojiGrapheme('📂'), '📂', 'A single emoji must remain intact');
assert.equal(lastEmojiGrapheme('📂🚀'), '🚀', 'The most recently entered emoji must win');
assert.equal(lastEmojiGrapheme('📂👨‍👩‍👧‍👦'), '👨‍👩‍👧‍👦', 'Joined emoji must remain one grapheme');

console.log('Bin Modal model tests passed.');
