import { handled, unhandled, type BrowserMockResult } from './result';

let status = {
  is_active: false,
  queue: [] as string[],
  item_ids: [] as number[],
  current_index: 0,
  total_count: 0,
};
let nextItemId = 1;

function snapshot() {
  status = { ...status, total_count: status.queue.length };
  return { ...status, queue: [...status.queue], item_ids: [...status.item_ids] };
}

function queueIndex(args: Record<string, unknown> | undefined) {
  const index = Number(args?.index);
  if (!Number.isInteger(index) || index < 0) throw new Error('A valid Queue index is required.');
  return index;
}

export function handleQueueBrowserMock(
  command: string,
  args: Record<string, unknown> | undefined,
): BrowserMockResult {
  switch (command) {
    case 'get_sequential_status':
      return handled(snapshot());
    case 'get_queue_paste_target':
      return handled({
        name: 'Browser',
        automaticPasteAvailable: false,
        unavailableReason: 'This window cannot send system-wide paste commands.',
      });
    case 'start_sequential_paste':
      status.is_active = true;
      return handled(snapshot());
    case 'stop_sequential_paste':
      status.is_active = false;
      return handled(snapshot());
    case 'push_sequential_item':
      status.queue.push(String(args?.item ?? ''));
      status.item_ids.push(nextItemId++);
      return handled(snapshot());
    case 'pop_sequential_paste': {
      const item = status.queue.shift() ?? null;
      status.item_ids.shift();
      snapshot();
      return handled(item);
    }
    case 'paste_sequential_item_by_index': {
      const index = queueIndex(args);
      const [item] = status.queue.splice(index, 1);
      status.item_ids.splice(index, 1);
      snapshot();
      return handled(item ?? null);
    }
    case 'remove_sequential_item_by_index': {
      const index = queueIndex(args);
      status.queue.splice(index, 1);
      status.item_ids.splice(index, 1);
      return handled(snapshot());
    }
    case 'reorder_sequential_items': {
      const orderedIds = Array.isArray(args?.itemIds) ? args.itemIds.map(Number) : [];
      const currentIds = new Set(status.item_ids);
      if (orderedIds.length !== status.item_ids.length
        || new Set(orderedIds).size !== orderedIds.length
        || orderedIds.some((id) => !currentIds.has(id))) {
        throw new Error('Queue reorder must contain every current item exactly once.');
      }
      const textById = new Map(status.item_ids.map((id, index) => [id, status.queue[index]]));
      status.item_ids = orderedIds;
      status.queue = orderedIds.map((id) => textById.get(id) ?? '');
      return handled(snapshot());
    }
    case 'paste_all_sequential': {
      const combined = status.queue.length > 0 ? status.queue.join('\n\n') : null;
      status.queue = [];
      status.item_ids = [];
      status.is_active = false;
      snapshot();
      return handled(combined);
    }
    default:
      return unhandled;
  }
}
