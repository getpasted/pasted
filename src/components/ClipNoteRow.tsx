import type React from 'react';
import { Edit3, Eye, Trash2 } from 'lucide-react';
import type { ClipNote } from '../types';
import { FloatingActionStrip } from './FloatingActionStrip';
import { OverflowText } from './OverflowText';
import { translate } from '../localization/runtime';

interface NoteRowItemProps {
  noteItem: ClipNote;
  readOnly?: boolean;
  totalNotes: number;
  editingNoteId: string | null;
  editingNoteText: string;
  setEditingNoteId: (id: string | null) => void;
  setEditingNoteText: (text: string) => void;
  handleUpdateNoteItem: (id: string, text: string) => void;
  handleDeleteNoteItem: (id: string) => void;
  setViewingNote: (note: ClipNote | null) => void;
  isDragging: boolean;
  reorderOffsetY: number;
  onReorderPointerDown: (event: React.PointerEvent) => void;
}

export const NoteRowItem: React.FC<NoteRowItemProps> = ({
  noteItem,
  readOnly = false,
  totalNotes,
  editingNoteId,
  editingNoteText,
  setEditingNoteId,
  setEditingNoteText,
  handleUpdateNoteItem,
  handleDeleteNoteItem,
  setViewingNote,
  isDragging,
  reorderOffsetY,
  onReorderPointerDown,
}) => {
  return (
    <div
      data-stable-reorder-id={noteItem.id}
      style={reorderOffsetY !== 0 || isDragging ? {
        transform: `translateY(${reorderOffsetY}px)`,
        zIndex: isDragging ? 'var(--layer-drag)' : 1,
      } : undefined}
      className={`note-row relative group min-h-[42px] px-3 py-2 rounded-lg border flex items-center justify-between gap-3 select-none transition-[background-color,border-color,box-shadow,opacity,transform] duration-100 ease-out ${
        !readOnly && totalNotes > 1 ? 'cursor-grab active:cursor-grabbing' : 'cursor-default'
      } ${isDragging ? 'is-dragging elevation-floating opacity-60 ring-1 ring-inset' : ''}`}
      onPointerDown={(event) => {
        if (readOnly) return;
        if ((event.target as HTMLElement).closest('button, input, textarea, select, a')) return;
        onReorderPointerDown(event);
      }}
    >
      {editingNoteId === noteItem.id && !readOnly ? (
        <div className="flex-1 flex flex-col space-y-2 py-1 min-w-0">
          <textarea dir="auto"
            rows={3}
            value={editingNoteText}
            onChange={(e) => setEditingNoteText(e.target.value)}
            className="w-full p-0 m-0 bg-transparent border-none outline-none focus:outline-none focus:ring-0 text-xs resize-y min-h-[60px] note-input font-sans leading-relaxed"
            autoFocus
            onKeyDown={(e) => {
              if (e.key === 'Escape') setEditingNoteId(null);
            }}
          />
          <div className="flex items-center justify-end space-x-2">
            <button
              type="button"
              onClick={() => setEditingNoteId(null)}
              className="note-cancel-button"
            >
              {translate('common.cancel')}
            </button>
            <button
              type="button"
              onClick={() => handleUpdateNoteItem(noteItem.id, editingNoteText)}
              className="note-save-button"
            >
              {translate('common.save')}
            </button>
          </div>
        </div>
      ) : (
        <>
          <OverflowText
            as="div"
            text={noteItem.text}
            className="note-text flex items-start truncate flex-1 select-none py-1 min-w-0 text-xs font-normal whitespace-pre-wrap break-words leading-relaxed"
          />

          {!isDragging && (
            <FloatingActionStrip label={translate('component.clipNoteRow.noteActions')} revealOnGroupInteraction>
              <button
                type="button"
                onClick={(event) => {
                  event.stopPropagation();
                  setViewingNote(noteItem);
                }}
                className="floating-action-button"
                title={translate('component.clipNoteRow.viewNote')}
              >
                <Eye />
              </button>
              {!readOnly && (
                <button
                  type="button"
                  onClick={(event) => {
                    event.stopPropagation();
                    setEditingNoteId(noteItem.id);
                    setEditingNoteText(noteItem.text);
                  }}
                  className="floating-action-button is-warning"
                  title={translate('action.editNote')}
                >
                  <Edit3 />
                </button>
              )}
              {!readOnly && (
                <button
                  type="button"
                  onClick={(event) => {
                    event.stopPropagation();
                    handleDeleteNoteItem(noteItem.id);
                  }}
                  className="floating-action-button is-danger"
                  title={translate('component.clipNoteRow.deleteNote')}
                >
                  <Trash2 />
                </button>
              )}
            </FloatingActionStrip>
          )}
        </>
      )}
    </div>
  );
};
