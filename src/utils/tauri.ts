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
  board_id: number | null;
  board_ids: number[];
};

const mockClips: MockClip[] = [
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
    board_id: null,
    board_ids: [],
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
    board_id: null,
    board_ids: [],
  },
];

const mockBoards = [
  { id: 1, name: 'My Manual Bin', icon: '📂', color: '#3b82f6', smart_rule: null, board_type: 'category' },
  { id: 2, name: 'Work Bin', icon: '💼', color: '#10b981', smart_rule: '', board_type: 'category' },
];

function assignMockClips(ids: number[], boardId: number | null) {
  for (const clip of mockClips) {
    if (!ids.includes(clip.id)) continue;
    clip.board_id = boardId;
    const tagIds = clip.board_ids.filter((id) => mockBoards.find((board) => board.id === id)?.board_type === 'tag');
    clip.board_ids = boardId === null ? tagIds : [...tagIds, boardId];
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
          const boardId = Number(args?.boardId);
          return !Number.isInteger(boardId) || boardId <= 0 || clip.board_ids.includes(boardId);
        })
        .map((clip) => ({ ...clip, board_ids: [...clip.board_ids] })) as unknown as T;
    case 'get_boards':
      return mockBoards.map((board) => ({
        ...board,
        clip_count: mockClips.filter((clip) => clip.board_ids.includes(board.id)).length,
      })) as unknown as T;
    case 'get_filters':
      return [] as unknown as T;
    case 'get_sequential_status':
      return { active: false, queue: [], current_index: 0 } as unknown as T;
    case 'get_trashed_clips':
      return [] as unknown as T;
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
    case 'assign_clip_board': {
      const clipId = Number(args?.clipId);
      const boardId = args?.boardId === null ? null : Number(args?.boardId);
      if (Number.isInteger(clipId) && (boardId === null || Number.isInteger(boardId))) {
        assignMockClips([clipId], boardId);
      }
      return true as unknown as T;
    }
    case 'batch_assign_board_clips': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(Number).filter(Number.isInteger) : [];
      const boardId = args?.boardId === null ? null : Number(args?.boardId);
      if (boardId === null || Number.isInteger(boardId)) assignMockClips(ids, boardId);
      return true as unknown as T;
    }
    default:
      return null as unknown as T;
  }
}
