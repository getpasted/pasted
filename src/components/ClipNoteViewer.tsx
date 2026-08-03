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
    <div className="fixed inset-0 z-[99999] bg-black/75 backdrop-blur-md flex items-center justify-center p-4 animate-in fade-in duration-150" role="dialog" aria-modal="true" aria-labelledby="clip-note-viewer-title" onMouseDown={(event) => {
      if (event.target === event.currentTarget) onClose();
    }}>
      <div className="bg-[#1a1813] border border-amber-500/40 rounded-2xl w-full max-w-lg shadow-2xl overflow-hidden flex flex-col max-h-[80vh]">
        <div className="px-5 py-3.5 border-b border-amber-500/20 bg-[#14120e] flex items-center justify-between">
          <div className="flex items-center space-x-2 text-amber-400 font-semibold text-sm">
            <StickyNote className="w-4 h-4" />
            <span id="clip-note-viewer-title">Note Annotation</span>
          </div>
          <button type="button" onClick={onClose} className="p-1 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg transition-colors" aria-label="Close note viewer">
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="p-5 overflow-y-auto space-y-3">
          <div className="bg-[#12100c] border border-amber-500/30 rounded-xl p-4 text-amber-100 font-mono text-xs whitespace-pre-wrap leading-relaxed select-text shadow-inner">
            {note.text}
          </div>
          <div className="flex items-center justify-between text-[11px] text-amber-400/70 font-sans px-1">
            <span>App Source: <strong className="text-amber-200">{sourceApp}</strong></span>
            <span>{note.text.length} Characters</span>
          </div>
        </div>

        <div className="px-5 py-3 border-t border-amber-500/20 bg-[#14120e] flex items-center justify-end space-x-2">
          <button type="button" onClick={() => void copyNote()} className="flex items-center space-x-1.5 px-3 py-1.5 bg-amber-950/80 hover:bg-amber-900 text-amber-300 border border-amber-700/50 rounded-xl text-xs font-semibold transition-all cursor-pointer">
            <Copy className="w-3.5 h-3.5" />
            <span>Copy Note</span>
          </button>
          <button type="button" onClick={onClose} className="px-3 py-1.5 bg-[#26231c] hover:bg-[#343026] text-amber-200 rounded-xl text-xs font-semibold transition-colors cursor-pointer">Close</button>
        </div>
      </div>
    </div>
  );
}
