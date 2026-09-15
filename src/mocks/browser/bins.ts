import { handled, unhandled, type BrowserMockResult } from './result';
import { browserClipMatchesBin } from './smartBins';

interface BrowserBin { id: number; smart_rule?: string | null }
interface BrowserClip { is_trashed: number; source: string; bin_ids: number[] }

export function handleBinBrowserMock(command: string, bins: readonly BrowserBin[], clips: readonly BrowserClip[]): BrowserMockResult {
  if (command !== 'get_bins') return unhandled;
  return handled(bins.map((bin) => ({
    ...bin,
    clip_count: clips.filter((clip) => clip.is_trashed === 0 && browserClipMatchesBin(clip, bin)).length,
  })));
}
