import { useCallback, useEffect, useState } from 'react';
import type { ClipItem } from '../types';

export function useClipMetadataPrompts(notesEnabled: boolean, namingEnabled: boolean) {
  const [notePromptClip, setNotePromptClip] = useState<ClipItem | null>(null);
  const [notePromptText, setNotePromptText] = useState('');
  const [namePromptClip, setNamePromptClip] = useState<ClipItem | null>(null);
  const [namePromptText, setNamePromptText] = useState('');

  useEffect(() => {
    if (!notesEnabled) setNotePromptClip(null);
    if (!namingEnabled) setNamePromptClip(null);
  }, [namingEnabled, notesEnabled]);

  const promptAddNote = useCallback((clip: ClipItem) => {
    if (!notesEnabled) return;
    setNotePromptClip(clip);
    setNotePromptText(clip.note || '');
  }, [notesEnabled]);
  const promptNameClip = useCallback((clip: ClipItem) => {
    if (!namingEnabled) return;
    setNamePromptClip(clip);
    setNamePromptText(clip.name || '');
  }, [namingEnabled]);

  return {
    notePromptClip, setNotePromptClip, notePromptText, setNotePromptText,
    namePromptClip, setNamePromptClip, namePromptText, setNamePromptText,
    promptAddNote, promptNameClip,
  };
}
