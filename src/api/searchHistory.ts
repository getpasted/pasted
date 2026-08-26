import type { ClipSearchRequest, SearchHistoryPage } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';

export const searchHistoryApi = {
  list: (limit: number, offset: number) =>
    invoke<SearchHistoryPage>('list_search_history', { limit, offset }),
  record: (request: ClipSearchRequest, resultCount: number) =>
    invoke<void>('record_search_history', { request, resultCount }),
  delete: (id: number) => invoke<void>('delete_search_history', { id }),
  clear: () => invoke<void>('clear_search_history'),
};
