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
  pin_order: number;
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
    pin_order: 0,
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
    pin_order: 0,
    is_trashed: 0,
    bin_id: null,
    bin_ids: [],
  },
];

let mockBins = [
  { id: 1, name: 'My Manual Bin', icon: '📂', color: '#3b82f6', smart_rule: null, bin_type: 'category' },
  { id: 2, name: 'Work Bin', icon: '💼', color: '#10b981', smart_rule: '', bin_type: 'category' },
];

function assignMockClips(ids: number[], binId: number | null) {
  for (const clip of mockClips) {
    if (!ids.includes(clip.id) || clip.is_trashed !== 0) continue;
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
      clip_count: mockClips.filter((clip) => clip.is_trashed === 0 && clip.bin_ids.includes(bin.id)).length,
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
    case 'get_clip_version_count':
      return 0 as unknown as T;
    case 'get_clip_image':
      return null as unknown as T;
    case 'update_clip_text': {
      const clipId = Number(args?.clipId);
      const clip = mockClips.find((item) => item.id === clipId);
      if (clip && clip.is_trashed === 0 && typeof args?.text === 'string') clip.text_content = args.text;
      return null as unknown as T;
    }
    case 'update_clip_note': {
      const clipId = Number(args?.clipId);
      const clip = mockClips.find((item) => item.id === clipId);
      if (clip && clip.is_trashed === 0) clip.note = typeof args?.note === 'string' ? args.note : null;
      return null as unknown as T;
    }
    case 'delete_clip': {
      const clip = mockClips.find((item) => item.id === Number(args?.id));
      if (clip && !clip.is_protected) {
        clip.is_trashed = 1;
        clip.bin_id = null;
        clip.bin_ids = clip.bin_ids.filter((id) => mockBins.find((bin) => bin.id === id)?.bin_type === 'tag');
      }
      return null as unknown as T;
    }
    case 'batch_trash_clips': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(Number) : [];
      for (const clip of mockClips) {
        if (ids.includes(clip.id) && !clip.is_protected) {
          clip.is_trashed = 1;
          clip.bin_id = null;
          clip.bin_ids = clip.bin_ids.filter((id) => mockBins.find((bin) => bin.id === id)?.bin_type === 'tag');
        }
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
    case 'create_bin': {
      const id = Math.max(0, ...mockBins.map((bin) => bin.id)) + 1;
      mockBins.push({
        id,
        name: typeof args?.name === 'string' ? args.name : 'Untitled Bin',
        icon: typeof args?.icon === 'string' ? args.icon : '📂',
        color: typeof args?.color === 'string' ? args.color : '#3b82f6',
        smart_rule: typeof args?.smartRule === 'string' ? args.smartRule : null,
        bin_type: 'category',
      });
      return id as unknown as T;
    }
    case 'update_bin': {
      const bin = mockBins.find((item) => item.id === Number(args?.id));
      if (bin) {
        if (typeof args?.name === 'string') bin.name = args.name;
        if (typeof args?.icon === 'string') bin.icon = args.icon;
        if (typeof args?.color === 'string') bin.color = args.color;
        bin.smart_rule = typeof args?.smartRule === 'string' ? args.smartRule : null;
      }
      return null as unknown as T;
    }
    case 'delete_bin': {
      const id = Number(args?.id);
      mockBins = mockBins.filter((bin) => bin.id !== id);
      for (const clip of mockClips) {
        clip.bin_ids = clip.bin_ids.filter((binId) => binId !== id);
        if (clip.bin_id === id) clip.bin_id = null;
      }
      return null as unknown as T;
    }
    case 'toggle_pin_clip': {
      const clip = mockClips.find((item) => item.id === Number(args?.id));
      if (clip) {
        const nextPinned = clip.is_pinned === 0;
        if (nextPinned) {
          for (const item of mockClips) {
            if (item.is_pinned) item.pin_order += 1;
          }
        }
        clip.is_pinned = nextPinned ? 1 : 0;
        clip.pin_order = 0;
      }
      return Boolean(clip?.is_pinned) as unknown as T;
    }
    case 'batch_pin_clips': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(Number).filter(Number.isInteger) : [];
      const pinState = Boolean(args?.pinState);
      if (pinState) {
        for (const clip of mockClips) {
          if (clip.is_pinned && !ids.includes(clip.id)) clip.pin_order += ids.length;
        }
      }
      ids.forEach((id, index) => {
        const clip = mockClips.find((item) => item.id === id);
        if (clip) {
          clip.is_pinned = pinState ? 1 : 0;
          clip.pin_order = pinState ? index : 0;
        }
      });
      return null as unknown as T;
    }
    case 'reorder_pinned_clips': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(Number).filter(Number.isInteger) : [];
      ids.forEach((id, index) => {
        const clip = mockClips.find((item) => item.id === id);
        if (clip?.is_pinned) clip.pin_order = index;
      });
      return null as unknown as T;
    }
    case 'toggle_clip_protected': {
      const clip = mockClips.find((item) => item.id === Number(args?.clipId));
      if (clip) clip.is_protected = clip.is_protected ? 0 : 1;
      return null as unknown as T;
    }
    default:
      return null as unknown as T;
  }
}
