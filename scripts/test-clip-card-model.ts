import assert from 'node:assert/strict';

import { clipCardPropsEqual, type ClipCardProps } from '../src/components/clipCardModel.ts';
import type { ClipItem } from '../src/types.ts';
import type { ClipViewPolicy } from '../src/utils/clipViewPolicy.ts';

const clip: ClipItem = {
  id: 1,
  content_type: 'text',
  content_types: ['prose'],
  text_content: 'Clip',
  html_content: null,
  image_base64: null,
  content_hash: 'hash',
  source: 'App',
  is_pinned: false,
  bin_id: null,
  bin_ids: [],
  created_at: '2026-08-22T00:00:00Z',
};
const noOp = () => undefined;
const viewPolicy: ClipViewPolicy = {
  state: 'active',
  canDragClips: true,
  canOrganize: true,
  canAssignBins: true,
  canEditNotes: true,
  canMutateContent: true,
  canRunManualTransforms: true,
  showOrganizeBatchActions: true,
};
const props: ClipCardProps = {
  clip,
  isSelected: false,
  viewPolicy,
  bins: [],
  filePreviewMode: 'safe',
  filePreviewMaxMb: 10,
  trashEnabled: true,
  onSelect: noOp,
  onPin: noOp,
  onDelete: noOp,
  onCopy: noOp,
  onContextMenu: noOp,
};

assert.equal(clipCardPropsEqual(props, { ...props, onCopy: () => undefined }), true,
  'callback identity changes must not rerender a ClipCard');
assert.equal(clipCardPropsEqual(props, {
  ...props,
  clip: { ...clip, html_content: '<b>unused</b>' },
}), true, 'clip fields that are not rendered must not invalidate a ClipCard');
assert.equal(clipCardPropsEqual(props, {
  ...props,
  clip: { ...clip, content_types: ['link'] },
}), false, 'rendered Content Type changes must invalidate a ClipCard');
assert.equal(clipCardPropsEqual(props, {
  ...props,
  viewPolicy: { ...props.viewPolicy, canOrganize: false },
}), false, 'organizing policy changes must update ClipCard actions');
assert.equal(clipCardPropsEqual(props, {
  ...props,
  bins: [{
    id: 1,
    name: 'Bin',
    icon: 'folder',
    color: 'blue',
    smart_rule: null,
    protect_clips: false,
    conceal_clips: false,
  } as ClipCardProps['bins'][number]],
}), false, 'Bin metadata changes must update ClipCard badges and policy');

console.log('ClipCard model tests passed.');
