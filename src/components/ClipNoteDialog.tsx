import { StickyNote } from 'lucide-react';
import type { ClipItem } from '../types';

interface ClipNoteDialogProps {
  clip: ClipItem;
  text: string;
  onTextChange: (text: string) => void;
  onCancel: () => void;
  onSave: (clip: ClipItem, note: string | null) => void | Promise<void>;
}

export function ClipNoteDialog({ clip, text, onTextChange, onCancel, onSave }: ClipNoteDialogProps) {
  return (
    <div className="app-dialog-overlay fixed inset-0 flex items-center justify-center p-4 animate-in fade-in duration-150 select-none">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="clip-note-title"
        className="app-dialog-panel theme-panel border rounded-2xl p-5 max-w-md w-full space-y-4 font-sans"
      >
        <div className="flex items-center space-x-3">
          <div className="p-2.5 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-400 shrink-0">
            <StickyNote className="w-5 h-5" />
          </div>
          <div>
            <h3 id="clip-note-title" className="theme-title text-sm font-bold">{clip.note ? 'Edit Clip Note' : 'Add Note to Clip'}</h3>
            <p className="theme-text-muted text-xs mt-0.5">Attach custom annotations or metadata to this clip.</p>
          </div>
        </div>

        <textarea
          value={text}
          onChange={(event) => onTextChange(event.target.value)}
          placeholder="Type your note here..."
          rows={4}
          autoFocus
          className="app-dialog-input theme-input w-full border rounded-xl p-3 text-xs focus:outline-none focus:border-amber-500 transition-colors resize-none font-sans"
        />

        <div className="flex justify-end space-x-2">
          <button
            type="button"
            onClick={onCancel}
            className="app-dialog-cancel theme-secondary-button px-4 py-1.5 rounded-xl border text-xs font-semibold transition-colors cursor-pointer"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={() => onSave(clip, text.trim() || null)}
            className="px-4 py-1.5 rounded-xl bg-amber-600 hover:bg-amber-500 text-white text-xs font-semibold transition-colors shadow-md cursor-pointer"
          >
            Save Note
          </button>
        </div>
      </div>
    </div>
  );
}
