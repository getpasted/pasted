import assert from 'node:assert/strict';

import {
  hudPasteShortcutIndex,
  hudPrimaryModifierLabel,
} from '../src/components/quickHudModel.ts';

const keyEvent = (key: string, overrides: Partial<Parameters<typeof hudPasteShortcutIndex>[0]> = {}) => ({
  key,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  ...overrides,
});

assert.equal(hudPasteShortcutIndex(keyEvent('1', { metaKey: true })), 0);
assert.equal(hudPasteShortcutIndex(keyEvent('9', { ctrlKey: true })), 8);
assert.equal(hudPasteShortcutIndex(keyEvent('4')), null, 'plain digits must remain searchable');
assert.equal(hudPasteShortcutIndex(keyEvent('1', { shiftKey: true, metaKey: true })), null);
assert.equal(hudPasteShortcutIndex(keyEvent('1', { altKey: true, ctrlKey: true })), null);
assert.equal(hudPasteShortcutIndex(keyEvent('1', { metaKey: true, ctrlKey: true })), null);
assert.equal(hudPasteShortcutIndex(keyEvent('0', { metaKey: true })), null);

assert.equal(hudPrimaryModifierLabel('macos'), '⌘');
assert.equal(hudPrimaryModifierLabel('windows'), 'Ctrl');
assert.equal(hudPrimaryModifierLabel('linux'), 'Ctrl');

console.log('Quick HUD model tests passed.');
