import type { MockClip } from './models';
import { unhandledValue } from './result';

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

export const getMockConnectionCount = () => mockIntelligenceConnections.length;
export const getMockSavedTransforms = () => mockSavedTransforms;
export const hasMockBinTransform = (binId: number) => mockBinTransforms.has(binId);

export function resetMockIntelligence() {
  mockIntelligenceConnections = [];
  mockSavedTransforms = [];
  mockClipTransformations = new Map();
  mockBinTransforms = new Map();
}

export async function invokeIntelligenceBrowserMock<T>(
  cmd: string,
  args: Record<string, unknown> | undefined,
  mockClips: MockClip[],
): Promise<T | typeof unhandledValue> {
  switch (cmd) {
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
      const request = args?.request as { input?: string; target?: { kind?: string; transformRef?: string; operationRef?: string } } | undefined;
      const input = request?.input || '';
      const targetRef = request?.target?.transformRef || request?.target?.operationRef || '';
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
    case 'preview_manual_transform_steps': {
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
  }
  return unhandledValue;
}
