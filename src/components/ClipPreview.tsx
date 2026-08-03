import React, { useState, useEffect, useRef } from 'react';
import { Reorder, AnimatePresence } from 'framer-motion';
import { formatClipDateTime } from '../utils/date';
import { ClipItem, Board, FilterRule, ClipNote, parseClipNotes, serializeClipNotes, ClipVersion } from '../types';
import { parseColor, ColorFormats } from '../utils/color';
import { soundManager } from '../utils/sound';
import { detectSmartFilterRecommendations } from '../utils/smartFilterDetector';
import { startWindowDrag } from '../utils/windowDrag';
import { ClipRevisionHistory } from './ClipRevisionHistory';
import { ClipPreviewContent } from './ClipPreviewContent';
import {
  Copy,
  Check,
  Trash2,
  Sliders,
  Folder,
  FileText,
  StickyNote,
  Eye,
  Edit3,
  Sparkles,
  X,
  ArrowUp,
  ArrowDown,
  GripVertical,
  History,
} from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';

interface ClipPreviewProps {
  clip: ClipItem | null;
  boards: Board[];
  filters: FilterRule[];
  onUpdateClip: () => void;
  onAssignBoard: (clipId: number, boardId: number | null) => void | Promise<void>;
  onDeleteClip: (id: number) => void;
  onUpdateClipNote?: (clipId: number, noteContent: string | null) => void;
}

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

const NoteRowItem: React.FC<NoteRowItemProps> = ({
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

const CLEVER_PLACEHOLDERS = [
  "Add a note before future-you forgets why you copied this...",
  "Jot down your secret brilliance...",
  "What's the tea on this snippet?...",
  "Note to self: Don't lose this thought...",
  "Drop some wisdom, context, or grocery items...",
];

export const ClipPreview: React.FC<ClipPreviewProps> = ({
  clip,
  boards,
  filters,
  onUpdateClip,
  onAssignBoard,
  onDeleteClip,
  onUpdateClipNote,
}) => {
  const [copied, setCopied] = useState(false);
  const [copiedFormat, setCopiedFormat] = useState<string | null>(null);
  const [transformedText, setTransformedText] = useState<string | null>(null);
  const [activeFilterName, setActiveFilterName] = useState<string | null>(null);
  const [notes, setNotes] = useState<ClipNote[]>(() => parseClipNotes(clip?.note));
  const notesRef = useRef(notes);
  const noteBoxRef = useRef<HTMLDivElement>(null);
  const copiedTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const copiedFormatTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    notesRef.current = notes;
  }, [notes]);

  const [isAddingNote, setIsAddingNote] = useState<boolean>(false);
  const [newNoteText, setNewNoteText] = useState<string>('');
  const [placeholderText, setPlaceholderText] = useState<string>(CLEVER_PLACEHOLDERS[0]);
  const [editingNoteId, setEditingNoteId] = useState<string | null>(null);
  const [editingNoteText, setEditingNoteText] = useState<string>('');
  const [viewingNote, setViewingNote] = useState<ClipNote | null>(null);
  const [isOcrLoading, setIsOcrLoading] = useState<boolean>(false);
  const [resolvedImageBase64, setResolvedImageBase64] = useState<string | null>(clip?.image_base64 || null);
  const [showHistory, setShowHistory] = useState<boolean>(false);
  const [versions, setVersions] = useState<ClipVersion[]>([]);
  const [isHistoryLoading, setIsHistoryLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    if (clip && showHistory) {
      setVersions([]);
      setIsHistoryLoading(true);
      invoke<ClipVersion[]>('get_clip_versions', { clipId: clip.id })
        .then((res) => {
          if (!cancelled) setVersions(Array.isArray(res) ? res : []);
        })
        .catch((e) => console.error('Failed to load clip versions:', e))
        .finally(() => {
          if (!cancelled) setIsHistoryLoading(false);
        });
    } else {
      setIsHistoryLoading(false);
    }
    return () => {
      cancelled = true;
    };
  }, [clip?.id, showHistory]);

  useEffect(() => {
    let cancelled = false;
    if (clip?.content_type === 'image') {
      if (clip.image_base64) {
        setResolvedImageBase64(clip.image_base64);
      } else {
        setResolvedImageBase64(null);
        invoke<string | null>('get_clip_image', { id: clip.id })
          .then((b64) => {
            if (!cancelled) setResolvedImageBase64(b64);
          })
          .catch(console.error);
      }
    } else {
      setResolvedImageBase64(null);
    }
    return () => {
      cancelled = true;
    };
  }, [clip?.id, clip?.image_base64, clip?.content_type]);

  useEffect(() => {
    setTransformedText(null);
    setActiveFilterName(null);
    setShowHistory(false);
    const parsed = parseClipNotes(clip?.note);
    setNotes(parsed);
    notesRef.current = parsed;
    setIsAddingNote(false);
    setNewNoteText('');
    setEditingNoteId(null);
    setEditingNoteText('');
    setViewingNote(null);
    setIsOcrLoading(false);
    setCopied(false);
    setCopiedFormat(null);
    setVersions([]);
    setIsHistoryLoading(false);
    if (copiedTimerRef.current) clearTimeout(copiedTimerRef.current);
    if (copiedFormatTimerRef.current) clearTimeout(copiedFormatTimerRef.current);
  }, [clip]);

  useEffect(() => () => {
    if (copiedTimerRef.current) clearTimeout(copiedTimerRef.current);
    if (copiedFormatTimerRef.current) clearTimeout(copiedFormatTimerRef.current);
  }, []);

  const saveNotes = (updatedNotes: ClipNote[]) => {
    if (!clip) return;
    setNotes(updatedNotes);
    notesRef.current = updatedNotes;
    const serialized = serializeClipNotes(updatedNotes);
    if (onUpdateClipNote) {
      onUpdateClipNote(clip.id, serialized);
    }
    invoke('update_clip_note', {
      clipId: clip.id,
      note: serialized,
    }).catch((e) => console.error('Failed to update clip note:', e));
  };

  const handleCreateNote = () => {
    if (!newNoteText.trim()) return;
    const newNote: ClipNote = {
      id: `note-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
      text: newNoteText.trim(),
      created_at: new Date().toISOString(),
    };
    const updated = [...notes, newNote];
    setNewNoteText('');
    setIsAddingNote(false);
    saveNotes(updated);
  };

  const handleUpdateNoteItem = (id: string, text: string) => {
    const updated = notes
      .map((n) => (n.id === id ? { ...n, text: text.trim() } : n))
      .filter((n) => n.text.length > 0);
    setEditingNoteId(null);
    saveNotes(updated);
  };

  const handleDeleteNoteItem = (id: string) => {
    const updated = notes.filter((n) => n.id !== id);
    saveNotes(updated);
  };

  const handleMoveNoteUp = (index: number) => {
    if (index === 0) return;
    const reordered = [...notes];
    const [item] = reordered.splice(index, 1);
    reordered.splice(index - 1, 0, item);
    saveNotes(reordered);
  };

  const handleMoveNoteDown = (index: number) => {
    if (index === notes.length - 1) return;
    const reordered = [...notes];
    const [item] = reordered.splice(index, 1);
    reordered.splice(index + 1, 0, item);
    saveNotes(reordered);
  };

  const handleRunOCR = async () => {
    if (!clip) return;
    setIsOcrLoading(true);
    try {
      await invoke<string>('extract_ocr_from_clip', { clipId: clip.id });
      soundManager.playCopySound(true);
      onUpdateClip();
    } catch (e) {
      console.error('OCR Extraction Failed:', e);
    } finally {
      setIsOcrLoading(false);
    }
  };

  if (!clip) {
    return (
      <div className="flex-1 col-preview h-screen flex flex-col items-center justify-center text-gray-500 bg-[#212121] p-8 select-none border-l border-[#2b2b2b]">
        <div className="w-16 h-16 rounded-2xl bg-[#181818] border border-gray-700/60 flex items-center justify-center mb-4 shadow-xl">
          <FileText className="w-8 h-8 text-gray-400" />
        </div>
        <p className="text-sm font-medium text-gray-300">No Clip Selected</p>
        <p className="text-xs text-gray-500 mt-1 max-w-xs text-center">
          Select an item from history or right-click to copy, filter, add notes, or organize.
        </p>
      </div>
    );
  }

  const displayText = transformedText ?? clip.text_content ?? '';
  const colorData: ColorFormats | null =
    clip.content_type === 'color' || (displayText && displayText.length < 30)
      ? parseColor(displayText)
      : null;

  const handleCopy = async () => {
    try {
      await invoke('copy_clip_to_system', {
        text: displayText,
        imageBase64: clip.content_type === 'image' ? resolvedImageBase64 : null,
      });
      setCopied(true);
      if (copiedTimerRef.current) clearTimeout(copiedTimerRef.current);
      copiedTimerRef.current = setTimeout(() => setCopied(false), 2000);
    } catch (e) {
      console.error(e);
    }
  };

  const handleCopySpecificFormat = async (label: string, value: string) => {
    try {
      await invoke('copy_clip_to_system', { text: value, imageBase64: null });
      setCopiedFormat(label);
      soundManager.playCopySound(true);
      if (copiedFormatTimerRef.current) clearTimeout(copiedFormatTimerRef.current);
      copiedFormatTimerRef.current = setTimeout(() => setCopiedFormat(null), 2000);
    } catch (e) {
      console.error(e);
    }
  };

  const handleApplyFilter = async (filter: FilterRule) => {
    if (!clip.text_content) return;
    try {
      const res = await invoke<string>('transform_text', {
        input: clip.text_content,
        filterType: filter.filter_type,
        config: filter.config,
      });
      setTransformedText(res);
      setActiveFilterName(filter.name);
    } catch (e) {
      console.error(e);
    }
  };

  const handleResetTransform = () => {
    setTransformedText(null);
    setActiveFilterName(null);
  };

  const handleAssignBoard = async (boardId: number | null) => {
    try {
      await onAssignBoard(clip.id, boardId);
    } catch (e) {
      console.error(e);
    }
  };

  const handleRestoreVersion = async (version: ClipVersion) => {
    try {
      await invoke('update_clip_text', { clipId: clip.id, text: version.text_content });
      setTransformedText(null);
      setActiveFilterName(null);
      setShowHistory(false);
      soundManager.playCopySound(true);
      onUpdateClip();
    } catch (error) {
      console.error('Failed to restore clip version:', error);
    }
  };

  const charCount = displayText.length;
  const wordCount = displayText.trim() ? displayText.trim().split(/\s+/).length : 0;
  const lineCount = displayText ? displayText.split('\n').length : 0;

  return (
    <div className="flex-1 col-preview h-screen flex flex-col bg-[#212121] border-l border-[#2b2b2b] overflow-hidden">
      {/* Finder Top Header Bar */}
      <div
        onMouseDown={startWindowDrag}
        className="h-[60px] border-b border-[#2b2b2b] bg-[#171717]/80 backdrop-blur-md px-4 flex items-center justify-between cursor-default titlebar-drag-handle shrink-0"
      >
        <div className="flex items-center space-x-3 titlebar-drag-handle">
          <span className="clip-type-badge text-xs font-semibold px-2.5 py-1 rounded-md bg-gray-800 text-gray-200 border border-gray-700 capitalize titlebar-drag-handle">
            {clip.content_type}
          </span>
          <span className="text-xs text-gray-300 font-medium truncate max-w-[200px] titlebar-drag-handle">
            {clip.source_app}
          </span>
        </div>

        <div className="flex items-center space-x-2 titlebar-no-drag">
          <button
            onClick={() => setShowHistory((prev) => !prev)}
            className={`flex items-center space-x-1 px-2.5 py-1.5 rounded-lg text-xs font-semibold border transition-all cursor-pointer ${
              showHistory
                ? 'bg-purple-900/80 text-purple-200 border-purple-500/50 shadow-md'
                : 'bg-gray-800/80 text-gray-300 border-gray-700 hover:text-white hover:bg-gray-700'
            }`}
            title="Revision History & Diffs"
          >
            <History className="w-3.5 h-3.5 text-purple-400" />
            <span>History</span>
          </button>

          <button
            onClick={handleCopy}
            className="copy-clip-main-btn flex items-center space-x-1.5 px-3.5 py-1.5 rounded-lg text-xs font-semibold shadow-md active:scale-95 transition-all"
          >
            {copied ? (
              <>
                <Check className="w-3.5 h-3.5 text-emerald-600" />
                <span>Copied!</span>
              </>
            ) : (
              <>
                <Copy className="w-3.5 h-3.5" />
                <span>Copy Clip</span>
              </>
            )}
          </button>

          <button
            onClick={() => onDeleteClip(clip.id)}
            className="p-1.5 text-gray-400 hover:text-red-400 hover:bg-gray-800 hover:scale-110 active:scale-95 rounded-lg transition-all"
            title="Delete Clip"
          >
            <Trash2 className="w-4 h-4" />
          </button>
        </div>
      </div>

      {showHistory && (
        <ClipRevisionHistory
          versions={versions}
          isLoading={isHistoryLoading}
          onClose={() => setShowHistory(false)}
          onRestore={(version) => void handleRestoreVersion(version)}
        />
      )}

      {/* Smart Recommended Actions Bar */}
      {(() => {
        const currentText = transformedText !== null ? transformedText : (clip.text_content || '');
        const { detectedTypes, recommendedFilters } = detectSmartFilterRecommendations(currentText, filters);
        if (recommendedFilters.length === 0) return null;

        return (
          <div className="px-4 py-2 bg-cyan-950/30 border-b border-cyan-800/40 flex items-center justify-between text-xs space-x-2 overflow-x-auto">
            <div className="flex items-center space-x-1.5 shrink-0 text-cyan-400 font-semibold text-[11px]">
              <Sparkles className="w-3.5 h-3.5 text-cyan-400 animate-pulse" />
              <span>Smart Actions ({detectedTypes.join(', ')}):</span>
            </div>
            <div className="flex items-center space-x-1.5 overflow-x-auto scrollbar-none py-0.5">
              {recommendedFilters.map((f) => (
                <button
                  key={f.id}
                  onClick={() => handleApplyFilter(f)}
                  className="px-2 py-0.5 rounded-md bg-cyan-500/20 hover:bg-cyan-500/30 text-cyan-200 border border-cyan-500/40 text-[11px] font-medium transition-all flex items-center space-x-1 whitespace-nowrap shadow-sm hover:scale-105"
                  title={`Apply ${f.name}`}
                >
                  <span>{f.name}</span>
                </button>
              ))}
            </div>
          </div>
        );
      })()}

      {/* Quick Bin Assignment & Note Section */}
      <div className="preview-bin-bar px-4 py-2 flex items-center justify-between text-xs border-b">
        <div className="flex items-center space-x-2">
          <Folder className="w-3.5 h-3.5" />
          <span>Bin:</span>
          <select
            value={clip.board_id ?? ''}
            onChange={(e) => {
              const val = e.target.value;
              handleAssignBoard(val ? Number(val) : null);
            }}
            className="bg-[#181818] border border-gray-700 text-gray-200 text-xs rounded-md px-2 py-1 focus:outline-none focus:border-gray-500"
          >
            <option value="">– None –</option>
            {boards.filter((b) => b.board_type !== 'tag' && !b.smart_rule).map((b) => (
              <option key={b.id} value={b.id}>
                {b.name}
              </option>
            ))}
          </select>
        </div>

        <div className="flex items-center space-x-2">
          <button
            onClick={() => {
              if (!isAddingNote) {
                const nextIdx = Math.floor(Math.random() * CLEVER_PLACEHOLDERS.length);
                setPlaceholderText(CLEVER_PLACEHOLDERS[nextIdx]);
              }
              setIsAddingNote(!isAddingNote);
            }}
            className="add-note-btn flex items-center space-x-1.5 px-3 py-1 rounded-md bg-amber-500/20 text-amber-300 border border-amber-500/30 hover:bg-amber-500/30 text-xs font-semibold transition-all cursor-pointer"
          >
            <StickyNote className="w-3.5 h-3.5" />
            <span>+ Add Note</span>
          </button>
        </div>
      </div>

      {/* Multi-Note Container (Inline Input Row, Framer Motion Animated Reordering, Non-Selectable) */}
      {(notes.length > 0 || isAddingNote) && (
        <div className="px-4 py-2.5 bg-amber-950/20 border-b border-amber-500/20 space-y-2 note-container select-none">
          <div className="note-header-text flex items-center space-x-1.5 text-[11px] font-semibold text-amber-400 uppercase tracking-wider select-none">
            <StickyNote className="w-3.5 h-3.5" />
            <span>Notes ({notes.length})</span>
          </div>

          <div ref={noteBoxRef} className="note-row-box relative rounded-xl border border-amber-500/30 overflow-hidden shadow-sm bg-[#171510] divide-y divide-amber-500/20">
            <Reorder.Group
              axis="y"
              values={notes}
              onReorder={(newOrder) => {
                setNotes(newOrder);
              }}
              className="divide-y divide-amber-500/20"
            >
              <AnimatePresence initial={false}>
                {notes.map((noteItem, index) => (
                  <NoteRowItem
                    key={noteItem.id}
                    noteItem={noteItem}
                    index={index}
                    totalNotes={notes.length}
                    noteBoxRef={noteBoxRef}
                    editingNoteId={editingNoteId}
                    editingNoteText={editingNoteText}
                    setEditingNoteId={setEditingNoteId}
                    setEditingNoteText={setEditingNoteText}
                    saveNotes={saveNotes}
                    notesRef={notesRef}
                    handleUpdateNoteItem={handleUpdateNoteItem}
                    handleMoveNoteUp={handleMoveNoteUp}
                    handleMoveNoteDown={handleMoveNoteDown}
                    handleDeleteNoteItem={handleDeleteNoteItem}
                    setViewingNote={setViewingNote}
                  />
                ))}
              </AnimatePresence>
            </Reorder.Group>

            {/* Inline New Multiline Note Input Drawer */}
            {isAddingNote && (
              <div className="note-input-row p-3 bg-[#221f17] flex flex-col space-y-2 border-t border-amber-500/20 animate-in fade-in duration-100">
                <textarea
                  rows={3}
                  placeholder={placeholderText}
                  value={newNoteText}
                  onChange={(e) => setNewNoteText(e.target.value)}
                  className="w-full bg-transparent border-none outline-none focus:outline-none focus:ring-0 text-xs text-amber-100 placeholder-amber-400/50 resize-y min-h-[60px] note-input font-sans leading-relaxed"
                  autoFocus
                  onKeyDown={(e) => {
                    if (e.key === 'Escape') {
                      setIsAddingNote(false);
                      setNewNoteText('');
                    }
                  }}
                />
                <div className="flex items-center justify-end space-x-2 pt-1">
                  <button
                    type="button"
                    onClick={() => {
                      setIsAddingNote(false);
                      setNewNoteText('');
                    }}
                    className="px-3 py-1 bg-[#2c2921] hover:bg-[#3a362c] text-gray-300 rounded-md text-xs font-medium transition-colors cursor-pointer"
                  >
                    Cancel
                  </button>
                  <button
                    type="button"
                    onClick={handleCreateNote}
                    className="flex items-center space-x-1 px-3 py-1 bg-amber-600 hover:bg-amber-500 text-white rounded-md text-xs font-semibold shadow cursor-pointer"
                  >
                    <Check className="w-3.5 h-3.5" />
                    <span>Save</span>
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Main Preview Workspace */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4 font-mono text-xs">
        {activeFilterName && (
          <div className="flex items-center justify-between px-3 py-2 bg-indigo-500/20 border border-indigo-500/40 rounded-lg text-indigo-300">
            <div className="flex items-center space-x-2">
              <Sliders className="w-4 h-4 text-cyan-400" />
              <span>Filtered via: <strong>{activeFilterName}</strong></span>
            </div>
            <button
              onClick={handleResetTransform}
              className="text-xs underline hover:text-white"
            >
              Reset
            </button>
          </div>
        )}

        <ClipPreviewContent
          clip={clip}
          displayText={displayText}
          colorData={colorData}
          resolvedImageBase64={resolvedImageBase64}
          copiedFormat={copiedFormat}
          isOcrLoading={isOcrLoading}
          onColorChange={setTransformedText}
          onCopyFormat={(label, value) => void handleCopySpecificFormat(label, value)}
          onRunOCR={() => void handleRunOCR()}
        />

      </div>

      {/* Sleek Filter Pipeline Selector Bar */}
      {clip.content_type !== 'image' && filters.length > 0 && (
        <div className="px-4 py-2.5 bg-[#171717] border-t border-[#2b2b2b] select-none">
          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center space-x-2 shrink-0">
              <Sliders className="w-4 h-4 text-cyan-400" />
              <span className="text-xs font-semibold text-gray-300">Apply Filter:</span>
            </div>

            {/* Dropdown Selector */}
            <div className="flex-1 max-w-xs relative">
              <select
                value={activeFilterName || ''}
                onChange={(e) => {
                  const selectedName = e.target.value;
                  if (!selectedName) {
                    handleResetTransform();
                  } else {
                    const found = filters.find((f) => f.name === selectedName);
                    if (found) handleApplyFilter(found);
                  }
                }}
                className={`w-full bg-[#181818] border text-xs rounded-xl px-3 py-1.5 focus:outline-none transition-all cursor-pointer font-medium ${
                  activeFilterName
                    ? 'border-cyan-500 text-cyan-300 bg-cyan-950/40 ring-1 ring-cyan-500/40'
                    : 'border-gray-700/80 text-gray-300 hover:border-gray-600'
                }`}
              >
                <option value="">⚡ Raw Original Text (No Filter)</option>
                {filters.map((f) => (
                  <option key={f.id} value={f.name}>
                    {f.name}
                  </option>
                ))}
              </select>
            </div>

            {/* Reset Action */}
            {activeFilterName && (
              <button
                onClick={handleResetTransform}
                className="flex items-center space-x-1 px-2.5 py-1 rounded-lg bg-cyan-950/60 hover:bg-cyan-900/80 border border-cyan-700/60 text-cyan-300 text-xs font-semibold transition-all shrink-0"
                title="Reset to raw clip text"
              >
                <span>Reset</span>
              </button>
            )}
          </div>
        </div>
      )}

      {/* Stats Footer */}
      <div className="px-4 py-2.5 bg-[#171717] border-t border-[#2b2b2b] flex items-center justify-between text-[11px] text-gray-500">
        <div className="flex items-center space-x-4">
          <span>Chars: <strong className="text-gray-300">{charCount}</strong></span>
          <span>Words: <strong className="text-gray-300">{wordCount}</strong></span>
          <span>Lines: <strong className="text-gray-300">{lineCount}</strong></span>
        </div>
        <div>
          <span>Captured: {formatClipDateTime(clip.created_at)}</span>
        </div>
      </div>

      {/* Dedicated Full Note Viewer Modal */}
      {viewingNote && clip && (
        <div className="fixed inset-0 z-[99999] bg-black/75 backdrop-blur-md flex items-center justify-center p-4 animate-in fade-in duration-150">
          <div className="bg-[#1a1813] border border-amber-500/40 rounded-2xl w-full max-w-lg shadow-2xl overflow-hidden flex flex-col max-h-[80vh]">
            {/* Modal Header */}
            <div className="px-5 py-3.5 border-b border-amber-500/20 bg-[#14120e] flex items-center justify-between">
              <div className="flex items-center space-x-2 text-amber-400 font-semibold text-sm">
                <StickyNote className="w-4 h-4" />
                <span>Note Annotation</span>
              </div>
              <button
                type="button"
                onClick={() => setViewingNote(null)}
                className="p-1 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg transition-colors"
                title="Close"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            {/* Modal Body */}
            <div className="p-5 overflow-y-auto space-y-3">
              <div className="bg-[#12100c] border border-amber-500/30 rounded-xl p-4 text-amber-100 font-mono text-xs whitespace-pre-wrap leading-relaxed select-text shadow-inner">
                {viewingNote.text}
              </div>

              <div className="flex items-center justify-between text-[11px] text-amber-400/70 font-sans px-1">
                <span>App Source: <strong className="text-amber-200">{clip.source_app}</strong></span>
                <span>{viewingNote.text.length} Characters</span>
              </div>
            </div>

            {/* Modal Footer */}
            <div className="px-5 py-3 border-t border-amber-500/20 bg-[#14120e] flex items-center justify-end space-x-2">
              <button
                type="button"
                onClick={() => {
                  navigator.clipboard.writeText(viewingNote.text || '');
                  soundManager.playCopySound(true);
                }}
                className="flex items-center space-x-1.5 px-3 py-1.5 bg-amber-950/80 hover:bg-amber-900 text-amber-300 border border-amber-700/50 rounded-xl text-xs font-semibold transition-all cursor-pointer"
              >
                <Copy className="w-3.5 h-3.5" />
                <span>Copy Note</span>
              </button>
              <button
                type="button"
                onClick={() => setViewingNote(null)}
                className="px-3 py-1.5 bg-[#26231c] hover:bg-[#343026] text-amber-200 rounded-xl text-xs font-semibold transition-colors cursor-pointer"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
