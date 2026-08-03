import type React from 'react';
import { Check, Edit3, Eye, Trash2 } from 'lucide-react';
import type { ClipNote } from '../types';

interface NoteRowItemProps {
  noteItem: ClipNote;
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
        zIndex: isDragging ? 30 : 10,
      } : undefined}
      className={`note-row relative group min-h-[42px] px-3 py-2 rounded-lg border flex items-center justify-between gap-3 select-none transition-[background-color,border-color,box-shadow,opacity,transform] duration-100 ease-out ${
        totalNotes > 1 ? 'cursor-grab active:cursor-grabbing' : 'cursor-default'
      } ${isDragging ? 'opacity-60 shadow-lg ring-1 ring-inset ring-amber-400/60' : ''}`}
      onPointerDown={(event) => {
        if ((event.target as HTMLElement).closest('button, input, textarea, select, a')) return;
        onReorderPointerDown(event);
      }}
    >
      {editingNoteId === noteItem.id ? (
        <div className="flex-1 flex flex-col space-y-2 py-1 min-w-0">
          <textarea
            rows={3}
            value={editingNoteText}
            onChange={(e) => setEditingNoteText(e.target.value)}
            className="w-full p-0 m-0 bg-transparent border-none outline-none focus:outline-none focus:ring-0 text-xs text-amber-200 resize-y min-h-[60px] note-input font-sans leading-relaxed"
            autoFocus
            onKeyDown={(e) => {
              if (e.key === 'Escape') setEditingNoteId(null);
            }}
          />
          <div className="flex items-center justify-end space-x-2">
            <button
              type="button"
              onClick={() => setEditingNoteId(null)}
              className="px-2.5 py-1 bg-[#2c2921] hover:bg-[#3a362c] text-gray-300 rounded text-xs font-medium transition-colors cursor-pointer"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={() => handleUpdateNoteItem(noteItem.id, editingNoteText)}
              className="flex items-center space-x-1 px-2.5 py-1 bg-amber-600 hover:bg-amber-500 text-white rounded text-xs font-semibold shadow cursor-pointer"
            >
              <Check className="w-3.5 h-3.5" />
              <span>Save</span>
            </button>
          </div>
        </div>
      ) : (
        <>
          <div className="flex items-start truncate flex-1 select-none py-1 min-w-0">
            <span className="note-text text-xs text-amber-100 font-normal whitespace-pre-wrap break-words leading-relaxed select-none">
              {noteItem.text}
            </span>
          </div>

          <div className="opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 transition-opacity duration-100 flex items-center gap-1 shrink-0">
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                setViewingNote(noteItem);
              }}
              className="note-icon-btn p-1 text-amber-400/70 hover:text-amber-200 hover:bg-white/10 rounded transition-colors"
              title="View Note Modal"
            >
              <Eye className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                setEditingNoteId(noteItem.id);
                setEditingNoteText(noteItem.text);
              }}
              className="note-icon-btn p-1 text-amber-400/70 hover:text-amber-200 hover:bg-white/10 rounded transition-colors"
              title="Edit Note"
            >
              <Edit3 className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                handleDeleteNoteItem(noteItem.id);
              }}
              className="note-icon-btn p-1 text-amber-400/70 hover:text-red-400 hover:bg-white/10 rounded transition-colors"
              title="Delete Note"
            >
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          </div>
        </>
      )}
    </div>
  );
};
