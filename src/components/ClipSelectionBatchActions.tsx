import type { ClipViewPolicy } from '../utils/clipViewPolicy';
import type { ClipCollectionDefinition } from '../utils/clipCollections';
import { ClipBatchActionBar } from './ClipBatchActionBar';
import { clipCollectionShowsGeneralPinActions, getClipBatchCollectionAction } from './clipBatchActionModel';

export function ClipSelectionBatchActions({
  selectedClipIds,
  collection,
  viewPolicy,
  hasRestrictedSelection,
  pinningEnabled,
  trashEnabled,
  onSetPinned,
  onUnprotect,
  onReveal,
  onTrash,
  onRestore,
  onDeletePermanently,
  onClearSelection,
}: {
  selectedClipIds: Set<number>;
  collection?: ClipCollectionDefinition;
  viewPolicy: ClipViewPolicy;
  hasRestrictedSelection: boolean;
  pinningEnabled: boolean;
  trashEnabled: boolean;
  onSetPinned: (pinned: boolean) => void;
  onUnprotect: (ids: number[]) => void;
  onReveal: (ids: number[]) => void;
  onTrash: () => void;
  onRestore: (ids: number[]) => void;
  onDeletePermanently: (ids: number[]) => void;
  onClearSelection: () => void;
}) {
  const trashMode = viewPolicy.state === 'trash';
  const collectionAction = getClipBatchCollectionAction(collection);
  const visible = selectedClipIds.size > 1
    && (trashMode || (viewPolicy.showOrganizeBatchActions && !hasRestrictedSelection));
  if (!visible) return null;
  const selectedIds = Array.from(selectedClipIds);
  return <ClipBatchActionBar
    selectedCount={selectedIds.length}
    pinningEnabled={pinningEnabled && clipCollectionShowsGeneralPinActions(collection)}
    trashEnabled={trashEnabled}
    trashMode={trashMode}
    collectionAction={collectionAction}
    onSetPinned={onSetPinned}
    onUnprotect={() => onUnprotect(selectedIds)}
    onReveal={() => onReveal(selectedIds)}
    onTrash={onTrash}
    onRestore={() => onRestore(selectedIds)}
    onDeletePermanently={() => onDeletePermanently(selectedIds)}
    onClearSelection={onClearSelection}
  />;
}
