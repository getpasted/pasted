import type { ClipVersion } from '../types';
import { translate } from '../localization/runtime';
import { ConfirmationDialog } from './ConfirmationDialog';

export function DeleteClipVersionDialog({
  deleting,
  onCancel,
  onConfirm,
  version,
}: {
  deleting: boolean;
  onCancel: () => void;
  onConfirm: () => void | Promise<void>;
  version: ClipVersion | null;
}) {
  return <ConfirmationDialog
    request={version ? {
      title: translate('component.clipRevisionHistory.deleteVersionQuestion'),
      description: translate('component.clipRevisionHistory.deleteVersionDescription'),
      details: translate('component.clipRevisionHistory.deleteVersionDetails'),
      confirmLabel: translate('common.delete'),
      confirmDisabled: deleting,
      tone: 'danger',
      onConfirm,
    } : null}
    onCancel={onCancel}
  />;
}
