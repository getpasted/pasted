import { StickyNote } from 'lucide-react';
import type { ClipItem } from '../types';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading, SaveButtonContent } from './AppDialogLayout';
import { translate } from '../localization/runtime';

interface ClipNoteDialogProps {
  clip: ClipItem;
  text: string;
  onTextChange: (text: string) => void;
  onCancel: () => void;
  onSave: (clip: ClipItem, note: string | null) => void | Promise<void>;
}

export function ClipNoteDialog({ clip, text, onTextChange, onCancel, onSave }: ClipNoteDialogProps) {
  return (
    <AppDialog
      isOpen
      onClose={onCancel}
      labelledBy="clip-note-title"
      isDirty={text !== (clip.note || '')}
      panelClassName="theme-panel border rounded-2xl max-w-md w-full overflow-hidden font-sans"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose}>
          <AppDialogHeading
            id="clip-note-title"
            title={clip.note ? translate('action.editNote') : translate('action.addNote')}
            description={translate('component.clipNoteDialog.attachCustomAnnotationsOrMetadataToThisClip')}
            icon={<StickyNote />}
            tone="warning"
          />
        </AppDialogHeader>
        <AppDialogBody>
          <textarea
            value={text}
            onChange={(event) => onTextChange(event.target.value)}
            placeholder={translate('component.clipNoteDialog.typeYourNoteHere')}
            rows={4}
            autoFocus
            className="app-dialog-input theme-input ui-field-radius w-full border p-3 text-xs focus:outline-none transition-colors resize-none font-sans"
          />
        </AppDialogBody>
        <AppDialogFooter>
          <AppDialogButton onClick={requestClose}>{translate('common.cancel')}</AppDialogButton>
          <AppDialogButton variant="warning" onClick={() => onSave(clip, text.trim() || null)}><SaveButtonContent /></AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
}
