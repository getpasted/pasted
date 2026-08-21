import { handled, unhandled, type BrowserMockResult } from './result';

interface BrowserClip {
  id: number;
  content_type: string;
  source: string;
  is_trashed: number;
  is_pinned: number;
  is_protected: number;
  is_concealed?: number | boolean;
  note?: string | null;
  bin_ids: number[];
  content_types?: string[];
  file_formats?: string[];
}

export function handleClipBrowserMock<T extends BrowserClip>(
  command: string,
  args: Record<string, unknown> | undefined,
  clips: readonly T[],
  withProtection: (clip: T) => object,
): BrowserMockResult {
  if (command === 'get_clips') {
    const offset = Math.max(0, Number(args?.offset ?? 0));
    const limit = Math.max(1, Number(args?.limit ?? 10_000));
    const binId = Number(args?.binId);
    return handled(clips
      .filter((clip) => clip.is_trashed === 0
        && (!Number.isInteger(binId) || binId <= 0 || clip.bin_ids.includes(binId)))
      .slice(offset, offset + limit)
      .map((clip) => ({ ...withProtection(clip), content_types: [...(clip.content_types ?? [])], bin_ids: [...clip.bin_ids] })));
  }
  if (command === 'get_trashed_clips') {
    const offset = Math.max(0, Number(args?.offset ?? 0));
    const limit = Math.max(1, Number(args?.limit ?? 10_000));
    return handled(clips.filter((clip) => clip.is_trashed !== 0).slice(offset, offset + limit));
  }
  if (command === 'get_clip_collection_summary') {
    const active = clips.filter((clip) => clip.is_trashed === 0);
    const countBy = (key: 'content_type' | 'source') => [...active.reduce((counts, clip) => counts.set(String(clip[key]), (counts.get(String(clip[key])) ?? 0) + 1), new Map<string, number>())];
    const contentTypeCounts = [...active.reduce((counts, clip) => {
      [...new Set(clip.content_types ?? [])].forEach((contentType) => counts.set(contentType, (counts.get(contentType) ?? 0) + 1));
      return counts;
    }, new Map<string, number>())];
    const fileFormatCounts = [...active.reduce((counts, clip) => {
      [...new Set(clip.file_formats ?? [])].forEach((fileFormat) => counts.set(fileFormat, (counts.get(fileFormat) ?? 0) + 1));
      return counts;
    }, new Map<string, number>())];
    return handled({
      activeCount: active.length,
      trashCount: clips.length - active.length,
      pinnedCount: active.filter((clip) => clip.is_pinned).length,
      protectedCount: active.filter((clip) => clip.is_protected).length,
      concealedCount: active.filter((clip) => clip.is_concealed).length,
      notedCount: active.filter((clip) => Boolean(clip.note?.trim())).length,
      clipTypeCounts: countBy('content_type').map(([clip_type, count]) => ({ clip_type, count })),
      fileFormatCounts: fileFormatCounts.map(([file_format, count]) => ({ file_format, count })),
      typeCounts: contentTypeCounts.map(([content_type, count]) => ({ content_type, count })),
      sourceCounts: countBy('source').map(([name, count]) => ({ name, count })),
    });
  }
  return unhandled;
}
