import { getClipCollection } from './clipCollections';

export type ClipViewState = 'active' | 'trash';

export interface ClipViewPolicy {
  state: ClipViewState;
  canDragClips: boolean;
  canOrganize: boolean;
  canAssignBins: boolean;
  canEditNotes: boolean;
  canMutateContent: boolean;
  canRunManualTransforms: boolean;
  showOrganizeBatchActions: boolean;
}

const ACTIVE_CLIP_POLICY: ClipViewPolicy = Object.freeze({
  state: 'active',
  canDragClips: true,
  canOrganize: true,
  canAssignBins: true,
  canEditNotes: true,
  canMutateContent: true,
  canRunManualTransforms: true,
  showOrganizeBatchActions: true,
});

const TRASH_CLIP_POLICY: ClipViewPolicy = Object.freeze({
  state: 'trash',
  canDragClips: false,
  canOrganize: false,
  canAssignBins: false,
  canEditNotes: false,
  canMutateContent: false,
  canRunManualTransforms: false,
  showOrganizeBatchActions: false,
});

const QUEUE_CLIP_POLICY: ClipViewPolicy = Object.freeze({
  state: 'active',
  canDragClips: true,
  canOrganize: false,
  canAssignBins: false,
  canEditNotes: false,
  canMutateContent: false,
  canRunManualTransforms: false,
  showOrganizeBatchActions: false,
});

export function getClipViewPolicy(
  view: string,
  clip?: { is_trashed?: boolean | number } | null,
): ClipViewPolicy {
  const collection = getClipCollection(view);
  if (collection?.membership === 'trash' || Boolean(clip?.is_trashed)) return TRASH_CLIP_POLICY;
  if (collection?.membership === 'queue') return QUEUE_CLIP_POLICY;
  return ACTIVE_CLIP_POLICY;
}
