import type { ClipSearchRequest } from './types';

export interface SearchHistoryEntry {
  id: number;
  request: ClipSearchRequest;
  resultCount: number;
  useCount: number;
  lastUsedAt: string;
}

export interface SearchHistoryPage {
  items: SearchHistoryEntry[];
  totalCount: number;
}
