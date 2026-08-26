import { Copy, StickyNote } from 'lucide-react';
import { UI_COPY } from '../utils/uiCopy';
import type { ClipNote } from '../types';
import { soundManager } from '../utils/sound';
import { AppDialog } from './AppDialog';
import { translate } from '../localization/runtime';
import {
  AppDialogBody,
  AppDialogButton,
  AppDialogFooter,
  AppDialogHeader,
  AppDialogHeading,
} from './AppDialogLayout';

interface ClipNoteViewerProps {
  note: ClipNote;
  source: string | null;
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
      panelClassName="clip-note-viewer-card border rounded-2xl w-full max-w-lg overflow-hidden flex flex-col max-h-[80vh]"
    >
      {({ requestClose }) => (
        <>
          <AppDialogHeader onClose={requestClose} closeLabel={translate('component.clipNoteViewer.closeNoteViewer')}>
            <AppDialogHeading
              id="clip-note-viewer-title"
              title={translate('component.clipNoteViewer.noteAnnotation')}
              icon={<StickyNote />}
              tone="warning"
            />
          </AppDialogHeader>

          <AppDialogBody className="clip-note-viewer-body space-y-3">
            <div className="clip-note-viewer-content elevation-inset border rounded-xl p-4 font-mono text-xs whitespace-pre-wrap leading-relaxed select-text">
              {note.text}
            </div>
            <div className={`clip-note-viewer-meta flex items-center text-[11px] font-sans px-1 ${source ? 'justify-between' : 'justify-end'}`}>
              {source && <span>{translate('format.labelValue', { label: translate('component.clipNoteViewer.source'), value: source })}</span>}
              <span>{translate('format.characterCount', { count: note.text.length })}</span>
            </div>
          </AppDialogBody>

          <AppDialogFooter>
            <AppDialogButton variant="warning" onClick={() => void copyNote()}>
              <Copy className="w-3.5 h-3.5" />
              <span>{UI_COPY.copy}</span>
            </AppDialogButton>
            <AppDialogButton onClick={requestClose}>{translate('common.close')}</AppDialogButton>
          </AppDialogFooter>
        </>
      )}
    </AppDialog>
  );
}
