import type { ClipItem } from '../types';

export interface ClipListItem extends Omit<ClipItem, 'text_content' | 'html_content' | 'image_base64'> {
  preview_text: string | null;
  preview_truncated: boolean;
  file_names: string[];
  file_count: number;
}

export type ClipCollectionKind =
  | 'history' | 'trash' | 'bin' | 'pinned' | 'protected'
  | 'concealed' | 'named' | 'noted'
  | 'clipType' | 'contentType' | 'fileFormat' | 'source';

export interface ClipCollectionPageRequest {
  collection: ClipCollectionKind;
  binId?: number | null;
  value?: string | null;
  limit?: number;
  offset?: number;
}

export interface ClipListPage {
  schemaVersion: 1;
  items: ClipListItem[];
  totalCount: number;
  limit: number;
  offset: number;
}
