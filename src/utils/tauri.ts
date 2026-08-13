import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { CONTENT_TYPES } from './contentTypes';

type MockClip = {
  id: number;
  text_content: string;
  content_type: string;
  source: string;
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
    source: 'Safari',
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
    source: 'VS Code',
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

function mockDetector<T extends { id: number; patterns: string[] }>(detector: T) {
  return { ...detector, defaults: { ...detector, patterns: [...detector.patterns] } };
}
let mockDetectors = [
  mockDetector({ id: 1, stable_ref: 'email', name: 'Email Addresses', content_type: 'email', description: 'Individual email addresses', patterns: [String.raw`(?i)^[^\s@]+@[^\s@]+\.[^\s@]+$`], validator: null, enabled: true, priority: 30, is_builtin: true }),
  mockDetector({ id: 2, stable_ref: 'credential', name: 'Credentials', content_type: 'credential', description: 'Known API-key formats and secret assignments', patterns: [String.raw`^(?:sk_|ghp_).+$`], validator: null, enabled: true, priority: 60, is_builtin: true }),
  mockDetector({ id: 3, stable_ref: 'phone', name: 'Phone Numbers', content_type: 'phone', description: 'Formatted international and local phone numbers', patterns: [String.raw`^\+?[0-9 ()-]{7,}$`], validator: 'phone', enabled: true, priority: 160, is_builtin: true }),
];

let mockContentTypes: Array<{
  id: string; label: string; icon: string; group: string; isBuiltin: boolean; isArchived: boolean;
  defaults: { label: string; icon: string; group: string } | null;
}> = CONTENT_TYPES.map(({ value, label, icon, group }) => {
  const groupId = group.toLowerCase().replace(/ & /g, '_').replace(/ /g, '_');
  return { id: value as string, label, icon, group: groupId, isBuiltin: true, isArchived: false, defaults: { label, icon, group: groupId } };
});
let mockContentTypeGroups: Array<{
  id: string; label: string; sortOrder: number; isBuiltin: boolean; isArchived: boolean;
  defaults: { label: string; sortOrder: number } | null;
}> = [
  { id: 'general', label: 'General', sortOrder: 10, isBuiltin: true, isArchived: false, defaults: { label: 'General', sortOrder: 10 } },
  { id: 'developer', label: 'Developer', sortOrder: 20, isBuiltin: true, isArchived: false, defaults: { label: 'Developer', sortOrder: 20 } },
  { id: 'personal_financial', label: 'Personal and financial', sortOrder: 30, isBuiltin: true, isArchived: false, defaults: { label: 'Personal and financial', sortOrder: 30 } },
  { id: 'identifiers', label: 'Identifiers', sortOrder: 40, isBuiltin: true, isArchived: false, defaults: { label: 'Identifiers', sortOrder: 40 } },
  { id: 'custom', label: 'Custom', sortOrder: 50, isBuiltin: true, isArchived: false, defaults: { label: 'Custom', sortOrder: 50 } },
];

let mockLibraryLocation = {
  path: '/mock/Pasted/pasted.db',
  directory: '/mock/Pasted',
  isDefault: true,
};

const mockPipelines = [
  {
    id: 1,
    stableRef: 'transform:mock-uppercase',
    name: 'Uppercase',
    shortcut: null,
    revision: 1,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    steps: [{ position: 0, operationRef: 'builtin:uppercase', configJson: null, failurePolicy: 'stop' }],
  },
  {
    id: 2,
    stableRef: 'transform:mock-clean-url',
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
    case 'get_clips': {
      const offset = Math.max(0, Number(args?.offset ?? 0));
      const limit = Math.max(1, Number(args?.limit ?? 10_000));
      return mockClips
        .filter((clip) => {
          const binId = Number(args?.binId);
          return clip.is_trashed === 0
            && (!Number.isInteger(binId) || binId <= 0 || clip.bin_ids.includes(binId));
        })
        .slice(offset, offset + limit)
        .map((clip) => ({ ...clip, bin_ids: [...clip.bin_ids] })) as unknown as T;
    }
    case 'get_bins':
      return mockBins.map((bin) => ({
        ...bin,
      clip_count: mockClips.filter((clip) => clip.is_trashed === 0 && clip.bin_ids.includes(bin.id)).length,
      })) as unknown as T;
    case 'get_pipelines':
      return mockPipelines as unknown as T;
    case 'get_content_detectors':
      return mockDetectors.map((detector) => ({ ...detector, patterns: [...detector.patterns] })) as unknown as T;
    case 'get_library_items': {
      const kind = String(args?.kind ?? '');
      const items = [
        ...mockDetectors.map((detector) => ({ stableRef: detector.stable_ref, kind: 'detector', name: detector.name, description: detector.description, groupLabel: null, icon: 'FileText', enabled: detector.enabled, isBuiltin: detector.is_builtin, isArchived: false, sortOrder: detector.priority, revision: 1, inputContract: 'text', outputContract: `set_type:${detector.content_type}`, createdAt: '', updatedAt: '', capabilities: { canEdit: true, canDuplicate: true, canDelete: true, canDisable: true, canRestore: detector.is_builtin } })),
        ...mockPipelines.map((pipeline) => ({ stableRef: pipeline.stableRef, kind: 'transform', name: pipeline.name, description: '', groupLabel: 'Manual Transforms', icon: 'Workflow', enabled: null, isBuiltin: false, isArchived: false, sortOrder: pipeline.id, revision: pipeline.revision, inputContract: 'text', outputContract: 'preserve_type', createdAt: pipeline.createdAt, updatedAt: pipeline.updatedAt, capabilities: { canEdit: true, canDuplicate: true, canDelete: true, canDisable: false, canRestore: false } })),
      ];
      return items.filter((item) => !kind || item.kind === kind) as unknown as T;
    }
    case 'set_library_item_enabled':
      return undefined as T;
    case 'get_content_types':
      return mockContentTypes
        .filter((type) => Boolean(args?.includeArchived) || !type.isArchived)
        .map((type) => ({ ...type })) as unknown as T;
    case 'get_content_type_groups':
      return mockContentTypeGroups.filter((group) => Boolean(args?.includeArchived) || !group.isArchived).map((group) => ({ ...group })) as unknown as T;
    case 'create_content_type_group': {
      const input = args?.input as { id: string; label: string; sortOrder: number };
      const created = { ...input, isBuiltin: false, isArchived: false, defaults: null };
      mockContentTypeGroups.push(created);
      return created as unknown as T;
    }
    case 'update_content_type_group': {
      const index = mockContentTypeGroups.findIndex(({ id }) => id === String(args?.id));
      if (index >= 0) mockContentTypeGroups[index] = { ...mockContentTypeGroups[index], ...(args?.input as Record<string, string | number>) };
      return mockContentTypeGroups[index] as unknown as T;
    }
    case 'set_content_type_group_archived': {
      const index = mockContentTypeGroups.findIndex(({ id }) => id === String(args?.id));
      if (index >= 0) mockContentTypeGroups[index] = { ...mockContentTypeGroups[index], isArchived: Boolean(args?.archived) };
      return null as unknown as T;
    }
    case 'delete_content_type_group':
      mockContentTypeGroups = mockContentTypeGroups.filter(({ id }) => id !== String(args?.id));
      return null as unknown as T;
    case 'restore_default_content_type_groups':
      return mockContentTypeGroups as unknown as T;
    case 'create_content_type': {
      const input = args?.input as { id: string; label: string; icon: string; group: string };
      const created = { ...input, isBuiltin: false, isArchived: false, defaults: null };
      mockContentTypes.push(created);
      return created as unknown as T;
    }
    case 'update_content_type': {
      const index = mockContentTypes.findIndex(({ id }) => id === String(args?.id));
      if (index >= 0) mockContentTypes[index] = { ...mockContentTypes[index], ...(args?.input as Record<string, string>) };
      return mockContentTypes[index] as unknown as T;
    }
    case 'set_content_type_archived': {
      const index = mockContentTypes.findIndex(({ id }) => id === String(args?.id));
      if (index >= 0) mockContentTypes[index] = { ...mockContentTypes[index], isArchived: Boolean(args?.archived) };
      return mockContentTypes[index] as unknown as T;
    }
    case 'restore_default_content_types':
      return mockContentTypes as unknown as T;
    case 'create_content_detector': {
      const input = args?.input as Record<string, unknown>;
      const detector = { ...input, id: Math.max(0, ...mockDetectors.map(({ id }) => Number(id))) + 1, stable_ref: `custom-${Date.now()}`, is_builtin: false, defaults: null } as unknown as typeof mockDetectors[number];
      mockDetectors.push(detector);
      return detector as unknown as T;
    }
    case 'update_content_detector': {
      const index = mockDetectors.findIndex(({ id }) => id === Number(args?.id));
      if (index >= 0) mockDetectors[index] = { ...mockDetectors[index], ...(args?.input as Record<string, unknown>) } as typeof mockDetectors[number];
      return mockDetectors[index] as unknown as T;
    }
    case 'delete_content_detector':
      mockDetectors = mockDetectors.filter(({ id }) => id !== Number(args?.id));
      return null as unknown as T;
    case 'restore_default_content_detectors':
      return mockDetectors as unknown as T;
    case 'rescan_content_detection_history':
      return { scannedCount: mockClips.length, changedCount: 0, unchangedCount: mockClips.length } as unknown as T;
    case 'test_content_detector': {
      const input = args?.input as { patterns?: string[] };
      const sample = String(args?.sample ?? '');
      return Boolean(input.patterns?.some((pattern) => {
        try { return new RegExp(pattern.replace(/^\(\?i\)/, ''), pattern.startsWith('(?i)') ? 'i' : '').test(sample); } catch { return false; }
      })) as unknown as T;
    }
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
    case 'preview_pipeline_steps': {
      const input = typeof args?.input === 'string' ? args.input : '';
      const steps = Array.isArray(args?.steps) ? args.steps : [];
      const output = steps.reduce((current: string, step: { operationRef?: string }) => (
        step.operationRef?.includes('uppercase') ? current.toUpperCase() : current
      ), input);
      return output as unknown as T;
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
        cliPath: '/Applications/Pasted.app/Contents/MacOS/pasted',
      } as unknown as T;
    case 'get_third_party_licenses':
      return {
        schemaVersion: 1,
        componentCount: 2,
        components: [
          { ecosystem: 'cargo', name: 'tauri', version: '2.x', license: 'MIT OR Apache-2.0', repository: 'https://github.com/tauri-apps/tauri', noticeIds: ['development'] },
          { ecosystem: 'npm', name: 'react', version: '19.x', license: 'MIT', repository: 'https://github.com/facebook/react', noticeIds: ['development'] },
        ],
        noticeText: [
          'Pasted Third-Party Software Notices',
          '',
          'Development preview',
          '',
          'Production builds embed the complete generated component inventory and license text.',
          'Run `pasted licenses` or open this dialog in the native app to inspect that document.',
        ].join('\n'),
      } as unknown as T;
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
    case 'get_intent_transforms':
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
    case 'copy_clip_by_id':
    case 'paste_text_to_frontmost':
      return null as unknown as T;
    case 'get_sequential_status':
      return updateMockSequentialStatus() as unknown as T;
    case 'get_queue_paste_target':
      return {
        name: 'Browser',
        automaticPasteAvailable: false,
        unavailableReason: 'This window cannot send system-wide paste commands.',
      } as unknown as T;
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
    case 'get_trashed_clips': {
      const offset = Math.max(0, Number(args?.offset ?? 0));
      const limit = Math.max(1, Number(args?.limit ?? 10_000));
      return mockClips.filter((clip) => clip.is_trashed !== 0).slice(offset, offset + limit) as unknown as T;
    }
    case 'get_clip_collection_summary': {
      const active = mockClips.filter((clip) => clip.is_trashed === 0);
      const countBy = (key: 'content_type' | 'source') => [...active.reduce((counts, clip) => counts.set(String(clip[key]), (counts.get(String(clip[key])) ?? 0) + 1), new Map<string, number>())];
      return {
        activeCount: active.length,
        trashCount: mockClips.length - active.length,
        pinnedCount: active.filter((clip) => clip.is_pinned).length,
        protectedCount: active.filter((clip) => clip.is_protected).length,
        notedCount: active.filter((clip) => Boolean(clip.note?.trim())).length,
        typeCounts: countBy('content_type').map(([content_type, count]) => ({ content_type, count })),
        sourceCounts: countBy('source').map(([name, count]) => ({ name, count })),
      } as unknown as T;
    }
    case 'is_clipboard_paused':
      return false as unknown as T;
    case 'get_all_app_settings':
      return {} as unknown as T;
    case 'get_source_icons':
      return {} as unknown as T;
    case 'get_external_import_sources':
      return [
        { id: 'alfred', label: 'Alfred', description: 'Clipboard history from Alfred Powerpack', available: true, detected: true, defaultPath: '/mock/Alfred/clipboard.alfdb', supportsCustomFile: true, selectionKind: 'file' },
        { id: 'pastebot', label: 'Pastebot', description: 'Text history from Pastebot', available: false, detected: true, defaultPath: '/mock/Pastebot.sqlite', supportsCustomFile: true, selectionKind: 'folder' },
        { id: 'pasta', label: 'Pasta', description: 'Text history from Pasta', available: false, detected: false, defaultPath: '/mock/Pasta/pasta.sqlite', supportsCustomFile: true, selectionKind: 'file' },
        { id: 'paste', label: 'Paste', description: 'Text history from Paste', available: false, detected: false, defaultPath: '/mock/Paste/Paste.sqlite', supportsCustomFile: true, selectionKind: 'folder' },
        { id: 'copyclip', label: 'CopyClip 2', description: 'Text history from CopyClip 2', available: false, detected: false, defaultPath: '/mock/CopyClip.data', supportsCustomFile: true, selectionKind: 'file' },
        { id: 'maccy', label: 'Maccy', description: 'Text history from Maccy', available: false, detected: false, defaultPath: '/mock/Maccy/Storage.sqlite', supportsCustomFile: true, selectionKind: 'file' },
        { id: 'flycut', label: 'Flycut', description: 'Text history from Flycut', available: false, detected: false, defaultPath: '/mock/Flycut.plist', supportsCustomFile: true, selectionKind: 'file' },
      ] as unknown as T;
    case 'import_external_history':
      return {
        source: String(args?.source ?? 'pastebot'),
        scannedCount: 42,
        importedCount: 38,
        duplicateCount: 3,
        skippedCount: 1,
        historyCapacityAdjustedTo: 1200,
      } as unknown as T;
    case 'export_backup_file':
      return `/mock/Pasted_Library_Archive_${new Date().toISOString().slice(0, 10)}.json` as unknown as T;
    case 'inspect_library_archive_json': {
      const parsed = JSON.parse(String(args?.jsonStr ?? '{}')) as Record<string, unknown[]>;
      return {
        schemaVersion: Number((parsed as { version?: number }).version ?? 1),
        clipCount: parsed.clips?.length ?? 0,
        binCount: parsed.bins?.length ?? 0,
        operationCount: parsed.operations?.length ?? 0,
        transformCount: (parsed.saved_transforms?.length ?? 0) + (parsed.pipelines?.length ?? 0),
        detectorCount: parsed.content_detectors?.length ?? 0,
        contentTypeCount: parsed.content_types?.length ?? 0,
      } as unknown as T;
    }
    case 'choose_import_file':
      return {
        path: '/mock/Pasted_History_and_Organization.json',
        name: 'Pasted_History_and_Organization.json',
        kind: 'organization',
        format: 'json',
        sizeBytes: 184_320,
        library: {
          schemaVersion: 1,
          clipCount: 248,
          binCount: 7,
          operationCount: 5,
          transformCount: 12,
          detectorCount: 4,
          contentTypeCount: 9,
        },
      } as unknown as T;
    case 'import_inspected_file':
      return { importedCount: 248, duplicateCount: 0 } as unknown as T;
    case 'export_full_backup_file':
      return {
        path: `/mock/Pasted_Full_Backup_${new Date().toISOString().slice(0, 10)}.pastedbackup`,
        createdAt: new Date().toISOString(),
        sizeBytes: 2_457_600,
      } as unknown as T;
    case 'restore_full_backup_file':
      return {
        recoveryPath: '/mock/Pasted_Pre_Restore.pastedbackup',
        backupCreatedAt: new Date().toISOString(),
      } as unknown as T;
    case 'consume_pending_full_restore_client_state':
      return null as unknown as T;
    case 'get_library_location':
      return mockLibraryLocation as unknown as T;
    case 'move_library':
      mockLibraryLocation = {
        path: '/mock/Custom Pasted Library/pasted.db',
        directory: '/mock/Custom Pasted Library',
        isDefault: false,
      };
      return { location: mockLibraryLocation, recoveryPath: '/mock/Pasted/pasted.db' } as unknown as T;
    case 'restore_default_library_location':
      mockLibraryLocation = {
        path: '/mock/Pasted/pasted.db',
        directory: '/mock/Pasted',
        isDefault: true,
      };
      return { location: mockLibraryLocation, recoveryPath: '/mock/Custom Pasted Library/pasted.db' } as unknown as T;
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
        { id: 1, name: 'Screenshots', icon: '📸', color: '#ec4899', smart_rule: '{"type":"origin_kind","value":"screenshot"}', bin_type: 'category' },
        { id: 2, name: 'Links and web', icon: 'Link', color: '#3b82f6', smart_rule: '{"type":"content_type","value":"link"}', bin_type: 'category' },
        { id: 3, name: 'Code Snippets', icon: 'Code', color: '#10b981', smart_rule: '{"type":"content_type","value":"code"}', bin_type: 'category' },
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
    case 'export_activity_json':
      return JSON.stringify({ schemaVersion: 1, exportedAt: new Date().toISOString(), resource: { 'service.name': 'Pasted' }, entries: [] }, null, 2) as unknown as T;
    case 'export_activity_csv':
      return 'timestamp,observed_timestamp,event_name,severity_text,body,category,outcome,attributes_json\n' as unknown as T;
    case 'import_activity_json':
    case 'import_activity_csv':
      return { scannedCount: 0, importedCount: 0, duplicateCount: 0, retainedCount: 0 } as unknown as T;
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
    case 'restore_all_trashed_clips': {
      const restoredIds = mockClips.filter((clip) => clip.is_trashed !== 0).map((clip) => clip.id);
      for (const clip of mockClips) {
        if (clip.is_trashed !== 0) {
          clip.is_trashed = 0;
          clip.trashed_at = null;
        }
      }
      return {
        action: 'restore_all',
        requestedCount: restoredIds.length,
        changedCount: restoredIds.length,
        skippedCount: 0,
        clipIds: restoredIds,
      } as unknown as T;
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
