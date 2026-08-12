import assert from 'node:assert/strict';
import fs from 'node:fs';

const activityView = fs.readFileSync('src/components/ActivityLogView.tsx', 'utf8');
const rustSources = fs.readdirSync('src-tauri/src')
  .filter((name) => name.endsWith('.rs'))
  .map((name) => fs.readFileSync(`src-tauri/src/${name}`, 'utf8'))
  .join('\n');

const literalLogEvents = new Set();
for (const match of rustSources.matchAll(/log_activity(?:_internal)?\([\s\S]{0,100}?"([a-z][a-z0-9_]+)"/g)) {
  literalLogEvents.add(match[1]);
}

// These event names are selected dynamically in batch database mutations.
const dynamicLogEvents = [
  'clip_trashed', 'clips_trashed',
  'clip_pinned', 'clips_pinned', 'clip_unpinned', 'clips_unpinned',
  'clip_bin_assigned', 'clips_bin_assigned', 'clip_bin_unassigned', 'clips_bin_unassigned',
  'clip_protected_toggled', 'clips_protected_toggled',
  'setting_changed', 'autostart_enabled', 'autostart_disabled',
];
const emittedEvents = new Set([...literalLogEvents, ...dynamicLogEvents]);
const renderedEvents = new Set([...activityView.matchAll(/case '([^']+)'/g)].map((match) => match[1]));

const missingBadges = [...emittedEvents].filter((event) => !renderedEvents.has(event));
assert.deepEqual(missingBadges, [], `Activity Log badge coverage is missing: ${missingBadges.join(', ')}`);

const filterFamilies = [
  ['paused', (event) => event === 'recording_auto_paused' || event === 'recording_manually_paused', "event_type === 'recording_auto_paused'"],
  ['resumed', (event) => event === 'recording_auto_resumed' || event === 'recording_manually_resumed', "event_type === 'recording_auto_resumed'"],
  ['skipped', (event) => event === 'clipboard_capture_ignored', "event_type === 'clipboard_capture_ignored'"],
  ['trashed', (event) => ['clip_trashed', 'clips_trashed', 'clip_auto_trashed', 'clips_trashed_all'].includes(event), "event_type === 'clip_trashed'"],
  ['restored', (event) => event === 'clip_restored', "event_type === 'clip_restored'"],
  ['revisions', (event) => event === 'clip_revision_restored', "event_type === 'clip_revision_restored'"],
  ['purged', (event) => ['clip_deleted', 'trash_emptied', 'clips_purged_all'].includes(event), "event_type === 'clip_deleted'"],
  ['protection', (event) => event === 'clip_protected_toggled' || event === 'clips_protected_toggled', "event_type === 'clip_protected_toggled'"],
  ['pinning', (event) => event.includes('pinned'), "event_type.includes('pinned')"],
  ['notes', (event) => event === 'note_updated', "event_type === 'note_updated'"],
  ['bins', (event) => event.startsWith('bin_') || event.includes('_bin_'), "event_type.startsWith('bin_')"],
  ['transforms', (event) => event.startsWith('transform_') || event.startsWith('transformation_') || event.startsWith('bin_transform_') || event.startsWith('operation_') || event.startsWith('pipeline_') || event === 'library_item_enabled_changed' || event === 'clip_transformed' || event === 'intelligence_connection_fallback', "event_type.startsWith('transform_')"],
  ['queue', (event) => event.startsWith('queue_'), "event_type.startsWith('queue_')"],
  ['hud', (event) => event.startsWith('hud_'), "event_type.startsWith('hud_')"],
  ['app', (event) => event.startsWith('app_'), "event_type.startsWith('app_')"],
  ['settings', (event) => event.startsWith('setting_') || event.startsWith('settings_') || event.startsWith('autostart_'), "event_type.startsWith('setting_')"],
  ['detection', (event) => event.startsWith('content_detector') || event.startsWith('content_detection') || event.startsWith('content_type'), "event_type.startsWith('content_detector')"],
  ['storage', (event) => event.startsWith('library_') || event === 'external_history_imported', "event_type.startsWith('library_')"],
];

const missingFilters = [...emittedEvents].filter((event) => !filterFamilies.some(([, matches]) => matches(event)));
assert.deepEqual(missingFilters, [], `Activity Log filter coverage is missing: ${missingFilters.join(', ')}`);

for (const [value, , predicate] of filterFamilies) {
  assert.match(activityView, new RegExp(`selectedTypeFilter === '${value}'[^\\n]+${predicate.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`),
    `Activity Log must retain the ${value} filter family`);
  assert.match(activityView, new RegExp(`value: '${value}'`),
    `Activity Log must expose the ${value} filter option`);
}

assert.match(activityView, /group: 'Automation'/, 'Activity filters should remain grouped for scanning');
assert.match(activityView, /<OverflowText text=\{log\.description\}/, 'Activity descriptions must disclose clipped text');
console.log(`Activity event audit passed (${emittedEvents.size} event types have badges and filters).`);
