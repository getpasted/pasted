import { AlertTriangle } from 'lucide-react';
import { UI_COPY } from '../utils/uiCopy';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';
import { translate } from '../localization/runtime';

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
            title={mode === 'purge' ? translate('component.clearHistoryDialog.deleteClipboardHistory') : translate('component.clearHistoryDialog.moveClipboardHistoryToTrash')}
            description={mode === 'purge' ? translate('common.thisActionCannotBeUndone') : translate('component.clearHistoryDialog.itemsCanBeRestoredFromTrash')}
            icon={<AlertTriangle />}
            tone="danger"
          />
        </AppDialogHeader>
        <AppDialogBody>
          <p className="app-dialog-message theme-surface text-xs leading-relaxed p-3 rounded-xl border">
            {mode === 'purge'
              ? translate('component.clearHistoryDialog.permanentlyDeleteAllUnpinnedAndUnprotectedClipboardHistoryPinnedClipsProtectedClips')
              : translate('component.clearHistoryDialog.moveAllUnpinnedAndUnprotectedClipboardHistoryIntoTrashPinnedClipsProtected')}
          </p>
        </AppDialogBody>
        <AppDialogFooter>
          <AppDialogButton onClick={requestClose} autoFocus>{translate('common.cancel')}</AppDialogButton>
          <AppDialogButton variant="danger" onClick={onConfirm}>{mode === 'purge' ? translate('component.clearHistoryDialog.deleteHistory') : UI_COPY.moveToTrash}</AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
}
