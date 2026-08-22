import { useCallback, useEffect, useRef, useState } from 'react';

import type { ClipItem, ClipNote } from '../types';
import { parseClipNotes, serializeClipNotes } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { useStableVerticalReorder } from './useStableVerticalReorder';

const NOTE_PLACEHOLDERS = [
  "Add a note before future-you forgets why you copied this...",
  "Jot down your secret brilliance...",
  "What's the tea on this snippet?...",
  "Note to self: Don't lose this thought...",
  "Drop some wisdom, context, or grocery items...",
];

interface UseClipPreviewNotesInput {
  clip: ClipItem | null;
  canEdit: boolean;
  onUpdateClipNote?: (clipId: number, noteContent: string | null) => void;
}

export function useClipPreviewNotes({ clip, canEdit, onUpdateClipNote }: UseClipPreviewNotesInput) {
  const [notes, setNotes] = useState<ClipNote[]>(() => parseClipNotes(clip?.note));
  const [isAdding, setIsAdding] = useState(false);
  const [newNoteText, setNewNoteText] = useState('');
  const [placeholder, setPlaceholder] = useState(NOTE_PLACEHOLDERS[0]);
  const [editingNoteId, setEditingNoteId] = useState<string | null>(null);
  const [editingNoteText, setEditingNoteText] = useState('');
  const [viewingNote, setViewingNote] = useState<ClipNote | null>(null);
  const notesRef = useRef(notes);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    notesRef.current = notes;
  }, [notes]);

  useEffect(() => {
    const parsed = parseClipNotes(clip?.note);
    setNotes(parsed);
    notesRef.current = parsed;
    setIsAdding(false);
    setNewNoteText('');
    setEditingNoteId(null);
    setEditingNoteText('');
    setViewingNote(null);
  }, [clip]);

  useEffect(() => {
    if (canEdit) return;
    setIsAdding(false);
    setNewNoteText('');
    setEditingNoteId(null);
    setEditingNoteText('');
  }, [canEdit]);

  const save = useCallback((updatedNotes: ClipNote[]) => {
    if (!clip || !canEdit) return;
    setNotes(updatedNotes);
    notesRef.current = updatedNotes;
    const serialized = serializeClipNotes(updatedNotes);
    onUpdateClipNote?.(clip.id, serialized);
    invoke('update_clip_note', { clipId: clip.id, note: serialized })
      .catch((error) => console.error('Failed to update clip note:', error));
  }, [canEdit, clip, onUpdateClipNote]);

  const reorder = useStableVerticalReorder({
    itemIds: notes.map((note) => note.id),
    containerRef,
    disabled: !canEdit || notes.length < 2 || editingNoteId !== null,
    onCommit: (orderedIds) => {
      const byId = new Map(notesRef.current.map((note) => [note.id, note]));
      save(orderedIds.map((id) => byId.get(id)).filter((note): note is ClipNote => Boolean(note)));
    },
  });

  const toggleAdding = () => {
    if (!isAdding) setPlaceholder(NOTE_PLACEHOLDERS[Math.floor(Math.random() * NOTE_PLACEHOLDERS.length)]);
    setIsAdding((current) => !current);
  };

  const cancelAdding = () => {
    setIsAdding(false);
    setNewNoteText('');
  };

  const create = () => {
    if (!canEdit || !newNoteText.trim()) return;
    save([...notes, {
      id: `note-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
      text: newNoteText.trim(),
      created_at: new Date().toISOString(),
    }]);
    cancelAdding();
  };

  const update = (id: string, text: string) => {
    if (!canEdit) return;
    setEditingNoteId(null);
    save(notes.map((note) => note.id === id ? { ...note, text: text.trim() } : note)
      .filter((note) => note.text.length > 0));
  };

  const remove = (id: string) => {
    if (canEdit) save(notes.filter((note) => note.id !== id));
  };

  return {
    notes,
    isAdding,
    newNoteText,
    setNewNoteText,
    placeholder,
    editingNoteId,
    setEditingNoteId,
    editingNoteText,
    setEditingNoteText,
    viewingNote,
    setViewingNote,
    containerRef,
    reorder,
    toggleAdding,
    cancelAdding,
    create,
    update,
    remove,
  };
}

export type ClipPreviewNotesController = ReturnType<typeof useClipPreviewNotes>;
