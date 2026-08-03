import { useEffect } from 'react';
import { Copy, StickyNote, X } from 'lucide-react';
import type { ClipNote } from '../types';
import { soundManager } from '../utils/sound';

interface ClipNoteViewerProps {
  note: ClipNote;
  sourceApp: string;
  onClose: () => void;
}

export function ClipNoteViewer({ note, sourceApp, onClose }: ClipNoteViewerProps) {
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  const copyNote = async () => {
    try {
      await navigator.clipboard.writeText(note.text);
      soundManager.playCopySound(true);
    } catch (error) {
      console.error('Failed to copy note:', error);
    }
  };

  return (
    <div className="clip-note-viewer-overlay fixed inset-0 z-[99999] backdrop-blur-md flex items-center justify-center p-4 animate-in fade-in duration-150" role="dialog" aria-modal="true" aria-labelledby="clip-note-viewer-title" onMouseDown={(event) => {
      if (event.target === event.currentTarget) onClose();
    }}>
      <div className="clip-note-viewer-card border rounded-2xl w-full max-w-lg shadow-2xl overflow-hidden flex flex-col max-h-[80vh]">
        <div className="clip-note-viewer-bar px-5 py-3.5 border-b flex items-center justify-between">
          <div className="clip-note-viewer-title flex items-center space-x-2 font-semibold text-sm">
            <StickyNote className="w-4 h-4" />
            <span id="clip-note-viewer-title">Note Annotation</span>
          </div>
          <button type="button" onClick={onClose} className="clip-note-viewer-close p-1 rounded-lg transition-colors" aria-label="Close note viewer">
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="clip-note-viewer-body p-5 overflow-y-auto space-y-3">
          <div className="clip-note-viewer-content border rounded-xl p-4 font-mono text-xs whitespace-pre-wrap leading-relaxed select-text shadow-inner">
            {note.text}
          </div>
          <div className="clip-note-viewer-meta flex items-center justify-between text-[11px] font-sans px-1">
            <span>App Source: <strong className="clip-note-viewer-meta-strong">{sourceApp}</strong></span>
            <span>{note.text.length} Characters</span>
          </div>
        </div>

        <div className="clip-note-viewer-bar px-5 py-3 border-t flex items-center justify-end space-x-2">
          <button type="button" onClick={() => void copyNote()} className="clip-note-viewer-copy flex items-center space-x-1.5 px-3 py-1.5 border rounded-xl text-xs font-semibold transition-colors cursor-pointer">
            <Copy className="w-3.5 h-3.5" />
            <span>Copy Note</span>
          </button>
          <button type="button" onClick={onClose} className="clip-note-viewer-dismiss px-3 py-1.5 rounded-xl text-xs font-semibold transition-colors cursor-pointer">Close</button>
        </div>
      </div>
    </div>
  );
}
