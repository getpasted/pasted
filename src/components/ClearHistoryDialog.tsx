import { AlertTriangle } from 'lucide-react';
import { UI_COPY } from '../utils/uiCopy';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';

export type ClearHistoryMode = 'trash' | 'purge';

interface ClearHistoryDialogProps {
  mode: ClearHistoryMode;
  onCancel: () => void;
  onConfirm: () => void | Promise<void>;
}

export function ClearHistoryDialog({ mode, onCancel, onConfirm }: ClearHistoryDialogProps) {
  return (
    <AppDialog
      isOpen
      onClose={onCancel}
      labelledBy="clear-history-title"
      panelClassName="app-dialog-danger theme-panel w-full max-w-md rounded-2xl border overflow-hidden font-sans"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose}>
          <AppDialogHeading
            id="clear-history-title"
            title={mode === 'purge' ? 'Delete Clipboard History?' : 'Move Clipboard History to Trash?'}
            description={mode === 'purge' ? 'This action cannot be undone.' : 'Items can be restored from Trash.'}
            icon={<AlertTriangle />}
            tone="danger"
          />
        </AppDialogHeader>
        <AppDialogBody>
          <p className="app-dialog-message theme-surface text-xs leading-relaxed p-3 rounded-xl border">
            {mode === 'purge'
              ? 'Permanently delete all unpinned and unprotected clipboard history? Pinned clips, protected clips, and Bin definitions will be preserved.'
              : 'Move all unpinned and unprotected clipboard history into Trash? Pinned clips, protected clips, and Bin definitions will be preserved.'}
          </p>
        </AppDialogBody>
        <AppDialogFooter>
          <AppDialogButton onClick={requestClose} autoFocus>Cancel</AppDialogButton>
          <AppDialogButton variant="danger" onClick={onConfirm}>{mode === 'purge' ? 'Delete History' : UI_COPY.moveToTrash}</AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
}
