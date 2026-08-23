import React, { useEffect, useRef, useState } from 'react';
import { Plus, RotateCcw, Sliders } from 'lucide-react';
import type { ManualTransform, Operation } from '../types';
import { transformsApi } from '../api/transforms';
import { useFeatures } from '../hooks/useFeatures';
import { translate } from '../localization/runtime';
import { safeInvoke as invoke } from '../utils/tauri';
import { startManualTransformPreview, type CancellableTransformRequest } from '../utils/transformExecution';
import { handleWindowDragDoubleClick, startWindowDrag } from '../utils/windowDrag';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading, SaveButtonContent } from './AppDialogLayout';
import { HotkeyRecorder } from './HotkeyRecorder';
import { ManualTransformStepEditor } from './ManualTransformStepEditor';
import { compileManualTransformStep, createDefaultManualTransformStep, pipelineStepToEditorStep, type ManualTransformEditorStep } from './manualTransformStepModel';
import { PlaygroundRunStatus, type PlaygroundRunState } from './PlaygroundRunStatus';
import { RegistryPanelHeader } from './RegistryPanelHeader';
import { TransformationPreviewPanel } from './TransformationPreviewPanel';

interface ManualTransformEditorModalProps {
  manualTransform: ManualTransform | null;
  isOpen: boolean;
  onClose: () => void;
  onSaveSuccess: () => void;
}

const DEFAULT_TEST_INPUT = 'Hello there! :) https://example.com?utm_source=test';

export const ManualTransformEditorModal: React.FC<ManualTransformEditorModalProps> = ({
  manualTransform,
  isOpen,
  onClose,
  onSaveSuccess,
}) => {
  const features = useFeatures();
  const [transformName, setTransformName] = useState('');
  const [hotkey, setHotkey] = useState<string | null>(null);
  const [steps, setSteps] = useState<ManualTransformEditorStep[]>([]);
  const [testInput, setTestInput] = useState(DEFAULT_TEST_INPUT);
  const [testOutput, setTestOutput] = useState('');
  const [testRunState, setTestRunState] = useState<PlaygroundRunState>('idle');
  const [testDurationMs, setTestDurationMs] = useState<number>();
  const [saveError, setSaveError] = useState('');
  const [isSaving, setIsSaving] = useState(false);
  const [operations, setOperations] = useState<Operation[]>([]);
  const testRequestIdRef = useRef(0);
  const activeTestExecutionRef = useRef<CancellableTransformRequest<string> | null>(null);
  const initialSnapshotRef = useRef('');

  useEffect(() => {
    if (!isOpen) return;
    invoke<Operation[]>('get_operations')
      .then(setOperations)
      .catch((error) => setSaveError(error instanceof Error ? error.message : String(error)));

    if (manualTransform) {
      const nextSteps = manualTransform.steps.map(pipelineStepToEditorStep);
      setTransformName(manualTransform.name);
      setHotkey(manualTransform.hotkey || null);
      setSteps(nextSteps);
      initialSnapshotRef.current = JSON.stringify({
        transformName: manualTransform.name,
        hotkey: manualTransform.hotkey || null,
        steps: nextSteps,
      });
    } else {
      const nextSteps = [createDefaultManualTransformStep()];
      setTransformName('');
      setHotkey(null);
      setSteps(nextSteps);
      initialSnapshotRef.current = JSON.stringify({ transformName: '', hotkey: null, steps: nextSteps });
    }
    setSaveError('');
  }, [isOpen, manualTransform]);

  const handleReset = () => {
    if (manualTransform) {
      setTransformName(manualTransform.name);
      setHotkey(manualTransform.hotkey || null);
      setSteps(manualTransform.steps.map(pipelineStepToEditorStep));
      return;
    }
    setTransformName('');
    setHotkey(null);
    setSteps([createDefaultManualTransformStep()]);
    setTestInput(DEFAULT_TEST_INPUT);
  };

  const runLiveTest = async () => {
    if (!testInput) {
      setTestOutput('');
      return;
    }
    const requestId = ++testRequestIdRef.current;
    const startedAt = performance.now();
    setTestRunState('running');
    setTestDurationMs(undefined);
    try {
      const execution = startManualTransformPreview(testInput, steps.map(compileManualTransformStep));
      activeTestExecutionRef.current = execution;
      const output = await execution.promise;
      if (requestId !== testRequestIdRef.current) return;
      setTestOutput(output);
      setTestRunState('success');
      setTestDurationMs(performance.now() - startedAt);
    } catch (error) {
      if (requestId !== testRequestIdRef.current) return;
      setTestOutput(translate('common.errorMessage', { error: String(error) }));
      setTestRunState(String(error).includes('execution_cancelled') ? 'cancelled' : 'error');
    } finally {
      if (requestId === testRequestIdRef.current) activeTestExecutionRef.current = null;
    }
  };

  useEffect(() => {
    if (!isOpen) return;
    const timer = window.setTimeout(() => void runLiveTest(), 350);
    return () => {
      window.clearTimeout(timer);
      void activeTestExecutionRef.current?.cancel();
      activeTestExecutionRef.current = null;
      testRequestIdRef.current += 1;
    };
  }, [steps, testInput, isOpen]);

  const cancelLiveTest = () => {
    void activeTestExecutionRef.current?.cancel();
    activeTestExecutionRef.current = null;
    testRequestIdRef.current += 1;
    setTestRunState('cancelled');
  };

  const handleMoveStep = (index: number, offset: -1 | 1) => {
    setSteps((current) => {
      const destination = index + offset;
      if (destination < 0 || destination >= current.length) return current;
      const next = [...current];
      [next[index], next[destination]] = [next[destination], next[index]];
      return next;
    });
  };

  const handleSave = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!transformName.trim()) return;
    setSaveError('');
    setIsSaving(true);
    try {
      const payload = { name: transformName.trim(), steps: steps.map(compileManualTransformStep), hotkey };
      if (manualTransform) await transformsApi.updateManual(manualTransform.stableRef, payload);
      else await transformsApi.createManual(payload);
      onSaveSuccess();
      onClose();
    } catch (error) {
      setSaveError(error instanceof Error ? error.message : String(error));
    } finally {
      setIsSaving(false);
    }
  };

  if (!isOpen) return null;
  const isDirty = JSON.stringify({ transformName, hotkey, steps }) !== initialSnapshotRef.current;

  return (
    <AppDialog isOpen={isOpen} onClose={onClose} labelledBy="manualTransform-editor-title" isDirty={isDirty} overlayClassName="p-6" panelClassName="theme-panel flex max-h-[90vh] w-full max-w-4xl flex-col overflow-hidden border">
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose} onMouseDown={startWindowDrag} onDoubleClick={handleWindowDragDoubleClick}>
          <AppDialogHeading id="manualTransform-editor-title" title={manualTransform ? translate('component.pipelineEditorModal.editTransform') : translate('component.pipelineEditorModal.buildTransformManually')} description={translate('component.pipelineEditorModal.chainReusableOperationsIntoALocalReplayableTransform')} icon={<Sliders />} tone="info" />
        </AppDialogHeader>

        <AppDialogBody className="relative space-y-6">
          <div className="grid grid-cols-1 items-end gap-4 md:grid-cols-3">
            <div className={features.hotkeys ? 'md:col-span-2' : 'md:col-span-3'}>
              <label className="mb-1 block text-xs font-semibold theme-text-muted">{translate('common.name')}</label>
              <input type="text" placeholder={translate('component.pipelineEditorModal.eGSanitizeHtmlAndConvertSmileys')} value={transformName} onChange={(event) => setTransformName(event.target.value)} className="theme-input ui-field-radius w-full border px-3 py-2 text-xs font-medium focus:outline-none" autoFocus />
            </div>
            {features.hotkeys && <div>
              <label className="mb-1 block text-xs font-semibold theme-text-muted">{translate('component.pipelineEditorModal.hotkey')}</label>
              <HotkeyRecorder value={hotkey} placeholder={translate('component.pipelineEditorModal.setHotkey')} onChange={setHotkey} />
            </div>}
          </div>

          <TransformationPreviewPanel
            description={translate('component.pipelineEditorModal.updatesAutomaticallyAsStepsChange')}
            status={<PlaygroundRunStatus state={testRunState} label={translate('component.pipelineEditorModal.preview')} durationMs={testDurationMs} onRetry={() => void runLiveTest()} onStop={cancelLiveTest} />}
            input={<textarea dir="auto" value={testInput} onChange={(event) => setTestInput(event.target.value)} className="theme-input ui-field-radius h-24 w-full border p-2.5 focus:outline-none" />}
            output={<div className="theme-input ui-field-radius overlay-scroll-region h-24 w-full overflow-y-auto whitespace-pre-wrap border p-2.5 font-mono">{testOutput || translate('component.pipelineEditorModal.transformedOutputWillAppearHere')}</div>}
          />

          <section className="theme-surface overflow-hidden rounded-xl border">
            <RegistryPanelHeader title={translate('component.pipelineEditorModal.stepCount', { count: steps.length })} actions={<AppDialogButton onClick={() => setSteps((current) => [...current, createDefaultManualTransformStep()])} className="h-7 min-h-7 px-2.5"><Plus className="h-3 w-3" /><span>{translate('component.pipelineEditorModal.addStep')}</span></AppDialogButton>} />
            <div className="theme-subtle-surface space-y-1 p-1.5">
              {steps.map((step, index) => <ManualTransformStepEditor
                key={step.id}
                step={step}
                index={index}
                totalSteps={steps.length}
                operations={operations}
                onRemove={() => setSteps((current) => current.length === 1 ? current : current.filter((item) => item.id !== step.id))}
                onUpdate={(updates) => setSteps((current) => current.map((item) => item.id === step.id ? { ...item, ...updates } : item))}
                onMoveUp={() => handleMoveStep(index, -1)}
                onMoveDown={() => handleMoveStep(index, 1)}
              />)}
            </div>
          </section>
          {saveError && <div role="alert" className="theme-status-danger rounded-xl border px-3 py-2 text-xs">{saveError}</div>}
        </AppDialogBody>

        <AppDialogFooter align="between">
          <AppDialogButton onClick={handleReset} title={translate('component.pipelineEditorModal.resetTransform')}><RotateCcw className="h-3.5 w-3.5" /><span>{translate('common.reset')}</span></AppDialogButton>
          <div className="flex items-center space-x-3">
            <AppDialogButton onClick={requestClose}>{translate('common.cancel')}</AppDialogButton>
            <AppDialogButton variant="primary" onClick={handleSave} disabled={isSaving}><SaveButtonContent isSaving={isSaving} /></AppDialogButton>
          </div>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
};
