import type { ClipListItem } from '../api/clipListTypes';
import type { ClipItem } from '../types';

export function uniqueClipItemsById(items: ClipItem[]): ClipItem[] {
  const seen = new Set<number>();
  return items.filter((item) => {
    if (seen.has(item.id)) return false;
    seen.add(item.id);
    return true;
  });
}

export function clipListItemsAsClips(items: ClipListItem[]): ClipItem[] {
  return uniqueClipItemsById(items.map((item) => ({
    ...item,
    text_content: item.content_type === 'file' ? JSON.stringify(item.file_names) : item.preview_text,
    html_content: null,
    image_base64: null,
    image_path: null,
    is_summary: true,
  })));
}

export function summarizeClipForList(clip: ClipItem): ClipItem {
  const fileNames = clip.content_type === 'file' && clip.text_content
    ? (() => {
        try {
          const paths = JSON.parse(clip.text_content);
          if (Array.isArray(paths)) return paths;
        } catch {
          // Legacy file clips may use newline-delimited paths.
        }
        return clip.text_content.split(/\r?\n/);
      })()
      .filter((path): path is string => typeof path === 'string')
      .map((path) => path.split(/[\\/]/).filter(Boolean).pop() ?? path)
      .map((name) => Array.from(name).slice(0, 255).join(''))
      .slice(0, 20)
    : [];
  const textCharacters = clip.text_content ? Array.from(clip.text_content) : [];
  const text = clip.content_type === 'file'
    ? JSON.stringify(fileNames)
    : textCharacters.slice(0, 1_024).join('') || null;
  return {
    ...clip,
    text_content: text,
    html_content: null,
    image_base64: null,
    image_path: null,
    is_summary: true,
    preview_truncated: clip.content_type !== 'file' && textCharacters.length > 1_024,
    file_names: fileNames,
  };
}

export function mergeVisibleClipSnapshots(history: ClipItem[], visible: ClipItem[]): ClipItem[] {
  const uniqueHistory = uniqueClipItemsById(history);
  const uniqueVisible = uniqueClipItemsById(visible);
  const visibleById = new Map(uniqueVisible.map((clip) => [clip.id, clip]));
  const historyIds = new Set(uniqueHistory.map((clip) => clip.id));
  return [
    ...uniqueHistory.map((clip) => visibleById.get(clip.id) ?? clip),
    ...uniqueVisible.filter((clip) => !historyIds.has(clip.id)),
  ];
}
