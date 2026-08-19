import assert from 'node:assert/strict';
import fs from 'node:fs';

const read = (path) => fs.readFileSync(path, 'utf8');
const commands = read('src-tauri/src/commands.rs');
const liveApp = read('src-tauri/src/live_app.rs');
const clipboardActions = read('src-tauri/src/clipboard_actions.rs');
const queueActions = read('src-tauri/src/queue_actions.rs');
const platformCapabilities = read('src-tauri/src/platform_capabilities.rs');

assert.doesNotMatch(liveApp, /crate::commands::/,
  'The live-app adapter must not call the GUI command adapter');

for (const sharedCall of [
  'clipboard_actions::copy_clip',
  'clipboard_actions::paste_clip',
  'queue_actions::paste_item',
  'queue_actions::paste_all',
]) {
  assert.match(liveApp, new RegExp(sharedCall),
    `The live-app adapter must use ${sharedCall}`);
}

assert.match(commands, /clipboard_actions::copy_clip/,
  'GUI copy must use the shared clipboard workflow');
assert.match(commands, /clipboard_actions::paste_clip/,
  'GUI paste must use the shared clipboard workflow');
assert.match(commands, /queue_actions::paste_item/,
  'GUI Queue paste must use the shared Queue workflow');
assert.match(commands, /queue_actions::paste_all/,
  'GUI Queue paste-all must use the shared Queue workflow');
assert.doesNotMatch(clipboardActions, /crate::commands::/,
  'Shared clipboard workflows must remain independent of GUI commands');
assert.doesNotMatch(queueActions, /crate::commands::/,
  'Shared Queue workflows must remain independent of GUI commands');
assert.match(platformCapabilities, /pub fn accessibility_status/,
  'Platform readiness must be exposed independently of GUI commands');

console.log('Application architecture boundary audit passed.');
