import type {
  ExecutePlanOutcome,
  IntentPlanningMode,
  PlanIntentOutcome,
  TransformationPlan,
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

export interface CancellableTransformRequest<T> {
  clientRequestId: string;
  promise: Promise<T>;
  cancel: () => Promise<boolean>;
}

function createRequestId() {
  return globalThis.crypto?.randomUUID?.()
    ?? `transform-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

function cancelTransformRequest(clientRequestId: string) {
  return invoke<boolean>('cancel_transformation_execution', { clientRequestId });
}

export function startTransformation(
  input: string,
  target: TransformationExecutionTarget,
  options: RunTransformationOptions = {},
): TransformationExecutionHandle {
  const clientRequestId = createRequestId();
  return {
    clientRequestId,
    promise: invoke<TransformationExecutionOutcome>('execute_transformation', {
      request: {
        input,
        target,
        sourceClipId: options.sourceClipId ?? null,
        trigger: options.trigger ?? 'manual',
        destination: options.destination ?? 'preview',
        clientRequestId,
      },
    }),
    cancel: () => cancelTransformRequest(clientRequestId),
  };
}

export function runTransformation(
  input: string,
  target: TransformationExecutionTarget,
  options: RunTransformationOptions = {},
) {
  return startTransformation(input, target, options).promise;
}

export function startTransformDraft(request: {
  intent: string;
  sampleInput?: string | null;
  planningMode: IntentPlanningMode;
  connectionId?: string | null;
}) {
  const clientRequestId = createRequestId();
  return {
    clientRequestId,
    promise: invoke<PlanIntentOutcome>('plan_transformation_intent', { request, clientRequestId }),
    cancel: () => cancelTransformRequest(clientRequestId),
  };
}

export function startTransformTest(request: {
  plan: TransformationPlan;
  input: string;
  connectionId?: string | null;
}) {
  const clientRequestId = createRequestId();
  return {
    clientRequestId,
    promise: invoke<ExecutePlanOutcome>('test_transformation_plan', { request, clientRequestId }),
    cancel: () => cancelTransformRequest(clientRequestId),
  };
}
