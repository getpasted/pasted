import { Trash2 } from 'lucide-react';
import type { Bin } from '../types';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';

interface DeleteBinDialogProps {
  bin: Bin;
  onCancel: () => void;
  onConfirm: (bin: Bin) => void | Promise<void>;
}

export function DeleteBinDialog({ bin, onCancel, onConfirm }: DeleteBinDialogProps) {
  return (
    <AppDialog
      isOpen
      onClose={onCancel}
      labelledBy="delete-bin-title"
      panelClassName="app-dialog-danger theme-panel border rounded-2xl max-w-sm w-full overflow-hidden"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose}>
          <AppDialogHeading id="delete-bin-title" title={<>Delete Bin &quot;{bin.name}&quot;?</>} icon={<Trash2 />} tone="danger" />
        </AppDialogHeader>
        <AppDialogBody>
          <p className="theme-text-muted text-xs">Clips in this bin will be unassigned and preserved.</p>
        </AppDialogBody>
        <AppDialogFooter>
          <AppDialogButton onClick={requestClose} autoFocus>Cancel</AppDialogButton>
          <AppDialogButton variant="danger" onClick={() => onConfirm(bin)}>Delete Bin</AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
}
