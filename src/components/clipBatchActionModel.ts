import type { ClipCollectionDefinition } from '../utils/clipCollections';

export type ClipBatchCollectionAction = 'unpin' | 'unprotect' | 'reveal' | 'restore';

export function getClipBatchCollectionAction(
  collection?: Pick<ClipCollectionDefinition, 'association' | 'membership'>,
): ClipBatchCollectionAction | undefined {
  if (collection?.membership === 'trash') return 'restore';
  if (collection?.association === 'pin') return 'unpin';
  if (collection?.association === 'protect') return 'unprotect';
  if (collection?.association === 'conceal') return 'reveal';
  return undefined;
}

export function clipCollectionShowsGeneralPinActions(
  collection?: Pick<ClipCollectionDefinition, 'membership'>,
): boolean {
  return collection?.membership === 'bin' || collection?.membership === 'facet';
}
