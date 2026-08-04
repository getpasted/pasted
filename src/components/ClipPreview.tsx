import React, { useState, useEffect, useRef } from 'react';
import { formatClipDateTime } from '../utils/date';
import { ClipItem, Bin, Pipeline, ClipNote, parseClipNotes, serializeClipNotes, ClipVersion } from '../types';
import type { ClipTransformationProvenance, ExecutePlanOutcome, TransformationRecipe } from '../types';
import { parseColor, ColorFormats } from '../utils/color';
import { soundManager } from '../utils/sound';
import { detectSmartPipelineRecommendations } from '../utils/smartPipelineDetector';
import { startWindowDrag } from '../utils/windowDrag';
import { ClipRevisionHistory } from './ClipRevisionHistory';
import { ClipPreviewContent } from './ClipPreviewContent';
import { ClipNoteViewer } from './ClipNoteViewer';
import { NoteRowItem } from './ClipNoteRow';
import {
  Copy,
  ClipboardPaste,
  Check,
  Trash2,
  Sliders,
  Folder,
  FileText,
  StickyNote,
  Sparkles,
  LoaderCircle,
  Workflow,
  Lightbulb,
} from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { useStableVerticalReorder } from '../hooks/useStableVerticalReorder';
import type { ClipViewPolicy } from '../utils/clipViewPolicy';

interface ClipPreviewProps {
  clip: ClipItem | null;
  viewPolicy: ClipViewPolicy;
  bins: Bin[];
  pipelines: Pipeline[];
  onUpdateClip: (updatedClip?: ClipItem) => void;
  onAssignBin: (clipId: number, binId: number | null) => void | Promise<void>;
  onDeleteClip: (id: number) => void;
  onUpdateClipNote?: (clipId: number, noteContent: string | null) => void;
  isTransforming?: boolean;
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
  pipelines,
  onUpdateClip,
  onAssignBin,
  onDeleteClip,
  onUpdateClipNote,
  isTransforming = false,
}) => {
  const [copied, setCopied] = useState(false);
  const [copiedFormat, setCopiedFormat] = useState<string | null>(null);
  const [transformedText, setTransformedText] = useState<string | null>(null);
  const [activePipelineRef, setActivePipelineRef] = useState<string | null>(null);
  const [activePipelineName, setActivePipelineName] = useState<string | null>(null);
  const [recipes, setRecipes] = useState<TransformationRecipe[]>([]);
  const [activeRecipeRef, setActiveRecipeRef] = useState<string | null>(null);
  const [activeRecipeName, setActiveRecipeName] = useState<string | null>(null);
  const [recipePreviewOutcome, setRecipePreviewOutcome] = useState<ExecutePlanOutcome | null>(null);
  const [provenance, setProvenance] = useState<ClipTransformationProvenance | null>(null);
  const [isPipelineRunning, setIsPipelineRunning] = useState(false);
  const [pipelineAction, setPipelineAction] = useState<'copied' | 'pasted' | null>(null);
  const [pipelineError, setPipelineError] = useState<string | null>(null);
  const [notes, setNotes] = useState<ClipNote[]>(() => parseClipNotes(clip?.note));
  const notesRef = useRef(notes);
  const noteBoxRef = useRef<HTMLDivElement>(null);
  const copiedTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const copiedFormatTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pipelineRequestIdRef = useRef(0);

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
  const [previewedVersion, setPreviewedVersion] = useState<ClipVersion | null>(null);
  const [restoringVersionId, setRestoringVersionId] = useState<number | null>(null);
  const [revisionCount, setRevisionCount] = useState<number | null>(null);
  const [isHistoryLoading, setIsHistoryLoading] = useState(false);

  useEffect(() => {
    invoke<TransformationRecipe[]>('get_transformation_recipes')
      .then((items) => setRecipes(Array.isArray(items) ? items : []))
      .catch((error) => console.error('Failed to load Recipes:', error));
  }, []);

  useEffect(() => {
    let cancelled = false;
    if (!clip) {
      setProvenance(null);
      return () => { cancelled = true; };
    }
    invoke<ClipTransformationProvenance | null>('get_clip_transformation_provenance', { clipId: clip.id })
      .then((value) => { if (!cancelled) setProvenance(value); })
      .catch((error) => console.error('Failed to load transformation provenance:', error));
    return () => { cancelled = true; };
  }, [clip?.id, clip?.is_transformed, clip?.text_content]);

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
  }, [clip?.id, clip?.is_transformed, clip?.text_content]);

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
  }, [clip?.id, clip?.is_transformed, clip?.text_content, showHistory]);

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
    pipelineRequestIdRef.current += 1;
    setTransformedText(null);
    setActivePipelineRef(null);
    setActivePipelineName(null);
    setActiveRecipeRef(null);
    setActiveRecipeName(null);
    setRecipePreviewOutcome(null);
    setIsPipelineRunning(false);
    setPipelineAction(null);
    setPipelineError(null);
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
    setPreviewedVersion(null);
    setRestoringVersionId(null);
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
    if (viewPolicy.canRunPipelines) return;
    setTransformedText(null);
    setActivePipelineRef(null);
    setActivePipelineName(null);
    setPipelineAction(null);
    setPipelineError(null);
  }, [viewPolicy.canRunPipelines]);

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
          Select an item from history or right-click to copy, transform, add notes, or organize.
        </p>
      </div>
    );
  }

  const displayText = previewedVersion?.text_content ?? transformedText ?? clip.text_content ?? '';
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

  const handlePreviewPipeline = async (pipeline: Pipeline) => {
    if (!clip.text_content) return;
    setPreviewedVersion(null);
    const requestId = ++pipelineRequestIdRef.current;
    setActivePipelineRef(pipeline.stableRef);
    setActivePipelineName(pipeline.name);
    setActiveRecipeRef(null);
    setActiveRecipeName(null);
    setRecipePreviewOutcome(null);
    setTransformedText(null);
    setIsPipelineRunning(true);
    setPipelineAction(null);
    setPipelineError(null);
    try {
      const res = await invoke<{ output: string }>('execute_transformation', {
        request: {
          input: clip.text_content,
          target: { kind: 'pipeline', pipelineRef: pipeline.stableRef },
          sourceClipId: clip.id,
          trigger: 'manual',
        },
      });
      if (requestId !== pipelineRequestIdRef.current) return;
      setTransformedText(res.output);
    } catch (e) {
      if (requestId !== pipelineRequestIdRef.current) return;
      console.error(e);
      setPipelineError(e instanceof Error ? e.message : 'Pipeline failed to run.');
    } finally {
      if (requestId === pipelineRequestIdRef.current) setIsPipelineRunning(false);
    }
  };

  const handlePreviewRecipe = async (recipe: TransformationRecipe) => {
    if (!clip.text_content) return;
    setPreviewedVersion(null);
    const requestId = ++pipelineRequestIdRef.current;
    setActiveRecipeRef(recipe.stableRef);
    setActiveRecipeName(recipe.name);
    setActivePipelineRef(null);
    setActivePipelineName(null);
    setTransformedText(null);
    setRecipePreviewOutcome(null);
    setIsPipelineRunning(true);
    setPipelineAction(null);
    setPipelineError(null);
    setPreviewedVersion(null);
    try {
      const result = await invoke<ExecutePlanOutcome>('execute_transformation_recipe', {
        recipeRef: recipe.stableRef,
        input: clip.text_content,
      });
      if (requestId !== pipelineRequestIdRef.current) return;
      setRecipePreviewOutcome(result);
      setTransformedText(result.output);
    } catch (error) {
      if (requestId !== pipelineRequestIdRef.current) return;
      setPipelineError(error instanceof Error ? error.message : 'Recipe failed to run.');
    } finally {
      if (requestId === pipelineRequestIdRef.current) setIsPipelineRunning(false);
    }
  };

  const handleApplyRecipe = async () => {
    if (!activeRecipeRef || !recipePreviewOutcome || transformedText === null || !clip.text_content) return;
    setIsPipelineRunning(true);
    setPipelineError(null);
    try {
      const saved = await invoke<ClipTransformationProvenance>('apply_recipe_preview_to_clip', {
        clipId: clip.id,
        recipeRef: activeRecipeRef,
        expectedInput: clip.text_content,
        output: transformedText,
        connectionId: recipePreviewOutcome.connectionId,
        durationMs: recipePreviewOutcome.durationMs,
      });
      setProvenance(saved);
      setRevisionCount((count) => (count ?? 0) + 1);
      soundManager.playCopySound(true);
      handleResetTransform();
      onUpdateClip();
    } catch (error) {
      setPipelineError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsPipelineRunning(false);
    }
  };

  const handleResetTransform = () => {
    pipelineRequestIdRef.current += 1;
    setTransformedText(null);
    setActivePipelineRef(null);
    setActivePipelineName(null);
    setActiveRecipeRef(null);
    setActiveRecipeName(null);
    setRecipePreviewOutcome(null);
    setPipelineAction(null);
    setPipelineError(null);
    setPreviewedVersion(null);
  };

  const handlePipelineOutput = async (destination: 'copy' | 'paste') => {
    if (transformedText === null) return;
    try {
      if (destination === 'copy') {
        await invoke('copy_clip_to_system', { text: transformedText, imageBase64: null });
        setPipelineAction('copied');
        soundManager.playCopySound(true);
      } else {
        await invoke('paste_text_to_frontmost', { text: transformedText });
        setPipelineAction('pasted');
        soundManager.playPasteSound(true);
      }
      setPipelineError(null);
    } catch (error) {
      console.error(`Failed to ${destination} Pipeline output:`, error);
      setPipelineError(`Could not ${destination} the Pipeline result.`);
    }
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
    if (!viewPolicy.canMutateContent || restoringVersionId !== null) return;
    setRestoringVersionId(version.id);
    try {
      const restoredClip = await invoke<ClipItem>('restore_clip_version', {
        clipId: clip.id,
        versionId: version.id,
      });
      invoke<number>('get_clip_version_count', { clipId: clip.id })
        .then(setRevisionCount)
        .catch((error) => console.error('Failed to refresh clip revision count:', error));
      setTransformedText(null);
      setActivePipelineRef(null);
      setActivePipelineName(null);
      setPipelineAction(null);
      setPipelineError(null);
      setShowHistory(false);
      setPreviewedVersion(null);
      soundManager.playCopySound(true);
      onUpdateClip(restoredClip);
    } catch (error) {
      console.error('Failed to restore clip version:', error);
    } finally {
      setRestoringVersionId(null);
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
          {isTransforming && (
            <LoaderCircle
              className="clip-transform-working h-4 w-4 shrink-0 animate-spin"
              aria-label="Applying Recipe"
            />
          )}
          {!isTransforming && provenance && (
            <Workflow
              className="transform-accent pipelines h-4 w-4 shrink-0"
              aria-label={`Transformed with ${provenance.recipeName}`}
            />
          )}
          {!isTransforming && provenance?.connectionId && (
            <Sparkles
              className="transform-accent pipelines h-3.5 w-3.5 shrink-0"
              aria-label="Recipe used connected intelligence"
            />
          )}
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
        {(activePipelineName || activeRecipeName || previewedVersion) && (
          <div className="active-filter-banner flex items-center justify-between px-3 py-2 border rounded-lg">
            <div className="flex items-center space-x-2">
              <Sliders className="w-4 h-4" />
              <span>
                {previewedVersion ? 'Previewing revision' : (isPipelineRunning ? 'Running' : 'Previewing')}:
                {' '}<strong>{previewedVersion
                  ? formatClipDateTime(previewedVersion.created_at)
                  : (activeRecipeName || activePipelineName)}</strong>
              </span>
            </div>
            {(activePipelineName || previewedVersion) && (
              <button
                onClick={handleResetTransform}
                className="active-filter-reset text-xs underline"
              >
                Reset
              </button>
            )}
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
        const { detectedTypes, recommendedPipelines } = detectSmartPipelineRecommendations(currentText, pipelines);
        if (!viewPolicy.canRunPipelines || clip.content_type === 'image' || recommendedPipelines.length === 0) return null;

        return (
          <div className="smart-actions-bar px-4 py-2 flex items-center justify-between text-xs space-x-2 overflow-x-auto">
            <div className="smart-actions-heading flex items-center space-x-1.5 shrink-0 font-semibold text-[11px]">
              <Lightbulb className="w-3.5 h-3.5" />
              <span>Smart Actions ({detectedTypes.join(', ')}):</span>
            </div>
            <div className="flex items-center space-x-1.5 overflow-x-auto scrollbar-none py-0.5">
              {recommendedPipelines.map((f) => (
                <button
                  key={f.id}
                  onClick={() => handlePreviewPipeline(f)}
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

      {viewPolicy.canRunPipelines && clip.content_type !== 'image' && recipes.length > 0 && (
        <div className="preview-filter-bar px-4 py-2.5 border-t select-none">
          <div className="flex flex-wrap items-center gap-3">
            <div className="flex items-center space-x-2 shrink-0">
              <Workflow className="preview-filter-accent w-4 h-4" />
              <span className="theme-text-main text-xs font-semibold">Apply Recipe:</span>
            </div>
            <div className="relative min-w-[14rem] flex-[1_1_18rem]">
              <select
                value={activeRecipeRef || ''}
                onChange={(event) => {
                  const selectedRef = event.target.value;
                  if (!selectedRef) {
                    handleResetTransform();
                  } else {
                    const recipe = recipes.find((item) => item.stableRef === selectedRef);
                    if (recipe) void handlePreviewRecipe(recipe);
                  }
                }}
                className={`theme-input w-full border text-xs rounded-xl px-3 py-1.5 focus:outline-none transition-colors cursor-pointer font-medium ${activeRecipeRef ? 'preview-filter-select-active' : 'form-field-valid'}`}
              >
                <option value="">Original clip (No Recipe)</option>
                {recipes.map((recipe) => (
                  <option key={recipe.stableRef} value={recipe.stableRef}>{recipe.name}</option>
                ))}
              </select>
            </div>
            {activeRecipeRef && (
              <div className="ml-auto flex items-center gap-1.5 shrink-0">
                <button
                  type="button"
                  onClick={() => void handleApplyRecipe()}
                  disabled={isPipelineRunning || transformedText === null || !recipePreviewOutcome}
                  className="transform-workspace-action pipelines flex items-center gap-1.5 rounded-lg px-2.5 py-1 text-xs font-bold transition-colors disabled:cursor-not-allowed disabled:opacity-40"
                  title="Replace the clip with this preview and preserve the original in Revisions"
                >
                  {isPipelineRunning ? <Sliders className="h-3.5 w-3.5 animate-spin" /> : <Check className="h-3.5 w-3.5" />}
                  <span>{isPipelineRunning ? 'Running…' : 'Apply'}</span>
                </button>
                <button
                  type="button"
                  onClick={handleResetTransform}
                  className="preview-filter-reset flex items-center space-x-1 px-2.5 py-1 rounded-lg border text-xs font-semibold transition-colors"
                >
                  <span>Cancel</span>
                </button>
              </div>
            )}
          </div>
          {activeRecipeRef && !isPipelineRunning && transformedText !== null && (
            <p className="theme-text-muted mt-2 text-[10px]">Preview only—Apply replaces the clip and keeps the original in Revision History.</p>
          )}
          {activeRecipeRef && pipelineError && <p role="status" className="theme-status-error mt-2 rounded-lg border px-2.5 py-1.5 text-[11px]">{pipelineError}</p>}
        </div>
      )}

      {/* Sleek Filter Pipeline Selector Bar */}
      {viewPolicy.canRunPipelines && clip.content_type !== 'image' && pipelines.length > 0 && (
        <div className="preview-filter-bar px-4 py-2.5 border-t select-none">
          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center space-x-2 shrink-0">
              <Sliders className="preview-filter-accent w-4 h-4" />
              <span className="theme-text-main text-xs font-semibold">Apply Pipeline:</span>
            </div>

            {/* Dropdown Selector */}
            <div className="flex-1 max-w-xs relative">
              <select
                value={activePipelineRef || ''}
                onChange={(e) => {
                  const selectedRef = e.target.value;
                  if (!selectedRef) {
                    handleResetTransform();
                  } else {
                    const found = pipelines.find((pipeline) => pipeline.stableRef === selectedRef);
                    if (found) void handlePreviewPipeline(found);
                  }
                }}
                className={`theme-input w-full border text-xs rounded-xl px-3 py-1.5 focus:outline-none transition-colors cursor-pointer font-medium ${
                  activePipelineRef
                    ? 'preview-filter-select-active'
                    : 'form-field-valid'
                }`}
              >
                <option value="">⚡ Raw Original Text (No Pipeline)</option>
                {pipelines.map((f) => (
                  <option key={f.id} value={f.stableRef}>
                    {f.name}
                  </option>
                ))}
              </select>
            </div>

            {/* Reset Action */}
            {activePipelineRef && (
              <div className="flex items-center gap-1.5 shrink-0">
                <button
                  type="button"
                  onClick={() => void handlePipelineOutput('copy')}
                  disabled={isPipelineRunning || transformedText === null}
                  className="theme-secondary-button flex items-center gap-1.5 rounded-lg border px-2.5 py-1 text-xs font-semibold transition-colors disabled:cursor-not-allowed disabled:opacity-40"
                  title="Copy the Pipeline result"
                >
                  {pipelineAction === 'copied' ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
                  <span>{pipelineAction === 'copied' ? 'Copied' : 'Copy'}</span>
                </button>
                <button
                  type="button"
                  onClick={() => void handlePipelineOutput('paste')}
                  disabled={isPipelineRunning || transformedText === null}
                  className="transform-workspace-action pipelines flex items-center gap-1.5 rounded-lg px-2.5 py-1 text-xs font-bold transition-colors disabled:cursor-not-allowed disabled:opacity-40"
                  title="Paste the Pipeline result into the frontmost app"
                >
                  <ClipboardPaste className="h-3.5 w-3.5" />
                  <span>{pipelineAction === 'pasted' ? 'Pasted' : 'Paste'}</span>
                </button>
                <button
                  onClick={handleResetTransform}
                  className="preview-filter-reset flex items-center space-x-1 px-2.5 py-1 rounded-lg border text-xs font-semibold transition-colors"
                  title="Reset to raw clip text"
                >
                  <span>Reset</span>
                </button>
              </div>
            )}
          </div>
          {pipelineError && <p role="status" className="theme-status-error mt-2 rounded-lg border px-2.5 py-1.5 text-[11px]">{pipelineError}</p>}
        </div>
      )}

      {showHistory && (
        <ClipRevisionHistory
          versions={versions}
          isLoading={isHistoryLoading}
          readOnly={!viewPolicy.canMutateContent}
          onClose={() => setShowHistory(false)}
          previewedVersionId={previewedVersion?.id ?? null}
          restoringVersionId={restoringVersionId}
          onPreview={(version) => setPreviewedVersion((current) => (
            current?.id === version.id ? null : version
          ))}
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
