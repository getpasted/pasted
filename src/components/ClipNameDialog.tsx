import { FilePenLine } from 'lucide-react';
import type { ClipItem } from '../types';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading, SaveButtonContent } from './AppDialogLayout';
import { translate } from '../localization/runtime';

interface ClipNameDialogProps {
  clip: ClipItem;
  text: string;
  onTextChange: (text: string) => void;
  onCancel: () => void;
  onSave: (clip: ClipItem, name: string | null) => void | Promise<void>;
}

export function ClipNameDialog({ clip, text, onTextChange, onCancel, onSave }: ClipNameDialogProps) {
  return (
    <AppDialog
      isOpen
      onClose={onCancel}
      labelledBy="clip-name-title"
      isDirty={text !== (clip.name || '')}
      panelClassName="theme-panel border rounded-2xl max-w-md w-full overflow-hidden font-sans"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose}>
          <AppDialogHeading
            id="clip-name-title"
            title={clip.name ? translate('action.editName') : translate('action.nameClip')}
            icon={<FilePenLine className="theme-named-text" />}
          />
        </AppDialogHeader>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            void onSave(clip, text.trim() || null);
          }}
        >
          <AppDialogBody>
            <input
              dir="auto"
              value={text}
              onChange={(event) => onTextChange(event.target.value)}
              placeholder={translate('component.clipNameDialog.nameOrEmoji')}
              maxLength={120}
              autoFocus
              className="app-dialog-input theme-input ui-field-radius w-full border p-3 text-xs focus:outline-none transition-colors font-sans"
            />
          </AppDialogBody>
          <AppDialogFooter>
            <AppDialogButton onClick={requestClose}>{translate('common.cancel')}</AppDialogButton>
            <AppDialogButton type="submit" variant="primary"><SaveButtonContent /></AppDialogButton>
          </AppDialogFooter>
        </form>
      </>}
    </AppDialog>
  );
}
