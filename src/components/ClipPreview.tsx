import React, { useState, useEffect, useRef } from 'react';
import { formatClipDateTime } from '../utils/date';
import { ClipItem, Bin, FilterRule, ClipNote, parseClipNotes, serializeClipNotes, ClipVersion } from '../types';
import { parseColor, ColorFormats } from '../utils/color';
import { soundManager } from '../utils/sound';
import { detectSmartFilterRecommendations } from '../utils/smartFilterDetector';
import { startWindowDrag } from '../utils/windowDrag';
import { ClipRevisionHistory } from './ClipRevisionHistory';
import { ClipPreviewContent } from './ClipPreviewContent';
import { ClipNoteViewer } from './ClipNoteViewer';
import { NoteRowItem } from './ClipNoteRow';
import {
  Copy,
  Check,
  Trash2,
  Sliders,
  Folder,
  FileText,
  StickyNote,
  Sparkles,
} from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { useStableVerticalReorder } from '../hooks/useStableVerticalReorder';
import type { ClipViewPolicy } from '../utils/clipViewPolicy';

interface ClipPreviewProps {
  clip: ClipItem | null;
  viewPolicy: ClipViewPolicy;
  bins: Bin[];
  filters: FilterRule[];
  onUpdateClip: () => void;
  onAssignBin: (clipId: number, binId: number | null) => void | Promise<void>;
  onDeleteClip: (id: number) => void;
  onUpdateClipNote?: (clipId: number, noteContent: string | null) => void;
}

const CLEVER_PLACEHOLDERS = [
  "Add a note before future-you forgets why you copied this...",
  "Jot down your secret brilliance...",
  "What's the tea on this snippet?...",
  "Note to self: Don't lose this thought...",
  "Drop some wisdom, context, or grocery items...",
];

export const ClipPreview: React.FC<ClipPreviewProps> = ({
  clip,
  viewPolicy,
  bins,
  filters,
  onUpdateClip,
  onAssignBin,
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
  const [revisionCount, setRevisionCount] = useState<number | null>(null);
  const [isHistoryLoading, setIsHistoryLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    if (!clip) {
      setRevisionCount(null);
      return () => {
        cancelled = true;
      };
    }

    setRevisionCount(null);
    invoke<number>('get_clip_version_count', { clipId: clip.id })
      .then((count) => {
        if (!cancelled) setRevisionCount(Number.isFinite(count) ? count : 0);
      })
      .catch((error) => console.error('Failed to load clip revision count:', error));

    return () => {
      cancelled = true;
    };
  }, [clip?.id]);

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

  useEffect(() => {
    if (viewPolicy.canEditNotes) return;
    setIsAddingNote(false);
    setNewNoteText('');
    setEditingNoteId(null);
    setEditingNoteText('');
  }, [viewPolicy.canEditNotes]);

  useEffect(() => {
    if (viewPolicy.canApplyFilters) return;
    setTransformedText(null);
    setActiveFilterName(null);
  }, [viewPolicy.canApplyFilters]);

  useEffect(() => () => {
    if (copiedTimerRef.current) clearTimeout(copiedTimerRef.current);
    if (copiedFormatTimerRef.current) clearTimeout(copiedFormatTimerRef.current);
  }, []);

  const saveNotes = (updatedNotes: ClipNote[]) => {
    if (!clip || !viewPolicy.canEditNotes) return;
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

  const {
    activeId: activeNoteId,
    offsets: noteReorderOffsets,
    isSettling: isNoteReorderSettling,
    startPointerReorder: startNotePointerReorder,
  } = useStableVerticalReorder({
    itemIds: notes.map((note) => note.id),
    containerRef: noteBoxRef,
    disabled: !viewPolicy.canEditNotes || notes.length < 2 || editingNoteId !== null,
    onCommit: (orderedIds) => {
      const byId = new Map(notesRef.current.map((note) => [note.id, note]));
      const reordered = orderedIds
        .map((id) => byId.get(id))
        .filter((note): note is ClipNote => Boolean(note));
      saveNotes(reordered);
    },
  });

  const handleCreateNote = () => {
    if (!viewPolicy.canEditNotes || !newNoteText.trim()) return;
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
    if (!viewPolicy.canEditNotes) return;
    const updated = notes
      .map((n) => (n.id === id ? { ...n, text: text.trim() } : n))
      .filter((n) => n.text.length > 0);
    setEditingNoteId(null);
    saveNotes(updated);
  };

  const handleDeleteNoteItem = (id: string) => {
    if (!viewPolicy.canEditNotes) return;
    const updated = notes.filter((n) => n.id !== id);
    saveNotes(updated);
  };

  const handleRunOCR = async () => {
    if (!clip || !viewPolicy.canMutateContent) return;
    setIsOcrLoading(true);
    try {
      await invoke<string>('extract_ocr_from_clip', { clipId: clip.id });
      invoke<number>('get_clip_version_count', { clipId: clip.id })
        .then(setRevisionCount)
        .catch((error) => console.error('Failed to refresh clip revision count:', error));
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
      <div className="clip-preview-empty flex-1 col-preview h-screen flex flex-col items-center justify-center p-8 select-none">
        <div className="clip-preview-empty-icon theme-surface w-16 h-16 rounded-2xl border flex items-center justify-center mb-4 shadow-xl">
          <FileText className="w-8 h-8" />
        </div>
        <p className="theme-text-main text-sm font-medium">No Clip Selected</p>
        <p className="theme-text-muted text-xs mt-1 max-w-xs text-center">
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

  const handleAssignBin = async (binId: number | null) => {
    if (!viewPolicy.canAssignBins) return;
    try {
      await onAssignBin(clip.id, binId);
    } catch (e) {
      console.error(e);
    }
  };

  const handleRestoreVersion = async (version: ClipVersion) => {
    if (!viewPolicy.canMutateContent) return;
    try {
      await invoke('update_clip_text', { clipId: clip.id, text: version.text_content });
      invoke<number>('get_clip_version_count', { clipId: clip.id })
        .then(setRevisionCount)
        .catch((error) => console.error('Failed to refresh clip revision count:', error));
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
    <div className="flex-1 col-preview h-screen flex flex-col overflow-hidden">
      {/* Finder Top Header Bar */}
      <div
        onMouseDown={startWindowDrag}
        className="col-preview-header h-[60px] px-4 flex items-center justify-between cursor-default titlebar-drag-handle shrink-0"
      >
        <div className="flex items-center space-x-3 titlebar-drag-handle">
          <span className="clip-type-badge theme-badge text-xs font-semibold px-2.5 py-1 rounded-md border capitalize titlebar-drag-handle">
            {clip.content_type}
          </span>
          <span className="theme-text-main text-xs font-medium truncate max-w-[200px] titlebar-drag-handle">
            {clip.source_app}
          </span>
        </div>

        <div className="flex items-center space-x-2 titlebar-no-drag">
          <button
            onClick={handleCopy}
            className="copy-clip-main-btn flex items-center space-x-1.5 px-3.5 py-1.5 rounded-lg text-xs font-semibold shadow-md active:scale-95 transition-[background-color,border-color,color,transform]"
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
            className="preview-delete-btn p-1.5 hover:scale-110 active:scale-95 rounded-lg transition-[background-color,color,transform]"
            title={viewPolicy.state === 'trash' ? 'Delete Permanently' : 'Delete Clip'}
          >
            <Trash2 className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Quick Bin Assignment & Note Section */}
      {viewPolicy.canOrganize ? (
      <div className="preview-bin-bar px-4 py-2 flex items-center justify-between text-xs border-b">
        <div className="flex items-center space-x-2">
          <Folder className="w-3.5 h-3.5" />
          <span>Bin:</span>
          <select
            value={clip.bin_id ?? ''}
            onChange={(e) => {
              const val = e.target.value;
              handleAssignBin(val ? Number(val) : null);
            }}
            className="theme-input form-field-valid border text-xs rounded-md px-2 py-1 focus:outline-none"
          >
            <option value="">– None –</option>
            {bins.filter((b) => b.bin_type !== 'tag' && !b.smart_rule).map((b) => (
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
            className="add-note-btn flex items-center space-x-1.5 px-3 py-1 rounded-md border text-xs font-semibold transition-colors cursor-pointer"
          >
            <StickyNote className="w-3.5 h-3.5" />
            <span>+ Add Note</span>
          </button>
        </div>
      </div>
      ) : (
        <div className="preview-bin-bar px-4 py-2 flex items-center justify-between text-xs border-b" role="note">
          <div className="preview-readonly-notice flex items-center space-x-2">
            <Trash2 className="w-3.5 h-3.5" />
            <span>Restore this clip to organize it or edit its notes.</span>
          </div>
        </div>
      )}

      {/* Multi-Note Container (Inline Input Row, Stable Animated Reordering, Non-Selectable) */}
      {(notes.length > 0 || isAddingNote) && (
        <div className="px-4 py-2.5 border-b space-y-2 note-container select-none">
          <div className="note-header-text flex items-center space-x-1.5 text-[11px] font-semibold uppercase tracking-wider select-none">
            <StickyNote className="w-3.5 h-3.5" />
            <span>Notes ({notes.length})</span>
          </div>

          <div
            ref={noteBoxRef}
            className={`note-row-stack relative space-y-2 ${isNoteReorderSettling ? 'is-settling-stable-reorder' : ''}`}
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
                    handleUpdateNoteItem={handleUpdateNoteItem}
                    handleDeleteNoteItem={handleDeleteNoteItem}
                    setViewingNote={setViewingNote}
                    readOnly={!viewPolicy.canEditNotes}
                    isDragging={activeNoteId === noteItem.id}
                    reorderOffsetY={noteReorderOffsets[noteItem.id] ?? 0}
                    onReorderPointerDown={(event) => startNotePointerReorder(noteItem.id, event)}
                  />
                ))}

            {/* Inline New Note Card */}
            {isAddingNote && (
              <div className="note-input-row p-3 rounded-lg border flex flex-col space-y-2 animate-in fade-in duration-100">
                <textarea
                  rows={3}
                  placeholder={placeholderText}
                  value={newNoteText}
                  onChange={(e) => setNewNoteText(e.target.value)}
                  className="w-full bg-transparent border-none outline-none focus:outline-none focus:ring-0 text-xs resize-y min-h-[60px] note-input font-sans leading-relaxed"
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
                    className="note-cancel-button px-3 py-1 rounded-md text-xs font-medium transition-colors cursor-pointer"
                  >
                    Cancel
                  </button>
                  <button
                    type="button"
                    onClick={handleCreateNote}
                    className="note-save-button flex items-center space-x-1 px-3 py-1 rounded-md text-xs font-semibold shadow cursor-pointer"
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
      <div className="clip-preview-workspace flex-1 overflow-y-auto p-4 space-y-4 font-mono text-xs">
        {activeFilterName && (
          <div className="active-filter-banner flex items-center justify-between px-3 py-2 border rounded-lg">
            <div className="flex items-center space-x-2">
              <Sliders className="w-4 h-4" />
              <span>Filtered via: <strong>{activeFilterName}</strong></span>
            </div>
            <button
              onClick={handleResetTransform}
              className="active-filter-reset text-xs underline"
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
          readOnly={!viewPolicy.canMutateContent}
          onColorChange={setTransformedText}
          onCopyFormat={(label, value) => void handleCopySpecificFormat(label, value)}
          onRunOCR={() => void handleRunOCR()}
        />

      </div>

      {/* Contextual filter suggestions live beside the controls they affect. */}
      {(() => {
        const currentText = transformedText !== null ? transformedText : (clip.text_content || '');
        const { detectedTypes, recommendedFilters } = detectSmartFilterRecommendations(currentText, filters);
        if (!viewPolicy.canApplyFilters || clip.content_type === 'image' || recommendedFilters.length === 0) return null;

        return (
          <div className="smart-actions-bar px-4 py-2 flex items-center justify-between text-xs space-x-2 overflow-x-auto">
            <div className="smart-actions-heading flex items-center space-x-1.5 shrink-0 font-semibold text-[11px]">
              <Sparkles className="w-3.5 h-3.5 animate-pulse" />
              <span>Smart Actions ({detectedTypes.join(', ')}):</span>
            </div>
            <div className="flex items-center space-x-1.5 overflow-x-auto scrollbar-none py-0.5">
              {recommendedFilters.map((f) => (
                <button
                  key={f.id}
                  onClick={() => handleApplyFilter(f)}
                  className="smart-action-button px-2 py-0.5 rounded-md border text-[11px] font-medium flex items-center space-x-1 whitespace-nowrap shadow-sm"
                  title={`Apply ${f.name}`}
                >
                  <span>{f.name}</span>
                </button>
              ))}
            </div>
          </div>
        );
      })()}

      {/* Sleek Filter Pipeline Selector Bar */}
      {viewPolicy.canApplyFilters && clip.content_type !== 'image' && filters.length > 0 && (
        <div className="preview-filter-bar px-4 py-2.5 border-t select-none">
          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center space-x-2 shrink-0">
              <Sliders className="preview-filter-accent w-4 h-4" />
              <span className="theme-text-main text-xs font-semibold">Apply Filter:</span>
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
                className={`theme-input w-full border text-xs rounded-xl px-3 py-1.5 focus:outline-none transition-colors cursor-pointer font-medium ${
                  activeFilterName
                    ? 'preview-filter-select-active'
                    : 'form-field-valid'
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
                className="preview-filter-reset flex items-center space-x-1 px-2.5 py-1 rounded-lg border text-xs font-semibold transition-colors shrink-0"
                title="Reset to raw clip text"
              >
                <span>Reset</span>
              </button>
            )}
          </div>
        </div>
      )}

      {showHistory && (
        <ClipRevisionHistory
          versions={versions}
          isLoading={isHistoryLoading}
          readOnly={!viewPolicy.canMutateContent}
          onClose={() => setShowHistory(false)}
          onRestore={(version) => void handleRestoreVersion(version)}
        />
      )}

      {/* Stats Footer */}
      <div className="clip-preview-footer px-4 py-2.5 border-t flex text-[11px]">
        <div className="clip-preview-footer-stats">
          <span className="clip-preview-footer-stat">
            <span>Chars:</span>
            <strong>{charCount}</strong>
          </span>
          <span className="clip-preview-footer-stat">
            <span>Words:</span>
            <strong>{wordCount}</strong>
          </span>
          <span className="clip-preview-footer-stat">
            <span>Lines:</span>
            <strong>{lineCount}</strong>
          </span>
          <span className="clip-preview-footer-stat">
            <span>Revisions:</span>
            <button
              type="button"
              onClick={() => setShowHistory((prev) => !prev)}
              className={`clip-revision-count ${showHistory ? 'is-active' : ''}`}
              title={revisionCount === null ? 'Loading revision count' : 'View revision history'}
              aria-label={revisionCount === null ? 'Loading clip revision count' : `View ${revisionCount} clip revisions`}
              aria-expanded={showHistory}
              aria-controls="clip-revision-history-panel"
            >
              {revisionCount ?? '…'}
            </button>
          </span>
        </div>
        <div className="clip-preview-footer-captured">
          <span>Captured:</span>
          <time dateTime={clip.created_at} title={formatClipDateTime(clip.created_at)}>
            {formatClipDateTime(clip.created_at)}
          </time>
        </div>
      </div>

      {viewingNote && (
        <ClipNoteViewer
          note={viewingNote}
          sourceApp={clip.source_app}
          onClose={() => setViewingNote(null)}
        />
      )}

    </div>
  );
};
