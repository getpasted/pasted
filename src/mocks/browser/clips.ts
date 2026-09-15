import { handled, unhandled, type BrowserMockResult } from './result';
import { browserClipMatchesBin } from './smartBins';

interface BrowserBin { id: number; smart_rule?: string | null }

interface BrowserClip {
  id: number;
  content_type: string;
  source: string;
  is_trashed: number;
  is_pinned: number;
  is_protected: number | boolean;
  is_concealed?: number | boolean;
  name?: string | null;
  note?: string | null;
  bin_ids: number[];
  content_types?: string[];
  file_formats?: string[];
  text_content?: string | null;
  html_content?: string | null;
  image_base64?: string | null;
  image_path?: string | null;
}

export function browserClipListItem<T extends BrowserClip>(clip: T) {
  const { text_content: text, html_content: _html, image_base64: _image, image_path: _path, ...metadata } = clip;
  let fileNames: string[] = [];
  let fileCount = 0;
  if (clip.content_type === 'file' && text) {
    try {
      const paths = JSON.parse(text);
      if (Array.isArray(paths)) fileNames = paths.filter((path): path is string => typeof path === 'string');
    } catch {
      fileNames = text.split(/\r?\n/).filter(Boolean);
    }
    fileCount = fileNames.length;
    fileNames = fileNames.map((path) => path.split(/[\\/]/).filter(Boolean).pop() ?? path).slice(0, 20);
  }
  const characters = text ? Array.from(text) : [];
  return {
    ...metadata,
    preview_text: clip.content_type === 'file' ? null : characters.slice(0, 1_024).join('') || null,
    preview_truncated: clip.content_type !== 'file' && characters.length > 1_024,
    file_names: fileNames,
    file_count: fileCount,
  };
}

export function handleClipBrowserMock<T extends BrowserClip>(
  command: string,
  args: Record<string, unknown> | undefined,
  clips: readonly T[],
  bins: readonly BrowserBin[],
  withPolicies: (clip: T) => object & { is_concealed: boolean; is_protected: boolean },
): BrowserMockResult {
  if (command === 'update_clip_name') {
    const clip = clips.find((item) => item.id === Number(args?.clipId));
    if (clip && clip.is_trashed === 0) clip.name = typeof args?.name === 'string' ? args.name.trim() || null : null;
    return handled(clip ? { ...withPolicies(clip) } : null);
  }
  if (command === 'get_clip_detail') {
    const clip = clips.find((item) => item.id === Number(args?.id));
    return handled(clip ? { ...withPolicies(clip) } : null);
  }
  if (command === 'get_clip_collection_page') {
    const request = (args?.request ?? {}) as Record<string, unknown>;
    const collection = String(request.collection ?? 'history');
    const value = String(request.value ?? '').toLowerCase();
    const binId = Number(request.binId);
    const offset = Math.max(0, Number(request.offset ?? 0));
    const limit = Math.min(500, Math.max(1, Number(request.limit ?? 100)));
    const items = clips.filter((clip) => {
      const active = clip.is_trashed === 0;
      const policies = withPolicies(clip);
      if (collection === 'trash') return !active;
      if (!active) return false;
      if (collection === 'bin') return browserClipMatchesBin(clip, bins.find((bin) => bin.id === binId));
      if (collection === 'pinned') return Boolean(clip.is_pinned);
      if (collection === 'protected') return policies.is_protected;
      if (collection === 'concealed') return policies.is_concealed;
      if (collection === 'named') return Boolean(clip.name?.trim());
      if (collection === 'noted') return Boolean(clip.note?.trim());
      if (collection === 'clipType') return clip.content_type.toLowerCase() === value;
      if (collection === 'contentType') return (clip.content_types ?? []).some((type) => type.toLowerCase() === value);
      if (collection === 'fileFormat') return (clip.file_formats ?? []).some((format) => format.toLowerCase() === value);
      if (collection === 'source') return clip.source.toLowerCase() === value;
      return collection === 'history';
    });
    return handled({
      schemaVersion: 1,
      items: items.slice(offset, offset + limit).map((clip) => browserClipListItem({ ...clip, ...withPolicies(clip) })),
      totalCount: items.length,
      limit,
      offset,
    });
  }
  if (command === 'get_clips') {
    const offset = Math.max(0, Number(args?.offset ?? 0));
    const limit = Math.max(1, Number(args?.limit ?? 10_000));
    const binId = Number(args?.binId);
    return handled(clips
      .filter((clip) => clip.is_trashed === 0
        && (!Number.isInteger(binId) || binId <= 0 || clip.bin_ids.includes(binId)))
      .slice(offset, offset + limit)
      .map((clip) => ({ ...withPolicies(clip), content_types: [...(clip.content_types ?? [])], bin_ids: [...clip.bin_ids] })));
  }
  if (command === 'get_trashed_clips') {
    const offset = Math.max(0, Number(args?.offset ?? 0));
    const limit = Math.max(1, Number(args?.limit ?? 10_000));
    return handled(clips
      .filter((clip) => clip.is_trashed !== 0)
      .slice(offset, offset + limit)
      .map((clip) => ({ ...withPolicies(clip), content_types: [...(clip.content_types ?? [])], bin_ids: [...clip.bin_ids] })));
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
      concealedCount: active.filter((clip) => withPolicies(clip).is_concealed).length,
      namedCount: active.filter((clip) => Boolean(clip.name?.trim())).length,
      notedCount: active.filter((clip) => Boolean(clip.note?.trim())).length,
      clipTypeCounts: countBy('content_type').map(([clip_type, count]) => ({ clip_type, count })),
      fileFormatCounts: fileFormatCounts.map(([file_format, count]) => ({ file_format, count })),
      typeCounts: contentTypeCounts.map(([content_type, count]) => ({ content_type, count })),
      sourceCounts: countBy('source').map(([name, count]) => ({ name, count })),
    });
  }
  return unhandled;
}
