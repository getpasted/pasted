import { useEffect, useRef, useState } from 'react';
import { CheckCircle2, LoaderCircle, Play, Sparkles } from 'lucide-react';
import type { ExecutePlanOutcome, PlanIntentOutcome, SavedTransform } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import {
  startTransformDraft,
  startTransformTest,
  type CancellableTransformRequest,
} from '../utils/transformExecution';

interface IntentTransformComposerProps {
  sampleInput: string;
  onTestResult: (result: ExecutePlanOutcome) => void;
  onTransformSaved: (transform: SavedTransform) => void;
  initialTransform?: SavedTransform | null;
  onDirtyChange?: (isDirty: boolean) => void;
  embedded?: boolean;
}

function errorMessage(reason: unknown) {
  if (reason && typeof reason === 'object' && 'message' in reason) return String(reason.message);
  return String(reason);
}

export function IntentTransformComposer({ sampleInput, onTestResult, onTransformSaved, initialTransform, onDirtyChange, embedded = false }: IntentTransformComposerProps) {
  const isEditing = Boolean(initialTransform);
  const [intent, setIntent] = useState(initialTransform?.plan.intent ?? '');
  const [transformName, setTransformName] = useState(initialTransform?.name ?? '');
  const [outcome, setOutcome] = useState<PlanIntentOutcome | null>(() => initialTransform ? {
    plan: initialTransform.plan,
    connectionId: initialTransform.connectionId ?? '',
    connectionName: initialTransform.connectionId ? 'Saved connection' : 'Automatic connection',
    durationMs: 0,
  } : null);
  const [error, setError] = useState('');
  const [isPlanning, setIsPlanning] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [savedTransformRef, setSavedTransformRef] = useState('');
  const planningRequestRef = useRef<CancellableTransformRequest<PlanIntentOutcome> | null>(null);
  const testingRequestRef = useRef<CancellableTransformRequest<ExecutePlanOutcome> | null>(null);
  const planningRequestIdRef = useRef(0);
  const testingRequestIdRef = useRef(0);

  useEffect(() => () => {
    void planningRequestRef.current?.cancel();
    void testingRequestRef.current?.cancel();
  }, []);

  useEffect(() => {
    const isDirty = initialTransform
      ? intent !== initialTransform.plan.intent
        || transformName !== initialTransform.name
        || JSON.stringify(outcome?.plan ?? null) !== JSON.stringify(initialTransform.plan)
      : Boolean(intent.trim() || outcome);
    onDirtyChange?.(isDirty && !savedTransformRef);
  }, [initialTransform, intent, onDirtyChange, outcome, savedTransformRef, transformName]);

  const buildTransform = async () => {
    if (!intent.trim() || isPlanning) return;
    setIsPlanning(true);
    setError('');
    setSavedTransformRef('');
    const requestId = ++planningRequestIdRef.current;
    try {
      const request = startTransformDraft({
        intent: intent.trim(),
        sampleInput: sampleInput || null,
        planningMode: 'pinned',
        connectionId: null,
      });
      planningRequestRef.current = request;
      const result = await request.promise;
      if (requestId !== planningRequestIdRef.current) return;
      setOutcome(result);
      setTransformName((current) => current.trim() ? current : result.plan.summary);
    } catch (reason) {
      if (requestId !== planningRequestIdRef.current) return;
      setError(errorMessage(reason));
    } finally {
      if (requestId === planningRequestIdRef.current) {
        planningRequestRef.current = null;
        setIsPlanning(false);
      }
    }
  };

  const cancelPlanning = () => {
    void planningRequestRef.current?.cancel();
    planningRequestRef.current = null;
    planningRequestIdRef.current += 1;
    setIsPlanning(false);
  };

  const testDraft = async () => {
    if (!outcome || isTesting || !sampleInput) return;
    setIsTesting(true);
    setError('');
    const requestId = ++testingRequestIdRef.current;
    try {
      const request = startTransformTest({
        plan: outcome.plan,
        input: sampleInput,
        connectionId: outcome.connectionId,
      });
      testingRequestRef.current = request;
      const result = await request.promise;
      if (requestId !== testingRequestIdRef.current) return;
      onTestResult(result);
    } catch (reason) {
      if (requestId !== testingRequestIdRef.current) return;
      setError(errorMessage(reason));
    } finally {
      if (requestId === testingRequestIdRef.current) {
        testingRequestRef.current = null;
        setIsTesting(false);
      }
    }
  };

  const cancelTesting = () => {
    void testingRequestRef.current?.cancel();
    testingRequestRef.current = null;
    testingRequestIdRef.current += 1;
    setIsTesting(false);
  };

  const saveTransform = async () => {
    if (!outcome || isSaving || savedTransformRef) return;
    setIsSaving(true);
    setError('');
    try {
      const args = {
        ...(initialTransform ? { transformRef: initialTransform.stableRef } : {}),
        name: transformName.trim() || outcome.plan.summary,
        plan: outcome.plan,
        connectionId: outcome.connectionId || null,
      };
      const transform = isEditing
        ? await invoke<SavedTransform>('update_saved_transform', args)
        : await invoke<SavedTransform>('save_saved_transform', args);
      setSavedTransformRef(transform.stableRef);
      onTransformSaved(transform);
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <section className={`${embedded ? '' : 'theme-surface border rounded-2xl p-5'} space-y-4`}>
      <div className="flex items-start gap-3">
        <span className="theme-badge border rounded-xl p-2.5 shrink-0"><Sparkles className="w-5 h-5" /></span>
        <div className="min-w-0 flex-1">
          <label htmlFor="transformation-intent" className="theme-title text-sm font-bold">What should happen?</label>
          <p className="theme-text-muted text-[10px] mt-1">Describe the result. Pasted will draft the implementation.</p>
        </div>
      </div>
      <div className="flex items-stretch gap-2">
        <textarea
          id="transformation-intent"
          value={intent}
          onChange={(event) => {
            const nextIntent = event.target.value;
            setIntent(nextIntent);
            setSavedTransformRef('');
            if (outcome && nextIntent !== outcome.plan.intent) setOutcome(null);
          }}
          onKeyDown={(event) => {
            if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') void buildTransform();
          }}
          placeholder="For example: Make this concise and friendly, preserving every URL."
          className="theme-input border rounded-xl px-3 py-2.5 min-h-20 flex-1 resize-y text-xs"
        />
        <button
          type="button"
          disabled={!intent.trim() && !isPlanning}
          onClick={isPlanning ? cancelPlanning : buildTransform}
          className="theme-primary-button border rounded-xl px-4 min-w-32 text-xs font-semibold flex flex-col items-center justify-center gap-1.5 disabled:opacity-40"
        >
          {isPlanning ? <LoaderCircle className="w-4 h-4 animate-spin" /> : <Sparkles className="w-4 h-4" />}
          <span>{isPlanning ? 'Cancel draft' : 'Build draft'}</span>
        </button>
      </div>

      {error && <div className="theme-status-danger border rounded-xl px-3 py-2 text-xs">{error}</div>}

      {outcome && (
        <div className="theme-card-idle border rounded-xl p-4 space-y-3">
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0 flex-1">
              <label htmlFor="transform-name" className="theme-text-muted text-[10px] font-semibold">Name</label>
              <input
                id="transform-name"
                value={transformName}
                onChange={(event) => {
                  setTransformName(event.target.value);
                  setSavedTransformRef('');
                }}
                className="theme-input mt-1 w-full rounded-lg border px-2.5 py-1.5 text-xs font-semibold"
              />
              <div className="theme-text-muted mt-1 text-[10px]">
                {isEditing && outcome.durationMs === 0
                  ? `Revision ${initialTransform?.revision ?? 1} · ${outcome.connectionName}`
                  : `Drafted by ${outcome.connectionName} in ${(outcome.durationMs / 1000).toFixed(1)}s`}
              </div>
            </div>
            <span className="theme-status-success border rounded-full px-2 py-1 text-[9px] font-semibold flex items-center gap-1 shrink-0">
              <CheckCircle2 className="w-3 h-3" /> Draft ready
            </span>
          </div>
          <ol className="space-y-2">
            {outcome.plan.steps.map((step, index) => (
              <li key={`${step.name}-${index}`} className="theme-surface border rounded-lg px-3 py-2 flex items-start gap-2.5">
                <span className="theme-text-muted font-mono text-[10px] mt-0.5">{index + 1}</span>
                <div className="min-w-0">
                  <div className="theme-text-main text-xs font-semibold">{step.name}</div>
                  <div className="theme-text-muted text-[10px] mt-0.5">{step.rationale}</div>
                </div>
                <span className="theme-badge border rounded-full px-2 py-0.5 text-[9px] ml-auto shrink-0">
                  {step.executor.kind === 'deterministic' ? 'Replayable' : 'AI'}
                </span>
              </li>
            ))}
          </ol>
          <div className="flex justify-end gap-2 pt-1">
            <button
              type="button"
              onClick={isTesting ? cancelTesting : () => void testDraft()}
              disabled={!sampleInput && !isTesting}
              className="theme-secondary-button border rounded-lg px-3 h-8 text-[10px] font-semibold flex items-center gap-1.5 disabled:opacity-40"
              title={!sampleInput ? 'Add Playground Input First' : 'Test Draft'}
            >
              {isTesting ? <LoaderCircle className="w-3.5 h-3.5 animate-spin" /> : <Play className="w-3.5 h-3.5" />}
              <span>{isTesting ? 'Cancel test' : 'Test draft'}</span>
            </button>
            <button
              type="button"
              onClick={() => void saveTransform()}
              disabled={isSaving || Boolean(savedTransformRef)}
              className="theme-primary-button border rounded-lg px-3 h-8 text-[10px] font-semibold disabled:opacity-55"
            >
              {isSaving ? 'Saving…' : savedTransformRef ? 'Saved' : isEditing ? 'Update Transform' : 'Save Transform'}
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
