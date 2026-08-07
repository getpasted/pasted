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
  is_transformed?: number;
  pin_order: number;
  is_trashed: number;
  trashed_at?: string | null;
  bin_id: number | null;
  bin_ids: number[];
  note?: string | null;
};

type MockBin = {
  id: number;
  name: string;
  icon: string;
  color: string;
  smart_rule: string | null;
  bin_type: string;
  clip_order?: number[];
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

let mockBins: MockBin[] = [
  { id: 1, name: 'My Manual Bin', icon: '📂', color: 'default', smart_rule: null, bin_type: 'category' },
  { id: 2, name: 'Work Bin', icon: '💼', color: '#10b981', smart_rule: '', bin_type: 'category' },
];

const mockPipelines = [
  {
    id: 1,
    stableRef: 'pipeline:mock-uppercase',
    name: 'Uppercase',
    shortcut: null,
    revision: 1,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    steps: [{ position: 0, operationRef: 'builtin:uppercase', configJson: null, failurePolicy: 'stop' }],
  },
  {
    id: 2,
    stableRef: 'pipeline:mock-clean-url',
    name: 'Clean URL',
    shortcut: null,
    revision: 1,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    steps: [{ position: 0, operationRef: 'builtin:clean_url_tracking', configJson: null, failurePolicy: 'stop' }],
  },
];

let mockIntelligenceConnections: Array<{
  id: string;
  name: string;
  providerKind: string;
  endpoint: string | null;
  model: string | null;
  credentialRef: string | null;
  enabled: boolean;
  priority: number;
  createdAt: string;
  updatedAt: string;
}> = [];

let mockSavedTransforms: Array<Record<string, unknown>> = [];
let mockClipTransformations = new Map<number, Record<string, unknown>>();
let mockBinTransforms = new Map<number, string>();
let mockSequentialStatus = {
  is_active: false,
  queue: [] as string[],
  item_ids: [] as number[],
  current_index: 0,
  total_count: 0,
};
let mockSequentialNextItemId = 1;

function updateMockSequentialStatus() {
  mockSequentialStatus = {
    ...mockSequentialStatus,
    total_count: mockSequentialStatus.queue.length,
  };
  return {
    ...mockSequentialStatus,
    queue: [...mockSequentialStatus.queue],
    item_ids: [...mockSequentialStatus.item_ids],
  };
}

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
    case 'get_pipelines':
      return mockPipelines as unknown as T;
    case 'get_hotkey_capability_status':
      return {
        platform: 'unsupported',
        backend: 'unsupported',
        state: 'unavailable',
        is_trusted: true,
        is_dev_mode: false,
        configured_count: 0,
        registered_count: 0,
        issues: [],
      } as unknown as T;
    case 'get_bin_transform_ref':
      return (mockBinTransforms.get(Number(args?.binId)) || null) as unknown as T;
    case 'set_bin_transform_ref': {
      const binId = Number(args?.binId);
      if (args?.transformRef) mockBinTransforms.set(binId, String(args.transformRef));
      else mockBinTransforms.delete(binId);
      return null as unknown as T;
    }
    case 'get_intelligence_connections':
      return [...mockIntelligenceConnections]
        .sort((left, right) => left.priority - right.priority)
        .map((connection) => ({ ...connection })) as unknown as T;
    case 'detect_intelligence_connections': {
      const detected = [
        { adapterId: 'codex_cli', name: 'Codex CLI', providerKind: 'cli', executablePath: '/opt/homebrew/bin/codex', defaultEndpoint: null, version: 'codex-cli', capabilities: ['structured_output', 'json_events', 'local_models'], executionSupported: true },
        { adapterId: 'claude_cli', name: 'Claude CLI', providerKind: 'cli', executablePath: '/opt/homebrew/bin/claude', defaultEndpoint: null, version: 'claude', capabilities: ['non_interactive', 'structured_output'], executionSupported: false },
        { adapterId: 'ollama', name: 'Ollama', providerKind: 'ollama', executablePath: '/opt/homebrew/bin/ollama', defaultEndpoint: 'http://127.0.0.1:11434', version: 'ollama', capabilities: ['local', 'openai_compatible'], executionSupported: false },
        { adapterId: 'antigravity_ide', name: 'Antigravity IDE', providerKind: 'cli', executablePath: '/Applications/Antigravity IDE.app/Contents/Resources/app/bin/antigravity-ide', defaultEndpoint: null, version: 'Antigravity IDE', capabilities: ['interactive_chat', 'mcp_client'], executionSupported: false },
      ];
      for (const candidate of detected) {
        const endpoint = candidate.providerKind === 'cli' ? candidate.executablePath : candidate.defaultEndpoint;
        if (mockIntelligenceConnections.some((connection) => connection.providerKind === candidate.providerKind && connection.endpoint === endpoint)) continue;
        const now = new Date().toISOString();
        mockIntelligenceConnections.push({
          id: `mock-detected-${candidate.adapterId}`,
          name: candidate.name,
          providerKind: candidate.providerKind,
          endpoint,
          model: null,
          credentialRef: null,
          enabled: false,
          priority: mockIntelligenceConnections.length,
          createdAt: now,
          updatedAt: now,
        });
      }
      return detected as unknown as T;
    }
    case 'create_intelligence_connection': {
      const now = new Date().toISOString();
      const connection = {
        id: `mock-connection-${Date.now()}`,
        name: String(args?.name || 'AI Connection'),
        providerKind: String(args?.providerKind || 'ollama'),
        endpoint: typeof args?.endpoint === 'string' ? args.endpoint : null,
        model: typeof args?.model === 'string' ? args.model : null,
        credentialRef: typeof args?.credentialRef === 'string' ? args.credentialRef : null,
        enabled: true,
        priority: mockIntelligenceConnections.length,
        createdAt: now,
        updatedAt: now,
      };
      mockIntelligenceConnections.push(connection);
      return connection as unknown as T;
    }
    case 'update_intelligence_connection': {
      const connection = mockIntelligenceConnections.find((item) => item.id === args?.id);
      if (connection) {
        connection.name = String(args?.name || connection.name);
        connection.providerKind = String(args?.providerKind || connection.providerKind);
        connection.endpoint = typeof args?.endpoint === 'string' ? args.endpoint : null;
        connection.model = typeof args?.model === 'string' ? args.model : null;
        connection.credentialRef = typeof args?.credentialRef === 'string' ? args.credentialRef : null;
        connection.enabled = Boolean(args?.enabled);
        connection.updatedAt = new Date().toISOString();
      }
      return null as unknown as T;
    }
    case 'delete_intelligence_connection':
      mockIntelligenceConnections = mockIntelligenceConnections.filter((connection) => connection.id !== args?.id);
      return null as unknown as T;
    case 'reorder_intelligence_connections': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(String) : [];
      ids.forEach((id, priority) => {
        const connection = mockIntelligenceConnections.find((item) => item.id === id);
        if (connection) connection.priority = priority;
      });
      return null as unknown as T;
    }
    case 'execute_transformation': {
      const request = args?.request as { input?: string; target?: { kind?: string; transformRef?: string; pipelineRef?: string; operationRef?: string } } | undefined;
      const input = request?.input || '';
      const targetRef = request?.target?.transformRef || request?.target?.pipelineRef || request?.target?.operationRef || '';
      const output = request?.target?.kind === 'transform'
        ? `# ${input}`
        : targetRef.includes('uppercase') ? input.toUpperCase() : input;
      return {
        executionId: 'mock-execution',
        output,
        connectionId: request?.target?.kind === 'transform' ? 'mock-detected-codex_cli' : null,
        connectionName: request?.target?.kind === 'transform' ? 'Codex CLI' : null,
        durationMs: request?.target?.kind === 'transform' ? 260 : 1,
      } as unknown as T;
    }
    case 'cancel_transformation_execution':
      return true as unknown as T;
    case 'get_intelligence_scheduler_snapshot':
      return {
        revision: 0,
        activeCount: 0,
        queuedCount: 0,
        jobs: [],
        recentEvents: [],
      } as unknown as T;
    case 'get_installation_diagnostics':
      return {
        appVersion: '1.0.0',
        buildKind: 'Development',
        platform: 'macos',
        architecture: 'aarch64',
        bundleIdentifier: 'software.jjj.pasted',
        appPath: '/Applications/Pasted.app',
        dataPath: '/Users/example/Library/Application Support/software.jjj.pasted',
        databaseSizeBytes: 2_457_600,
        signingStatus: 'Ad hoc',
        signingIdentity: null,
        signingTeamId: null,
        notarizationStatus: 'Not expected for development builds',
        cliPath: '/Applications/Pasted.app/Contents/MacOS/pasted-cli',
      } as unknown as T;
    case 'run_intelligence_scheduler_demo':
      return undefined as T;
    case 'get_ocr_backfill_status':
      return {
        totalImages: 0,
        eligibleCount: 0,
        queuedCount: 0,
        runningCount: 0,
        completedCount: 0,
        noTextCount: 0,
        failedCount: 0,
      } as unknown as T;
    case 'start_ocr_backfill':
    case 'cancel_ocr_backfill':
      return undefined as T;
    case 'retry_failed_ocr':
      return 0 as unknown as T;
    case 'plan_transformation_intent': {
      const request = args?.request as { intent?: string; sampleInput?: string; planningMode?: string } | undefined;
      await new Promise((resolve) => window.setTimeout(resolve, 220));
      return {
        plan: {
          schema_version: 1,
          intent: request?.intent || '',
          summary: 'Apply the requested transformation with semantic judgment.',
          planning_mode: request?.planningMode || 'pinned',
          steps: [{
            name: 'Transform text',
            rationale: 'The requested outcome depends on language and context.',
            scope: 'whole_input',
            executor: {
              kind: 'semantic',
              instructions: request?.intent || 'Transform the text as requested.',
              model_policy: 'balanced',
            },
          }],
        },
        connectionId: 'mock-detected-codex_cli',
        connectionName: 'Codex CLI',
        durationMs: 220,
      } as unknown as T;
    }
    case 'test_transformation_plan': {
      const request = args?.request as { input?: string; plan?: { intent?: string } } | undefined;
      await new Promise((resolve) => window.setTimeout(resolve, 260));
      return {
        output: `# ${request?.input || 'Transformed text'}`,
        connectionId: 'mock-detected-codex_cli',
        connectionName: 'Codex CLI',
        durationMs: 260,
      } as unknown as T;
    }
    case 'get_saved_transforms':
      return mockSavedTransforms.map((transform) => ({ ...transform })) as unknown as T;
    case 'save_saved_transform': {
      const now = new Date().toISOString();
      const plan = args?.plan as { summary?: string } | undefined;
      const transform = {
        id: Date.now(),
        stableRef: `transform:mock-${Date.now()}`,
        name: String(args?.name || plan?.summary || 'Untitled Transform'),
        plan,
        connectionId: args?.connectionId || null,
        revision: 1,
        createdAt: now,
        updatedAt: now,
      };
      mockSavedTransforms.unshift(transform);
      return transform as unknown as T;
    }
    case 'update_saved_transform': {
      const index = mockSavedTransforms.findIndex((transform) => transform.stableRef === args?.transformRef);
      if (index < 0) throw new Error('Transform not found');
      const current = mockSavedTransforms[index];
      const updated = {
        ...current,
        name: String(args?.name || current.name),
        plan: args?.plan || current.plan,
        connectionId: args?.connectionId || null,
        revision: Number(current.revision || 1) + 1,
        updatedAt: new Date().toISOString(),
      };
      mockSavedTransforms.splice(index, 1);
      mockSavedTransforms.unshift(updated);
      return updated as unknown as T;
    }
    case 'delete_saved_transform':
      mockSavedTransforms = mockSavedTransforms.filter((transform) => transform.stableRef !== args?.transformRef);
      return null as unknown as T;
    case 'apply_transform_preview_to_clip': {
      const clipId = Number(args?.clipId);
      const clip = mockClips.find((item) => item.id === clipId);
      if (clip) clip.text_content = String(args?.output || clip.text_content);
      if (clip) clip.is_transformed = 1;
      const transform = mockSavedTransforms.find((item) => item.stableRef === args?.transformRef);
      const provenance = {
        transformRef: String(args?.transformRef || ''),
        transformName: String(transform?.name || 'Transform'),
        transformRevision: Number(transform?.revision || 1),
        connectionId: args?.connectionId || null,
        durationMs: Number(args?.durationMs || 0),
        createdAt: new Date().toISOString(),
      };
      mockClipTransformations.set(clipId, provenance);
      return provenance as unknown as T;
    }
    case 'get_clip_transformation_provenance':
      return (mockClipTransformations.get(Number(args?.clipId)) || null) as unknown as T;
    case 'copy_clip_to_system':
    case 'paste_text_to_frontmost':
      return null as unknown as T;
    case 'get_sequential_status':
      return updateMockSequentialStatus() as unknown as T;
    case 'get_queue_paste_target':
      return { name: 'Browser' } as unknown as T;
    case 'start_sequential_paste':
      mockSequentialStatus.is_active = true;
      return updateMockSequentialStatus() as unknown as T;
    case 'stop_sequential_paste':
      mockSequentialStatus.is_active = false;
      return updateMockSequentialStatus() as unknown as T;
    case 'push_sequential_item':
      mockSequentialStatus.queue.push(String(args?.item ?? ''));
      mockSequentialStatus.item_ids.push(mockSequentialNextItemId++);
      return updateMockSequentialStatus() as unknown as T;
    case 'pop_sequential_paste': {
      const item = mockSequentialStatus.queue.shift() ?? null;
      mockSequentialStatus.item_ids.shift();
      updateMockSequentialStatus();
      return item as unknown as T;
    }
    case 'paste_sequential_item_by_index': {
      const index = Number(args?.index ?? -1);
      const [item] = mockSequentialStatus.queue.splice(index, 1);
      mockSequentialStatus.item_ids.splice(index, 1);
      updateMockSequentialStatus();
      return (item ?? null) as unknown as T;
    }
    case 'remove_sequential_item_by_index':
      mockSequentialStatus.queue.splice(Number(args?.index ?? -1), 1);
      mockSequentialStatus.item_ids.splice(Number(args?.index ?? -1), 1);
      return updateMockSequentialStatus() as unknown as T;
    case 'reorder_sequential_items': {
      const orderedIds = Array.isArray(args?.itemIds) ? args.itemIds.map(Number) : [];
      const textById = new Map(mockSequentialStatus.item_ids.map((id, index) => [id, mockSequentialStatus.queue[index]]));
      mockSequentialStatus.item_ids = orderedIds;
      mockSequentialStatus.queue = orderedIds.map((id) => textById.get(id) ?? '');
      return updateMockSequentialStatus() as unknown as T;
    }
    case 'paste_all_sequential': {
      const combined = mockSequentialStatus.queue.length > 0
        ? mockSequentialStatus.queue.join('\n\n')
        : null;
      mockSequentialStatus.queue = [];
      mockSequentialStatus.item_ids = [];
      mockSequentialStatus.is_active = false;
      updateMockSequentialStatus();
      return combined as unknown as T;
    }
    case 'get_trashed_clips':
      return mockClips.filter((clip) => clip.is_trashed !== 0) as unknown as T;
    case 'get_total_clip_count':
      return mockClips.filter((clip) => clip.is_trashed === 0).length as unknown as T;
    case 'is_clipboard_paused':
      return false as unknown as T;
    case 'get_all_app_settings':
      return {} as unknown as T;
    case 'export_backup_file':
      return `/mock/Pasted_Backup_${new Date().toISOString().slice(0, 10)}.json` as unknown as T;
    case 'factory_reset_app': {
      const report = {
        clipsDeleted: mockClips.length,
        binsDeleted: mockBins.length,
        transformsDeleted: mockSavedTransforms.length,
        connectionsDeleted: mockIntelligenceConnections.length,
        activityEntriesDeleted: 0,
      };
      mockClips = [];
      mockBins = [
        { id: 1, name: 'Code Snippets', icon: 'Code', color: '#10b981', smart_rule: '{"type":"content_type","value":"code"}', bin_type: 'category' },
        { id: 2, name: 'Links & Web', icon: 'Link', color: '#3b82f6', smart_rule: '{"type":"content_type","value":"link"}', bin_type: 'category' },
        { id: 3, name: 'Colors & Swatches', icon: 'Palette', color: '#f59e0b', smart_rule: '{"type":"content_type","value":"color"}', bin_type: 'category' },
      ];
      mockSavedTransforms = [];
      mockIntelligenceConnections = [];
      mockClipTransformations = new Map();
      mockBinTransforms = new Map();
      return report as unknown as T;
    }
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
    case 'get_file_clip_metadata':
      return {
        itemCount: 0,
        availableCount: 0,
        fileCount: 0,
        directoryCount: 0,
        totalSizeBytes: 0,
        extensions: [],
      } as unknown as T;
    case 'get_file_clip_previews':
      return [] as unknown as T;
    case 'restore_clip_version': {
      const clipId = Number(args?.clipId);
      const clip = mockClips.find((item) => item.id === clipId);
      return (clip ? { ...clip } : null) as unknown as T;
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
      const transformed = binId !== null && mockBinTransforms.has(binId)
        ? mockClips.find((clip) => clip.id === clipId) ?? null
        : null;
      return (transformed ? { ...transformed, is_transformed: true } : null) as unknown as T;
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
        color: typeof args?.color === 'string' ? args.color : 'default',
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
      const disposition = typeof args?.disposition === 'string' ? args.disposition : 'keep';
      const destinationBinId = Number(args?.destinationBinId);
      mockBins = mockBins.filter((bin) => bin.id !== id);
      for (const clip of mockClips) {
        const belongsToBin = clip.bin_id === id || clip.bin_ids.includes(id);
        if (!belongsToBin) continue;
        clip.bin_ids = clip.bin_ids.filter((binId) => binId !== id);
        if (disposition === 'move' && Number.isFinite(destinationBinId)) {
          clip.bin_ids = clip.bin_ids.filter((binId) => {
            const candidate = mockBins.find((bin) => bin.id === binId);
            return candidate?.bin_type === 'tag';
          });
          clip.bin_ids.push(destinationBinId);
          clip.bin_id = destinationBinId;
        } else if (disposition === 'trash' && !clip.is_protected) {
          clip.bin_ids = [];
          clip.bin_id = null;
          clip.is_trashed = 1;
          clip.trashed_at = new Date().toISOString();
        } else if (clip.bin_id === id) {
          clip.bin_id = null;
        }
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
    case 'reorder_bin_clips': {
      const binId = Number(args?.binId);
      const clipIds = Array.isArray(args?.clipIds) ? args.clipIds.map(Number) : [];
      const bin = mockBins.find((item) => item.id === binId);
      if (bin) bin.clip_order = clipIds;
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
