import { translate } from '../localization/runtime';
import type { ClipItem } from '../types';

export function getClipFilePaths(clip: Pick<ClipItem, 'content_type' | 'text_content'>): string[] {
  if (clip.content_type !== 'file' || !clip.text_content) return [];
  try {
    const paths = JSON.parse(clip.text_content);
    if (Array.isArray(paths) && paths.every((path) => typeof path === 'string')) return paths;
    if (typeof paths === 'string' && paths.trim()) return [paths.trim()];
  } catch {
    // Early file clips stored either one path or a newline-delimited selection.
  }
  return clip.text_content.split(/\r?\n/).map((path) => path.trim()).filter(Boolean);
}

export function getClipFileSummary(
  clip: Pick<ClipItem, 'content_type' | 'text_content' | 'file_count'>,
): string {
  const paths = getClipFilePaths(clip);
  if (paths.length === 0) return translate('component.analyticsView.files');
  const name = paths[0].split(/[\\/]/).filter(Boolean).pop() || paths[0];
  const count = Math.max(paths.length, clip.file_count ?? 0);
  return count === 1 ? name : translate('format.fileSummaryMore', { name, count: count - 1 });
}
