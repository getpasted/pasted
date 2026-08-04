import { StickyNote } from 'lucide-react';
import type { ClipItem } from '../types';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';

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
            title={clip.note ? 'Edit Note' : 'Add Note'}
            description="Attach custom annotations or metadata to this clip."
            icon={<StickyNote />}
            tone="warning"
          />
        </AppDialogHeader>
        <AppDialogBody>
          <textarea
            value={text}
            onChange={(event) => onTextChange(event.target.value)}
            placeholder="Type your note here..."
            rows={4}
            autoFocus
            className="app-dialog-input theme-input w-full border rounded-xl p-3 text-xs focus:outline-none focus:border-amber-500 transition-colors resize-none font-sans"
          />
        </AppDialogBody>
        <AppDialogFooter>
          <AppDialogButton onClick={requestClose}>Cancel</AppDialogButton>
          <AppDialogButton variant="warning" onClick={() => onSave(clip, text.trim() || null)}>Save Note</AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
}
