import assert from 'node:assert/strict';
import { createServer } from 'vite';

const server = await createServer({
  configFile: false,
  logLevel: 'silent',
  server: { middlewareMode: true, hmr: false },
  appType: 'custom',
});
const originalWarn = console.warn;
console.warn = () => {};
try {
  const { safeInvoke } = await server.ssrLoadModule('/src/utils/tauri.ts');

  const created = await safeInvoke('create_operation', {
    name: 'Browser regression operation',
    opType: 'regex',
  });
  const operations = await safeInvoke('get_operations');
  assert.equal(operations.filter(({ id }) => id === created.id).length, 1,
    'creating one browser operation must persist exactly one operation');

  const search = await safeInvoke('search_clips', { request: { query: 'Sample', limit: 10, offset: 0 } });
  assert.equal(search.totalCount, 2,
    'the dispatcher must route Search into the in-memory library with authoritative totals');
  assert.ok((await safeInvoke('get_content_extractors')).length >= 3,
    'the dispatcher must route Extractor reads into the Content runtime');

  const connection = await safeInvoke('create_intelligence_connection', { name: 'Browser AI' });
  assert.equal((await safeInvoke('get_intelligence_connections')).some(({ id }) => id === connection.id), true,
    'the dispatcher must preserve Intelligence runtime mutations across calls');

  const originalLocation = await safeInvoke('get_library_location');
  const moved = await safeInvoke('move_library');
  assert.notEqual(moved.location.path, originalLocation.path,
    'the dispatcher must route storage mutations into the System runtime');
  assert.equal((await safeInvoke('restore_default_library_location')).location.path, originalLocation.path);

  const diagnostics = await safeInvoke('get_installation_diagnostics');
  assert.equal(diagnostics.buildKind, 'development',
    'browser diagnostics must use the localized build-kind contract, not its display label');

  const firstPin = await safeInvoke('batch_pin_clips', { ids: [101], pinState: true });
  assert.deepEqual(firstPin, {
    action: 'pin', requestedCount: 1, changedCount: 1, skippedCount: 0, clipIds: [101],
  });
  const mixedPin = await safeInvoke('batch_pin_clips', { ids: [101, 102, 102], pinState: true });
  assert.deepEqual(mixedPin, {
    action: 'pin', requestedCount: 3, changedCount: 1, skippedCount: 2, clipIds: [102],
  });
  const clips = await safeInvoke('get_clips');
  assert.equal(clips.find(({ id }) => id === 102)?.pin_order, 0,
    'newly pinned clips must be placed first');
  assert.equal(clips.find(({ id }) => id === 101)?.pin_order, 1,
    'already pinned clips must retain their relative order behind new pins');

  await safeInvoke('update_bin_concealment', { id: 1, concealClips: true });
  await safeInvoke('assign_clip_bin', { clipId: 101, binId: 1 });
  assert.equal((await safeInvoke('get_clips')).find(({ id }) => id === 101)?.is_concealed, true,
    'manual Bin concealment must be effective in the browser backend');
  assert.equal(await safeInvoke('toggle_clip_concealed', { clipId: 101 }), false,
    'toggling an inherited concealed clip must reveal it');
  assert.equal((await safeInvoke('get_clips')).find(({ id }) => id === 101)?.is_concealed, false,
    'an explicit reveal must survive browser-backend normalization');

  await safeInvoke('delete_clip', { id: 102 });
  assert.equal(await safeInvoke('toggle_clip_concealed', { clipId: 102 }), false,
    'toggling concealment must not mutate a trashed clip');
  await safeInvoke('batch_conceal_clips', { ids: [102], concealedState: true });
  assert.equal((await safeInvoke('get_trashed_clips')).find(({ id }) => id === 102)?.is_concealed, false,
    'batch concealment must not mutate a trashed clip');

  await safeInvoke('push_sequential_item', { item: 'first' });
  await safeInvoke('push_sequential_item', { item: 'second' });
  const queue = await safeInvoke('get_sequential_status');
  assert.deepEqual(queue.queue, ['first', 'second']);
  assert.equal(queue.total_count, 2);
  await assert.rejects(
    () => safeInvoke('reorder_sequential_items', { itemIds: [queue.item_ids[0], 999_999] }),
    /every current item exactly once/,
  );
  assert.deepEqual((await safeInvoke('get_sequential_status')).queue, ['first', 'second'],
    'an invalid Queue reorder must not corrupt browser state');
  await assert.rejects(
    () => safeInvoke('remove_sequential_item_by_index', { index: -1 }),
    /valid Queue index/,
  );

  await safeInvoke('save_app_settings', { values: { browserTest: 'preserved' } });
  assert.equal((await safeInvoke('get_all_app_settings')).browserTest, 'preserved');
  assert.equal((await safeInvoke('configure_app_lock')).enabled, true);
  assert.equal((await safeInvoke('set_app_lock_idle_minutes', { minutes: 12 })).idleMinutes, 12);
  const resetPolicy = await safeInvoke('reset_app_lock_policy');
  assert.equal(resetPolicy.enabled, true, 'policy reset must preserve App Lock credentials and state');
  assert.equal(resetPolicy.idleMinutes, 5);

  await assert.rejects(() => safeInvoke('definitely_not_a_command'), /Unsupported browser IPC command/);
  console.log('Browser backend regression tests passed.');
} finally {
  console.warn = originalWarn;
  await server.close();
}
