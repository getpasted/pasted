import { invoke as tauriInvoke } from '@tauri-apps/api/core';

type MockClip = {
  id: number;
  text_content: string;
  content_type: string;
  source_app: string;
  created_at: string;
  char_count: number;
  word_count: number;
  line_count: number;
  is_pinned: number;
  is_protected: number;
  is_trashed: number;
  bin_id: number | null;
  bin_ids: number[];
  note?: string | null;
};

let mockClips: MockClip[] = [
  {
    id: 101,
    text_content: 'Sample Clip 1 for Drag Testing',
    content_type: 'text',
    source_app: 'Safari',
    created_at: new Date().toISOString(),
    char_count: 30,
    word_count: 6,
    line_count: 1,
    is_pinned: 0,
    is_protected: 0,
    is_trashed: 0,
    bin_id: null,
    bin_ids: [],
  },
  {
    id: 102,
    text_content: 'Sample Clip 2 for Drag Testing',
    content_type: 'text',
    source_app: 'VS Code',
    created_at: new Date().toISOString(),
    char_count: 30,
    word_count: 6,
    line_count: 1,
    is_pinned: 0,
    is_protected: 0,
    is_trashed: 0,
    bin_id: null,
    bin_ids: [],
  },
];

const mockBins = [
  { id: 1, name: 'My Manual Bin', icon: '📂', color: '#3b82f6', smart_rule: null, bin_type: 'category' },
  { id: 2, name: 'Work Bin', icon: '💼', color: '#10b981', smart_rule: '', bin_type: 'category' },
];

function assignMockClips(ids: number[], binId: number | null) {
  for (const clip of mockClips) {
    if (!ids.includes(clip.id)) continue;
    clip.bin_id = binId;
    const tagIds = clip.bin_ids.filter((id) => mockBins.find((bin) => bin.id === id)?.bin_type === 'tag');
    clip.bin_ids = binId === null ? tagIds : [...tagIds, binId];
  }
}

export async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
    return tauriInvoke<T>(cmd, args);
  }
  console.warn(`[safeInvoke mock] ${cmd}`, args);
  switch (cmd) {
    case 'get_clips':
      return mockClips
        .filter((clip) => {
          const binId = Number(args?.binId);
          return clip.is_trashed === 0
            && (!Number.isInteger(binId) || binId <= 0 || clip.bin_ids.includes(binId));
        })
        .map((clip) => ({ ...clip, bin_ids: [...clip.bin_ids] })) as unknown as T;
    case 'get_bins':
      return mockBins.map((bin) => ({
        ...bin,
        clip_count: mockClips.filter((clip) => clip.bin_ids.includes(bin.id)).length,
      })) as unknown as T;
    case 'get_filters':
      return [] as unknown as T;
    case 'get_sequential_status':
      return { active: false, queue: [], current_index: 0 } as unknown as T;
    case 'get_trashed_clips':
      return mockClips.filter((clip) => clip.is_trashed !== 0) as unknown as T;
    case 'get_total_clip_count':
      return mockClips.filter((clip) => clip.is_trashed === 0).length as unknown as T;
    case 'is_clipboard_paused':
      return false as unknown as T;
    case 'get_app_settings':
      return {} as unknown as T;
    case 'get_operations':
      return [] as unknown as T;
    case 'get_activity_logs':
      return [] as unknown as T;
    case 'get_clip_versions':
      return [] as unknown as T;
    case 'get_clip_image':
      return null as unknown as T;
    case 'update_clip_text': {
      const clipId = Number(args?.clipId);
      const clip = mockClips.find((item) => item.id === clipId);
      if (clip && typeof args?.text === 'string') clip.text_content = args.text;
      return null as unknown as T;
    }
    case 'update_clip_note': {
      const clipId = Number(args?.clipId);
      const clip = mockClips.find((item) => item.id === clipId);
      if (clip) clip.note = typeof args?.note === 'string' ? args.note : null;
      return null as unknown as T;
    }
    case 'delete_clip': {
      const clip = mockClips.find((item) => item.id === Number(args?.id));
      if (clip && !clip.is_protected) clip.is_trashed = 1;
      return null as unknown as T;
    }
    case 'batch_trash_clips': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(Number) : [];
      for (const clip of mockClips) {
        if (ids.includes(clip.id) && !clip.is_protected) clip.is_trashed = 1;
      }
      return null as unknown as T;
    }
    case 'restore_clip': {
      const clip = mockClips.find((item) => item.id === Number(args?.id));
      if (clip) clip.is_trashed = 0;
      return null as unknown as T;
    }
    case 'purge_clip_permanently':
      mockClips = mockClips.filter((clip) => clip.id !== Number(args?.id) || clip.is_protected);
      return null as unknown as T;
    case 'empty_trash':
      mockClips = mockClips.filter((clip) => clip.is_trashed === 0 || clip.is_protected);
      return null as unknown as T;
    case 'assign_clip_bin': {
      const clipId = Number(args?.clipId);
      const binId = args?.binId === null ? null : Number(args?.binId);
      if (Number.isInteger(clipId) && (binId === null || Number.isInteger(binId))) {
        assignMockClips([clipId], binId);
      }
      return true as unknown as T;
    }
    case 'batch_assign_bin_clips': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(Number).filter(Number.isInteger) : [];
      const binId = args?.binId === null ? null : Number(args?.binId);
      if (binId === null || Number.isInteger(binId)) assignMockClips(ids, binId);
      return true as unknown as T;
    }
    default:
      return null as unknown as T;
  }
}
