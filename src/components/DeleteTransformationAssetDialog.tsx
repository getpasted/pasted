import { Trash2 } from 'lucide-react';
import { AppDialog } from './AppDialog';
import { translate } from '../localization/runtime';
import {
  AppDialogBody,
  AppDialogButton,
  AppDialogFooter,
  AppDialogHeader,
  AppDialogHeading,
} from './AppDialogLayout';

interface DeleteTransformationAssetDialogProps {
  asset: { kind: 'Transform' | 'Operation'; name: string } | null;
  isDeleting?: boolean;
  onCancel: () => void;
  onConfirm: () => void | Promise<void>;
}

export function DeleteTransformationAssetDialog({
  asset,
  isDeleting = false,
  onCancel,
  onConfirm,
}: DeleteTransformationAssetDialogProps) {
  if (!asset) return null;
  return (
    <AppDialog
      isOpen
      onClose={onCancel}
      labelledBy="delete-transformation-asset-title"
      panelClassName="theme-panel w-full max-w-md overflow-hidden border"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose} closeLabel={translate('component.deleteTransformationAssetDialog.closeDeleteKindDialog', { kind: asset.kind })}>
          <AppDialogHeading
            id="delete-transformation-asset-title"
            title={translate('component.deleteTransformationAssetDialog.deleteKind', { kind: asset.kind })}
            description={asset.name}
            icon={<Trash2 />}
            tone="danger"
          />
        </AppDialogHeader>
        <AppDialogBody>
          <p className="text-xs theme-text-muted">{translate('component.deleteTransformationAssetDialog.removesKindFromLibrary', { kind: asset.kind.toLowerCase() })}</p>
        </AppDialogBody>
        <AppDialogFooter>
          <AppDialogButton onClick={requestClose} disabled={isDeleting} autoFocus>{translate('common.cancel')}</AppDialogButton>
          <AppDialogButton variant="danger" onClick={onConfirm} disabled={isDeleting}>
            {isDeleting ? translate('component.deleteTransformationAssetDialog.deleting') : translate('component.deleteTransformationAssetDialog.deleteKind2', { kind: asset.kind })}
          </AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
}
