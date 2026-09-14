import { useCallback } from 'react';
import type { ClipItem } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { soundManager } from '../utils/sound';
import { loadFullClip } from '../utils/clipDetail';

export function useClipQueueActions({
  queuedIndexMap,
  fetchSequentialStatus,
}: {
  queuedIndexMap: Map<string, number>;
  fetchSequentialStatus: () => Promise<void>;
}) {
  const addToSequentialStack = useCallback(async (clip: ClipItem) => {
    try {
      const fullClip = await loadFullClip(clip);
      const item = fullClip.content_type === 'file' ? null : fullClip.text_content;
      if (!item) {
        console.warn('Only clips containing text can be added to the Copy Queue');
        return;
      }
      await invoke('push_sequential_item', { item });
      soundManager.playStackSound();
      void fetchSequentialStatus();
    } catch (error) {
      console.error('Failed to add clip to queue:', error);
    }
  }, [fetchSequentialStatus]);

  const toggleSequentialStack = useCallback(async (clip: ClipItem) => {
    try {
      const fullClip = await loadFullClip(clip);
      const item = fullClip.content_type === 'file' ? null : fullClip.text_content;
      if (!item) return;
      const queueIndex = queuedIndexMap.get(item);
      if (queueIndex === undefined) return addToSequentialStack(fullClip);
      await invoke('remove_sequential_item_by_index', { index: queueIndex - 1 });
      await fetchSequentialStatus();
    } catch (error) {
      console.error('Failed to toggle clip in queue:', error);
    }
  }, [addToSequentialStack, fetchSequentialStatus, queuedIndexMap]);

  return { addToSequentialStack, toggleSequentialStack };
}
