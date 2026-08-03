export type ClipViewState = 'active' | 'trash';

export interface ClipViewPolicy {
  state: ClipViewState;
  canDragClips: boolean;
  canOrganize: boolean;
  canAssignBins: boolean;
  canEditNotes: boolean;
  canMutateContent: boolean;
  canApplyFilters: boolean;
  showOrganizeBatchActions: boolean;
}

const ACTIVE_CLIP_POLICY: ClipViewPolicy = Object.freeze({
  state: 'active',
  canDragClips: true,
  canOrganize: true,
  canAssignBins: true,
  canEditNotes: true,
  canMutateContent: true,
  canApplyFilters: true,
  showOrganizeBatchActions: true,
});

const TRASH_CLIP_POLICY: ClipViewPolicy = Object.freeze({
  state: 'trash',
  canDragClips: false,
  canOrganize: false,
  canAssignBins: false,
  canEditNotes: false,
  canMutateContent: false,
  canApplyFilters: false,
  showOrganizeBatchActions: false,
});

export function getClipViewPolicy(
  view: string,
  clip?: { is_trashed?: boolean | number } | null,
): ClipViewPolicy {
  return view === 'trash' || Boolean(clip?.is_trashed)
    ? TRASH_CLIP_POLICY
    : ACTIVE_CLIP_POLICY;
}
