import { StickyNote } from 'lucide-react';

import type { ClipPreviewNotesController } from '../hooks/useClipPreviewNotes';
import { translate } from '../localization/runtime';
import { NoteRowItem } from './ClipNoteRow';

interface ClipPreviewNotesPanelProps {
  controller: ClipPreviewNotesController;
  readOnly: boolean;
}

export function ClipPreviewNotesPanel({ controller, readOnly }: ClipPreviewNotesPanelProps) {
  const {
    notes,
    isAdding,
    newNoteText,
    setNewNoteText,
    placeholder,
    editingNoteId,
    setEditingNoteId,
    editingNoteText,
    setEditingNoteText,
    setViewingNote,
    containerRef,
    reorder,
    cancelAdding,
    create,
    update,
    remove,
  } = controller;

  if (notes.length === 0 && !isAdding) return null;

  return (
    <div className="px-4 py-2.5 border-b space-y-2 note-container select-none">
      <div className="note-header-text flex items-center space-x-1.5 text-[11px] font-semibold uppercase tracking-wider select-none">
        <StickyNote className="w-3.5 h-3.5" />
        <span>{translate('component.clipPreview.noteCount', { count: notes.length })}</span>
      </div>
      <div
        ref={containerRef}
        className={`note-row-stack relative space-y-2 ${reorder.isSettling ? 'is-settling-stable-reorder' : ''}`}
      >
        {notes.map((noteItem) => (
          <NoteRowItem
            key={noteItem.id}
            noteItem={noteItem}
            totalNotes={notes.length}
            editingNoteId={editingNoteId}
            editingNoteText={editingNoteText}
            setEditingNoteId={setEditingNoteId}
            setEditingNoteText={setEditingNoteText}
            handleUpdateNoteItem={update}
            handleDeleteNoteItem={remove}
            setViewingNote={setViewingNote}
            readOnly={readOnly}
            isDragging={reorder.activeId === noteItem.id}
            reorderOffsetY={reorder.offsets[noteItem.id] ?? 0}
            onReorderPointerDown={(event) => reorder.startPointerReorder(noteItem.id, event)}
          />
        ))}
        {isAdding && (
          <div className="note-input-row p-3 rounded-lg border flex flex-col space-y-2 animate-in fade-in duration-100">
            <textarea
              dir="auto"
              rows={3}
              placeholder={placeholder}
              value={newNoteText}
              onChange={(event) => setNewNoteText(event.target.value)}
              className="w-full bg-transparent border-none outline-none focus:outline-none focus:ring-0 text-xs resize-y min-h-[60px] note-input font-sans leading-relaxed"
              autoFocus
              onKeyDown={(event) => { if (event.key === 'Escape') cancelAdding(); }}
            />
            <div className="flex items-center justify-end space-x-2 pt-1">
              <button type="button" onClick={cancelAdding} className="note-cancel-button">
                {translate('common.cancel')}
              </button>
              <button type="button" onClick={create} className="note-save-button">
                {translate('common.save')}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
