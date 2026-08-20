import { handled, unhandled, type BrowserMockResult } from './result';

export type MockManualTransform = {
  id: number;
  stableRef: string;
  name: string;
  hotkey: string | null;
  revision: number;
  createdAt: string;
  updatedAt: string;
  steps: Array<{
    position: number;
    operationRef: string;
    configJson: string | null;
    failurePolicy: string;
  }>;
};

export let mockManualTransforms: MockManualTransform[] = [
  {
    id: 1,
    stableRef: 'transform:mock-uppercase',
    name: 'Uppercase',
    hotkey: null,
    revision: 1,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    steps: [{ position: 0, operationRef: 'builtin:uppercase', configJson: null, failurePolicy: 'stop' }],
  },
  {
    id: 2,
    stableRef: 'transform:mock-clean-url',
    name: 'Clean URL',
    hotkey: null,
    revision: 1,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    steps: [{ position: 0, operationRef: 'builtin:clean_url_tracking', configJson: null, failurePolicy: 'stop' }],
  },
];

type MockOperation = {
  id: number;
  stable_id: string;
  name: string;
  op_type: string;
  config: string | null;
  category: string;
  created_at: string;
};

let operations: MockOperation[] = [];

export function handleManualTransformBrowserMock(
  command: string,
  args: Record<string, unknown> | undefined,
): BrowserMockResult {
  switch (command) {
    case 'get_manual_transforms':
      return handled(mockManualTransforms.map((transform) => ({ ...transform, steps: [...transform.steps] })));
    case 'get_operations':
      return handled(operations.map((operation) => ({ ...operation })));
    case 'create_operation': {
      const id = Math.max(0, ...operations.map((operation) => operation.id)) + 1;
      const operation = {
        id,
        stable_id: `custom:${id}`,
        name: String(args?.name ?? 'Custom Operation'),
        op_type: String(args?.opType ?? 'regex'),
        config: typeof args?.config === 'string' ? args.config : null,
        category: String(args?.category ?? 'Custom Operations'),
        created_at: new Date().toISOString(),
      };
      operations.push(operation);
      return handled(operation);
    }
    case 'update_operation': {
      const operation = operations.find(({ id }) => id === Number(args?.id));
      if (operation) {
        operation.name = String(args?.name ?? operation.name);
        operation.op_type = String(args?.opType ?? operation.op_type);
        operation.config = typeof args?.config === 'string' ? args.config : null;
        operation.category = String(args?.category ?? operation.category);
      }
      return handled(undefined);
    }
    case 'duplicate_operation': {
      const source = operations.find(({ stable_id }) => stable_id === String(args?.reference));
      if (!source) throw new Error('Operation was not found');
      const id = Math.max(0, ...operations.map((operation) => operation.id)) + 1;
      const duplicate = {
        ...source,
        id,
        stable_id: `custom:${id}`,
        name: String(args?.name ?? `${source.name} Copy`),
      };
      operations.push(duplicate);
      return handled(duplicate);
    }
    case 'delete_operation':
      operations = operations.filter(({ id }) => id !== Number(args?.id));
      return handled(undefined);
    case 'create_manual_transform': {
      const id = Math.max(0, ...mockManualTransforms.map((transform) => transform.id)) + 1;
      const now = new Date().toISOString();
      const transform: MockManualTransform = {
        id,
        stableRef: `transform:mock-${id}`,
        name: String(args?.name ?? 'Untitled Transform'),
        hotkey: typeof args?.hotkey === 'string' ? args.hotkey : null,
        revision: 1,
        createdAt: now,
        updatedAt: now,
        steps: Array.isArray(args?.steps) ? args.steps as MockManualTransform['steps'] : [],
      };
      mockManualTransforms.push(transform);
      return handled(transform);
    }
    case 'update_manual_transform': {
      const transform = mockManualTransforms.find(({ stableRef }) => stableRef === String(args?.transformRef));
      if (!transform) throw new Error('Transform was not found');
      transform.name = String(args?.name ?? transform.name);
      transform.steps = Array.isArray(args?.steps) ? args.steps as MockManualTransform['steps'] : transform.steps;
      transform.hotkey = typeof args?.hotkey === 'string' ? args.hotkey : null;
      transform.revision += 1;
      transform.updatedAt = new Date().toISOString();
      return handled(transform);
    }
    case 'update_manual_transform_hotkey': {
      const transform = mockManualTransforms.find(({ stableRef }) => stableRef === String(args?.transformRef));
      if (transform) transform.hotkey = typeof args?.hotkey === 'string' ? args.hotkey : null;
      return handled(undefined);
    }
    case 'delete_manual_transform':
      mockManualTransforms = mockManualTransforms.filter(({ stableRef }) => stableRef !== String(args?.transformRef));
      return handled(undefined);
    default:
      return unhandled;
  }
}
