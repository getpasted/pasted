import { RotateCcw, Trash2, X } from 'lucide-react';

import { translate } from '../localization/runtime';
import type { ClipItem } from '../types';
import type { ClipViewPolicy } from '../utils/clipViewPolicy';
import { clipDeleteLabel, UI_COPY } from '../utils/uiCopy';

export function ClipPreviewTrashActions({
  clip,
  onDelete,
  onRestore,
  trashEnabled,
  viewPolicy,
}: {
  clip: ClipItem;
  onDelete: () => void;
  onRestore: () => void;
  trashEnabled: boolean;
  viewPolicy: ClipViewPolicy;
}) {
  const permanently = viewPolicy.state === 'trash' || !trashEnabled;
  return <>
    {viewPolicy.state === 'trash' && <button
      type="button"
      onClick={onRestore}
      className="clip-preview-action is-success theme-focusable transition-colors"
      title={UI_COPY.restore}
      aria-label={UI_COPY.restore}
    >
      <RotateCcw />
    </button>}
    <button
      type="button"
      onClick={onDelete}
      disabled={Boolean(clip.is_protected) && viewPolicy.state !== 'trash'}
      className={`clip-preview-action preview-delete-btn theme-danger-text theme-focusable transition-[background-color,color,opacity] ${clip.is_protected && viewPolicy.state !== 'trash' ? 'cursor-not-allowed opacity-45' : ''}`}
      title={clip.is_protected && viewPolicy.state !== 'trash'
        ? translate('component.clipPreview.clipIsProtectedUnprotectFirstToDelete')
        : clipDeleteLabel({ trashEnabled, permanent: permanently })}
      aria-label={permanently
        ? translate('component.clipPreview.deleteClipPermanently')
        : translate('component.clipPreview.moveClipToTrash')}
    >
      {permanently ? <X /> : <Trash2 />}
    </button>
  </>;
}
