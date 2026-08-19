import { handled, unhandled, type BrowserMockResult } from './result';

interface AnalyticsClip {
  is_trashed: number;
  text_content: string;
  source: string;
  content_type: string;
  content_types?: string[];
}

export function handleAnalyticsBrowserMock(command: string, clips: readonly AnalyticsClip[]): BrowserMockResult {
  if (command !== 'get_analytics_summary') return unhandled;
  const active = clips.filter((clip) => clip.is_trashed === 0);
  const countBy = (values: string[]) => Object.entries(values.reduce<Record<string, number>>((counts, value) => {
    counts[value] = (counts[value] ?? 0) + 1;
    return counts;
  }, {}));
  return handled({
    total_clips: active.length,
    total_chars: active.reduce((total, clip) => total + clip.text_content.length, 0),
    top_sources: countBy(active.map((clip) => clip.source)).map(([name, count]) => ({ name, count })),
    clip_types: countBy(active.map((clip) => clip.content_type === 'image' || clip.content_type === 'file' ? clip.content_type : 'text'))
      .map(([clip_type, count]) => ({ clip_type, count })),
    file_formats: [],
    content_types: countBy(active.flatMap((clip) => [...new Set(clip.content_types ?? [])]))
      .map(([content_type, count]) => ({ content_type, count })),
    daily_activity: [],
  });
}
