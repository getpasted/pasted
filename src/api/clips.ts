import type { ClipCollectionSummary, ClipItem, ClipMutationSummary, ClipSearchRequest, ClipSearchResult } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';

interface ClipPageRequest {
  binId?: number | null;
  onlyPinned?: boolean;
  limit: number;
  offset: number;
}

export interface BinAssignmentOutcome {
  updatedClips: ClipItem[];
}

export const clipsApi = {
  list: (request: ClipPageRequest) => invoke<unknown[]>('get_clips', { ...request }),
  listTrash: (request: Pick<ClipPageRequest, 'limit' | 'offset'>) => invoke<unknown[]>('get_trashed_clips', { ...request }),
  search: (request: ClipSearchRequest) => invoke<ClipSearchResult>('search_clips', { request }),
  collectionSummary: () => invoke<ClipCollectionSummary>('get_clip_collection_summary'),
  restore: (id: number) => invoke<void>('restore_clip', { id }),
  restoreAll: () => invoke<ClipMutationSummary>('restore_all_trashed_clips'),
  trash: (id: number) => invoke<void>('delete_clip', { id }),
  trashMany: (ids: number[]) => invoke<void>('batch_trash_clips', { ids }),
  purge: (id: number) => invoke<void>('purge_clip_permanently', { id }),
  emptyTrash: () => invoke<void>('empty_trash'),
  togglePin: (id: number) => invoke<boolean>('toggle_pin_clip', { id }),
  setPinned: (ids: number[], pinState: boolean) => invoke<void>('batch_pin_clips', { ids, pinState }),
  setProtected: (ids: number[], protectedState: boolean) =>
    invoke<void>('batch_protect_clips', { ids, protectedState }),
  setConcealed: (ids: number[], concealedState: boolean) =>
    invoke<void>('batch_conceal_clips', { ids, concealedState }),
  updateNote: (clipId: number, note: string | null) => invoke<void>('update_clip_note', { clipId, note }),
  updateName: (clipId: number, name: string | null) => invoke<ClipItem>('update_clip_name', { clipId, name }),
  copyById: (clipId: number) => invoke<void>('copy_clip_by_id', { clipId }),
  copyContent: (text: string | null, imageBase64: string | null) =>
    invoke<void>('copy_clip_to_system', { text, imageBase64 }),
  assignBin: (clipId: number, binId: number | null) => invoke<ClipItem | null>('assign_clip_bin', { clipId, binId }),
  assignManyToBin: (ids: number[], binId: number | null) =>
    invoke<BinAssignmentOutcome>('batch_assign_bin_clips', { ids, binId }),
  removeBin: (clipId: number, binId: number) => invoke<BinAssignmentOutcome>('remove_clip_bin', { clipId, binId }),
};
