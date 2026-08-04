import type {
  TransformExecutionDestination,
  TransformExecutionTrigger,
  TransformationExecutionOutcome,
} from '../types';
import { safeInvoke as invoke } from './tauri';

export type TransformationExecutionTarget =
  | { kind: 'transform'; transformRef: string }
  | { kind: 'operation'; operationRef: string }
  | { kind: 'pipeline'; pipelineRef: string };

interface RunTransformationOptions {
  sourceClipId?: number | null;
  trigger?: TransformExecutionTrigger;
  destination?: TransformExecutionDestination;
}

export interface TransformationExecutionHandle {
  clientRequestId: string;
  promise: Promise<TransformationExecutionOutcome>;
  cancel: () => Promise<boolean>;
}

function createRequestId() {
  return globalThis.crypto?.randomUUID?.()
    ?? `transform-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

export function startTransformation(
  input: string,
  target: TransformationExecutionTarget,
  options: RunTransformationOptions = {},
): TransformationExecutionHandle {
  const clientRequestId = createRequestId();
  const promise = invoke<TransformationExecutionOutcome>('execute_transformation', {
    request: {
      input,
      target,
      sourceClipId: options.sourceClipId ?? null,
      trigger: options.trigger ?? 'manual',
      destination: options.destination ?? 'preview',
      clientRequestId,
    },
  });
  return {
    clientRequestId,
    promise,
    cancel: () => invoke<boolean>('cancel_transformation_execution', { clientRequestId }),
  };
}

export function runTransformation(
  input: string,
  target: TransformationExecutionTarget,
  options: RunTransformationOptions = {},
) {
  return startTransformation(input, target, options).promise;
}
