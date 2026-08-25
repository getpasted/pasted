import { useCallback, useEffect, useState } from 'react';

import type { EffectiveVisualLabels } from '../components/clipPreviewModel';
import type { ClipItem } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';

export function useClipVisualLabels({
  clip,
  canMutate,
  onUpdate,
  onError,
}: {
  clip: ClipItem | null;
  canMutate: boolean;
  onUpdate: () => void;
  onError: (message: string) => void;
}) {
  const [visualLabels, setVisualLabels] = useState<EffectiveVisualLabels | null>(null);

  const refresh = useCallback(async (clipId: number) => {
    const labels = await invoke<EffectiveVisualLabels>('get_clip_visual_labels', { clipId });
    setVisualLabels(labels);
  }, []);

  useEffect(() => {
    let cancelled = false;
    setVisualLabels(null);
    if (!clip || (clip.content_type !== 'image' && clip.content_type !== 'file')) return undefined;
    invoke<EffectiveVisualLabels>('get_clip_visual_labels', { clipId: clip.id })
      .then((labels) => { if (!cancelled) setVisualLabels(labels); })
      .catch((error) => { if (!cancelled) console.error('Failed to load Visual Labels:', error); });
    return () => { cancelled = true; };
  }, [clip]);

  const mutate = async (operation: (clipId: number) => Promise<EffectiveVisualLabels>) => {
    if (!clip || !canMutate) return;
    try {
      const labels = await operation(clip.id);
      setVisualLabels(labels);
      onUpdate();
    } catch (error) {
      onError(String(error));
    }
  };

  return {
    refresh,
    contentProps: {
      visualLabels,
      onAddVisualLabel: (label: string) => mutate((clipId) =>
        invoke<EffectiveVisualLabels>('add_clip_visual_label', { clipId, label })),
      onRemoveVisualLabel: (label: string) => mutate((clipId) =>
        invoke<EffectiveVisualLabels>('remove_clip_visual_label', { clipId, label })),
      onResetVisualLabels: () => mutate((clipId) =>
        invoke<EffectiveVisualLabels>('reset_clip_visual_labels', { clipId })),
    },
  };
}
