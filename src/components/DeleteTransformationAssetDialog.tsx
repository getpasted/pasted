import { Trash2 } from 'lucide-react';
import { AppDialog } from './AppDialog';
import {
  AppDialogBody,
  AppDialogButton,
  AppDialogFooter,
  AppDialogHeader,
  AppDialogHeading,
} from './AppDialogLayout';

interface DeleteTransformationAssetDialogProps {
  asset: { kind: 'Transform' | 'Pipeline' | 'Operation'; name: string } | null;
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
      panelClassName="app-dialog-panel w-full max-w-md overflow-hidden rounded-2xl border shadow-2xl"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose} closeLabel={`Close delete ${asset.kind} dialog`}>
          <AppDialogHeading
            id="delete-transformation-asset-title"
            title={`Delete ${asset.kind}?`}
            description={asset.name}
            icon={<Trash2 />}
            tone="danger"
          />
        </AppDialogHeader>
        <AppDialogBody>
          <p className="text-xs theme-text-muted">
            This removes the {asset.kind.toLowerCase()} from Pasted. Clips already created or changed by it remain unchanged.
          </p>
        </AppDialogBody>
        <AppDialogFooter>
          <AppDialogButton onClick={requestClose} disabled={isDeleting} autoFocus>Cancel</AppDialogButton>
          <AppDialogButton variant="danger" onClick={onConfirm} disabled={isDeleting}>
            {isDeleting ? 'Deleting…' : `Delete ${asset.kind}`}
          </AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
}
