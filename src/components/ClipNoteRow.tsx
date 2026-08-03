import type React from 'react';
import { Reorder } from 'framer-motion';
import { ArrowDown, ArrowUp, Check, Edit3, Eye, GripVertical, Trash2 } from 'lucide-react';
import type { ClipNote } from '../types';

interface NoteRowItemProps {
  noteItem: ClipNote;
  index: number;
  totalNotes: number;
  noteBoxRef: React.RefObject<HTMLDivElement | null>;
  editingNoteId: string | null;
  editingNoteText: string;
  setEditingNoteId: (id: string | null) => void;
  setEditingNoteText: (text: string) => void;
  saveNotes: (notes: ClipNote[]) => void;
  notesRef: React.MutableRefObject<ClipNote[]>;
  handleUpdateNoteItem: (id: string, text: string) => void;
  handleMoveNoteUp: (index: number) => void;
  handleMoveNoteDown: (index: number) => void;
  handleDeleteNoteItem: (id: string) => void;
  setViewingNote: (note: ClipNote | null) => void;
}

export const NoteRowItem: React.FC<NoteRowItemProps> = ({
  noteItem,
  index,
  totalNotes,
  noteBoxRef,
  editingNoteId,
  editingNoteText,
  setEditingNoteId,
  setEditingNoteText,
  saveNotes,
  notesRef,
  handleUpdateNoteItem,
  handleMoveNoteUp,
  handleMoveNoteDown,
  handleDeleteNoteItem,
  setViewingNote,
}) => {
  return (
    <Reorder.Item
      key={noteItem.id}
      value={noteItem}
      drag={totalNotes > 1 ? 'y' : false}
      dragConstraints={noteBoxRef}
      dragElastic={0}
      onDragEnd={() => {
        if (totalNotes > 1) {
          saveNotes(notesRef.current);
        }
      }}
      layout="position"
      initial={{ opacity: 0, height: 0 }}
      animate={{ opacity: 1, height: 'auto' }}
      exit={{ opacity: 0, x: -24, scale: 0.95, height: 0 }}
      transition={{ duration: 0 }}
      className={`note-row group min-h-[38px] px-3 py-2 bg-[#171510] hover:bg-[#201d16] flex items-center justify-between space-x-3 border-transparent select-none ${
        totalNotes > 1 ? 'cursor-grab active:cursor-grabbing' : 'cursor-default'
      }`}
    >
      {editingNoteId === noteItem.id ? (
        <div className="flex-1 flex flex-col space-y-2 p-1">
          <textarea
            rows={3}
            value={editingNoteText}
            onChange={(e) => setEditingNoteText(e.target.value)}
            className="w-full bg-transparent border-none outline-none focus:outline-none focus:ring-0 text-xs text-amber-200 resize-y min-h-[60px] note-input font-sans leading-relaxed"
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
          <div className="flex items-start space-x-2 truncate flex-1 select-none py-1">
            {totalNotes > 1 && (
              <GripVertical className="w-3.5 h-3.5 text-amber-400/40 group-hover:text-amber-400 shrink-0 transition-colors mt-0.5 note-icon-btn" />
            )}
            <span className="note-text text-xs text-amber-100 font-normal whitespace-pre-wrap break-words leading-relaxed select-none">
              {noteItem.text}
            </span>
          </div>

          <div className="opacity-40 group-hover:opacity-100 transition-opacity duration-150 flex items-center space-x-1 shrink-0">
            {totalNotes > 1 && (
              <>
                <button
                  type="button"
                  disabled={index === 0}
                  onClick={(e) => {
                    e.stopPropagation();
                    handleMoveNoteUp(index);
                  }}
                  className="note-icon-btn p-1 text-amber-400/70 hover:text-amber-200 disabled:opacity-20 rounded transition-colors"
                  title="Move Note Up"
                >
                  <ArrowUp className="w-3.5 h-3.5" />
                </button>
                <button
                  type="button"
                  disabled={index === totalNotes - 1}
                  onClick={(e) => {
                    e.stopPropagation();
                    handleMoveNoteDown(index);
                  }}
                  className="note-icon-btn p-1 text-amber-400/70 hover:text-amber-200 disabled:opacity-20 rounded transition-colors"
                  title="Move Note Down"
                >
                  <ArrowDown className="w-3.5 h-3.5" />
                </button>
              </>
            )}
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
    </Reorder.Item>
  );
};


