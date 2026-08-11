import { Copy, StickyNote } from 'lucide-react';
import { UI_COPY } from '../utils/uiCopy';
import type { ClipNote } from '../types';
import { soundManager } from '../utils/sound';
import { AppDialog } from './AppDialog';
import {
  AppDialogBody,
  AppDialogButton,
  AppDialogFooter,
  AppDialogHeader,
  AppDialogHeading,
} from './AppDialogLayout';

interface ClipNoteViewerProps {
  note: ClipNote;
  source: string;
  onClose: () => void;
}

export function ClipNoteViewer({ note, source, onClose }: ClipNoteViewerProps) {
  const copyNote = async () => {
    try {
      await navigator.clipboard.writeText(note.text);
      soundManager.playCopySound();
    } catch (error) {
      console.error('Failed to copy note:', error);
    }
  };

  return (
    <AppDialog
      isOpen
      onClose={onClose}
      labelledBy="clip-note-viewer-title"
      panelClassName="clip-note-viewer-card border rounded-2xl w-full max-w-lg shadow-2xl overflow-hidden flex flex-col max-h-[80vh]"
    >
      {({ requestClose }) => (
        <>
          <AppDialogHeader onClose={requestClose} closeLabel="Close note viewer">
            <AppDialogHeading
              id="clip-note-viewer-title"
              title="Note Annotation"
              icon={<StickyNote />}
              tone="warning"
            />
          </AppDialogHeader>

          <AppDialogBody className="clip-note-viewer-body space-y-3">
            <div className="clip-note-viewer-content border rounded-xl p-4 font-mono text-xs whitespace-pre-wrap leading-relaxed select-text shadow-inner">
              {note.text}
            </div>
            <div className="clip-note-viewer-meta flex items-center justify-between text-[11px] font-sans px-1">
              <span>Source: <strong className="clip-note-viewer-meta-strong">{source}</strong></span>
              <span>{note.text.length} Characters</span>
            </div>
          </AppDialogBody>

          <AppDialogFooter>
            <AppDialogButton variant="warning" onClick={() => void copyNote()}>
              <Copy className="w-3.5 h-3.5" />
              <span>{UI_COPY.copy}</span>
            </AppDialogButton>
            <AppDialogButton onClick={requestClose}>Close</AppDialogButton>
          </AppDialogFooter>
        </>
      )}
    </AppDialog>
  );
}
