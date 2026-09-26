import { clipsApi } from '../api/clips';
import type { ClipItem } from '../types';

export function loadFullClip(clip: ClipItem): Promise<ClipItem> {
  return clip.is_summary ? clipsApi.detail(clip.id) : Promise.resolve(clip);
}
