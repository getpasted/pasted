import { useEffect, useRef, useState } from 'react';
import { LoaderCircle, Play, Sparkles } from 'lucide-react';
import type { ExecutePlanOutcome, PlanIntentOutcome, SavedTransform } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import {
  startTransformDraft,
  startTransformTest,
  type CancellableTransformRequest,
} from '../utils/transformExecution';
import { ActionButton, SaveButtonContent } from './AppDialogLayout';

interface IntentTransformComposerProps {
  sampleInput: string;
  onTestResult: (result: ExecutePlanOutcome) => void;
  onTransformSaved: (transform: SavedTransform) => void;
  onCancel: () => void;
  initialTransform?: SavedTransform | null;
  onDirtyChange?: (isDirty: boolean) => void;
  embedded?: boolean;
}

function errorMessage(reason: unknown) {
  if (reason && typeof reason === 'object' && 'message' in reason) return String(reason.message);
  return String(reason);
}

export function IntentTransformComposer({ sampleInput, onTestResult, onTransformSaved, onCancel, initialTransform, onDirtyChange, embedded = false }: IntentTransformComposerProps) {
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
    <section className={`${embedded ? '' : 'theme-surface border rounded-2xl p-5'} @container space-y-4`}>
      <div className="flex items-start gap-3">
        <span className="theme-badge border rounded-xl p-2.5 shrink-0"><Sparkles className="w-5 h-5" /></span>
        <div className="min-w-0 flex-1">
          <label htmlFor="transformation-intent" className="theme-title text-xs font-bold">What should happen?</label>
          <p className="theme-text-muted text-[10px] mt-1">Describe the result. Pasted will draft the implementation.</p>
        </div>
      </div>
      <div className="flex flex-col items-stretch gap-2 @md:flex-row">
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
          className="theme-input ui-field-radius border px-3 py-2.5 min-h-20 flex-1 resize-y text-xs"
        />
        <ActionButton
          variant="primary"
          disabled={!intent.trim() && !isPlanning}
          onClick={isPlanning ? cancelPlanning : buildTransform}
          className="min-h-10 min-w-32 px-4 disabled:opacity-40"
        >
          {isPlanning ? <LoaderCircle className="w-4 h-4 animate-spin" /> : <Sparkles className="w-4 h-4" />}
          <span>{isPlanning ? 'Cancel draft' : 'Build draft'}</span>
        </ActionButton>
      </div>

      {error && <div className="theme-status-danger border rounded-xl px-3 py-2 text-xs">{error}</div>}

      {outcome && (
        <section className="theme-surface overflow-hidden rounded-xl border">
          <div className="theme-divider flex flex-wrap items-end justify-between gap-3 border-b p-3">
            <div className="min-w-[min(14rem,100%)] flex-1">
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
          </div>
          <ol className="theme-subtle-surface space-y-1 p-1.5">
            {outcome.plan.steps.map((step, index) => (
              <li key={`${step.name}-${index}`} className="theme-card-idle flex items-start gap-2.5 border p-2">
                <span className="theme-text-subtle grid h-5 w-5 shrink-0 place-items-center rounded-full border text-[9px] font-bold">{index + 1}</span>
                <div className="min-w-0 flex-1">
                  <div className="theme-text-main text-xs font-semibold">{step.name}</div>
                  <div className="theme-text-muted text-[10px] mt-0.5">{step.rationale}</div>
                </div>
                <span className="theme-text-subtle shrink-0 text-[9px]">
                  {step.executor.kind === 'deterministic' ? 'Replayable' : 'AI'}
                </span>
              </li>
            ))}
          </ol>
          <div className="theme-divider flex flex-wrap items-center justify-between gap-2 border-t p-3">
            <ActionButton
              onClick={isTesting ? cancelTesting : () => void testDraft()}
              disabled={!sampleInput && !isTesting}
              title={!sampleInput ? 'Add Playground Input First' : 'Test Draft'}
            >
              {isTesting ? <LoaderCircle className="w-3.5 h-3.5 animate-spin" /> : <Play className="w-3.5 h-3.5" />}
              <span>{isTesting ? 'Cancel test' : 'Test draft'}</span>
            </ActionButton>
            <div className="flex items-center gap-2">
              <ActionButton onClick={onCancel}>Cancel</ActionButton>
              <ActionButton
                variant="primary"
                onClick={() => void saveTransform()}
                disabled={isSaving || Boolean(savedTransformRef)}
              >
                <SaveButtonContent isSaving={isSaving} isSaved={Boolean(savedTransformRef)} />
              </ActionButton>
            </div>
          </div>
        </section>
      )}
    </section>
  );
}
