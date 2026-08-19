import { handled, unhandled, type BrowserMockResult } from './result';

interface BrowserBin { id: number }
interface BrowserClip { is_trashed: number; bin_ids: number[] }

export function handleBinBrowserMock(command: string, bins: readonly BrowserBin[], clips: readonly BrowserClip[]): BrowserMockResult {
  if (command !== 'get_bins') return unhandled;
  return handled(bins.map((bin) => ({
    ...bin,
    clip_count: clips.filter((clip) => clip.is_trashed === 0 && clip.bin_ids.includes(bin.id)).length,
  })));
}
