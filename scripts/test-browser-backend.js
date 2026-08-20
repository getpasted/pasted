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

  await assert.rejects(() => safeInvoke('definitely_not_a_command'), /Unsupported browser IPC command/);
  console.log('Browser backend regression tests passed.');
} finally {
  console.warn = originalWarn;
  await server.close();
}
