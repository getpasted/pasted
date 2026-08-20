import assert from 'node:assert/strict';
import fs from 'node:fs';

const read = (path) => fs.readFileSync(path, 'utf8');
const events = read('src-tauri/src/app_events.rs');
const liveApp = read('src-tauri/src/live_app.rs');
const monitor = read('src-tauri/src/clipboard_monitor.rs');
const commands = read('src-tauri/src/commands.rs');
const appData = read('src/hooks/useAppData.ts');

for (const source of [liveApp, monitor, commands]) {
  assert.doesNotMatch(
    source,
    /emit\(\s*["']clipboard-pause-changed["']/,
    'Clipboard pause changes must use the shared typed event helper',
  );
}

assert.match(events, /pub struct ClipboardPauseChanged/,
  'Clipboard pause events must expose a typed payload');
assert.match(events, /#\[serde\(rename_all = "camelCase"\)\][\s\S]{0,120}pub struct ClipboardPauseChanged/,
  'Clipboard pause payloads must use the frontend wire naming convention');
assert.match(appData, /listen<ClipboardPauseChangedEvent>\(APP_EVENTS\.clipboardPauseChanged/,
  'The frontend must consume the canonical typed clipboard pause event');

for (const retired of ['clip-updated', 'clips-updated']) {
  assert.doesNotMatch(commands, new RegExp(`["']${retired}["']`),
    `${retired} must not survive beside the shared clip-library invalidation`);
}
assert.match(commands, /emit_clip_library_changed/,
  'Native clip mutations must use the shared library invalidation event');
assert.match(appData, /listen\(APP_EVENTS\.clipLibraryChanged/,
  'The frontend must reconcile shared clip-library invalidations');

console.log('Typed application-event contract audit passed.');
