import { useState } from 'react';

import type { ClipItem, ClipVersion } from '../types';
import { soundManager } from '../utils/sound';
import { safeInvoke as invoke } from '../utils/tauri';

export function useClipVersionMutations({
  canMutate,
  clip,
  onDeleted,
  onRestored,
  refreshCount,
}: {
  canMutate: boolean;
  clip: ClipItem | null;
  onDeleted: (version: ClipVersion) => void;
  onRestored: (clip: ClipItem) => void;
  refreshCount: () => Promise<void>;
}) {
  const [deletingVersionId, setDeletingVersionId] = useState<number | null>(null);
  const [restoringVersionId, setRestoringVersionId] = useState<number | null>(null);
  const mutationPending = deletingVersionId !== null || restoringVersionId !== null;

  const restore = async (version: ClipVersion) => {
    if (!clip || !canMutate || mutationPending) return false;
    setRestoringVersionId(version.id);
    try {
      const restoredClip = await invoke<ClipItem>('restore_clip_version', {
        clipId: clip.id,
        versionId: version.id,
      });
      void refreshCount();
      soundManager.playCopySound();
      onRestored(restoredClip);
      return true;
    } catch (error) {
      console.error('Failed to restore clip version:', error);
      return false;
    } finally {
      setRestoringVersionId(null);
    }
  };

  const deleteVersion = async (version: ClipVersion) => {
    if (!clip || !canMutate || mutationPending || version.is_current || version.is_original) return false;
    setDeletingVersionId(version.id);
    try {
      await invoke('delete_clip_version', { clipId: clip.id, versionId: version.id });
      onDeleted(version);
      return true;
    } catch (error) {
      console.error('Failed to delete clip version:', error);
      return false;
    } finally {
      setDeletingVersionId(null);
    }
  };

  return { deleteVersion, deletingVersionId, restore, restoringVersionId };
}
