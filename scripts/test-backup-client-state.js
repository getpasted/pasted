import assert from 'node:assert/strict';
import {
  applyBackupClientStateTo,
  collectBackupClientStateFrom,
} from '../src/utils/backupClientStateCodec.ts';

const createStorage = (entries = {}) => {
  const values = new Map(Object.entries(entries));
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
    removeItem: (key) => values.delete(key),
    snapshot: () => Object.fromEntries(values),
  };
};

const source = createStorage({
  pasted_app_ui_state: '{"version":2,"currentTab":"activity","sidebarSections":{"clips":false,"bins":true}}',
  pasted_sidebar_width: '284',
  pasted_list_width: '412',
  pasted_bin_order: '[3,1,2]',
  pasted_scroll_positions: '{"version":1,"positions":{"activity":{"scrollTop":640}}}',
  unrelated_secret: 'must-not-leave-storage',
});
const encoded = collectBackupClientStateFrom(source);
assert.deepEqual(JSON.parse(encoded), {
  version: 1,
  localStorage: {
    pasted_app_ui_state: '{"version":2,"currentTab":"activity","sidebarSections":{"clips":false,"bins":true}}',
    pasted_sidebar_width: '284',
    pasted_list_width: '412',
    pasted_bin_order: '[3,1,2]',
    pasted_scroll_positions: '{"version":1,"positions":{"activity":{"scrollTop":640}}}',
  },
});
assert.ok(!encoded.includes('unrelated_secret'));

const destination = createStorage({
  pasted_list_width: '999',
  unrelated_secret: 'preserved',
});
assert.equal(applyBackupClientStateTo(destination, encoded), true);
assert.deepEqual(destination.snapshot(), {
  pasted_app_ui_state: '{"version":2,"currentTab":"activity","sidebarSections":{"clips":false,"bins":true}}',
  pasted_sidebar_width: '284',
  pasted_list_width: '412',
  pasted_bin_order: '[3,1,2]',
  pasted_scroll_positions: '{"version":1,"positions":{"activity":{"scrollTop":640}}}',
  unrelated_secret: 'preserved',
});
assert.equal(applyBackupClientStateTo(destination, '{"version":2,"localStorage":{}}'), false);
assert.throws(() => applyBackupClientStateTo(destination, 'not-json'), SyntaxError);

console.log('Backup interface-state codec tests passed.');
