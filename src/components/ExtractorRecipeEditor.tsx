import type { Dispatch, SetStateAction } from 'react';
import { FolderOpen, Plus, ScanText, Trash2 } from 'lucide-react';

import { translate } from '../localization/runtime';
import { AppDialogButton } from './AppDialogLayout';
import { ExtractorPostProcessingEditor } from './ExtractorPostProcessingEditor';
import { MenuMultiSelect } from './MenuMultiSelect';
import { MenuSelect } from './MenuSelect';
import { SettingsSwitch } from './SettingsSwitch';
import {
  emptyRecipe,
  EXTRACTOR_INPUT_OPTIONS,
  EXTRACTOR_OUTPUT_OPTIONS,
  type ExtractorCapture,
  type ExtractorInputKind,
  type ExtractorRecipe,
  type ExtractorTestOutcome,
} from './contentExtractorModel';
import { EXTRACTOR_FILE_FORMAT_GROUPS, EXTRACTOR_FILE_FORMAT_OPTIONS } from './extractorFileFormats';

export function ExtractorRecipeEditor({
  recipe,
  onChange,
  onChooseExecutable,
  onChooseResource,
  onTest,
  testing,
  testOutcome,
  canSave,
}: {
  recipe: ExtractorRecipe;
  onChange: Dispatch<SetStateAction<ExtractorRecipe>>;
  onChooseExecutable: (index: number) => void | Promise<void>;
  onChooseResource: (index: number) => void | Promise<void>;
  onTest: () => void | Promise<void>;
  testing: boolean;
  testOutcome: ExtractorTestOutcome | null;
  canSave: boolean;
}) {
  const updateStep = (index: number, update: Partial<ExtractorRecipe['steps'][number]>) => {
    onChange((current) => ({
      ...current,
      steps: current.steps.map((step, stepIndex) => stepIndex === index
        ? { ...step, ...update }
        : step),
    }));
  };

  const updateStepExecutable = (index: number, update: Partial<ExtractorRecipe['steps'][number]['executable']>) => {
    const step = recipe.steps[index];
    if (!step) return;
    updateStep(index, { executable: { ...step.executable, ...update } });
  };

  const updateResource = (index: number, update: Partial<ExtractorRecipe['resources'][number]>) => {
    onChange((current) => ({
      ...current,
      resources: current.resources.map((resource, resourceIndex) => resourceIndex === index
        ? { ...resource, ...update }
        : resource),
    }));
  };

  const resourceRequiredLabel = translate('component.contentExtractorManagerDialog.resourceRequired');
  const fileFormatOptions = [
    ...EXTRACTOR_FILE_FORMAT_OPTIONS,
    ...recipe.acceptedFileFormats.filter((format) => !EXTRACTOR_FILE_FORMAT_OPTIONS.includes(format)),
  ];
  const fileFormatGroup = (format: string) => {
    const group = EXTRACTOR_FILE_FORMAT_GROUPS.find(({ formats }) => formats.includes(format))?.id ?? 'other';
    return translate(`component.contentExtractorManagerDialog.fileFormatGroup.${group}`);
  };

  return <>
            <details className="theme-subtle-surface rounded-xl border p-3 text-[10px]">
              <summary className="theme-text-muted cursor-pointer font-semibold">{translate('component.contentExtractorManagerDialog.advanced')}</summary>
              <div className="mt-3 space-y-4">
                <div className="grid grid-cols-1 gap-3 @md:grid-cols-3">
                  <label className="space-y-1">
                    <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.acceptedClipTypes')}</span>
                    <MenuMultiSelect
                      values={recipe.accepts}
                      onChange={(values) => onChange({ ...recipe, accepts: values as ExtractorInputKind[] })}
                      label={translate('component.contentExtractorManagerDialog.acceptedClipTypes')}
                      placeholder={translate('component.contentExtractorManagerDialog.chooseInputs')}
                      className="w-full"
                      options={EXTRACTOR_INPUT_OPTIONS.filter((option) => !option.disabled).map((option) => ({ value: option.value, label: option.label }))}
                    />
                  </label>
                  <label className="space-y-1">
                    <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.acceptedFileFormats')}</span>
                    <MenuMultiSelect
                      values={recipe.acceptedFileFormats}
                      onChange={(values) => {
                        const addedAny = values.includes('*') && !recipe.acceptedFileFormats.includes('*');
                        onChange({
                          ...recipe,
                          acceptedFileFormats: addedAny ? ['*'] : values.filter((value) => value !== '*'),
                        });
                      }}
                      label={translate('component.contentExtractorManagerDialog.acceptedFileFormats')}
                      placeholder={translate('component.contentExtractorManagerDialog.chooseFileFormats')}
                      groupToggleLabel={translate('common.all')}
                      className="w-full"
                      disabled={!recipe.accepts.includes('file_references')}
                      options={fileFormatOptions.map((format) => ({
                        value: format,
                        group: fileFormatGroup(format),
                        label: format === '*'
                          ? translate('component.contentExtractorManagerDialog.anyFileFormat')
                          : format.toUpperCase(),
                      }))}
                    />
                  </label>
                  <label className="space-y-1">
                    <span className="theme-text-muted block font-semibold">{translate('common.output')}</span>
                    <MenuSelect
                      value={recipe.output}
                      onChange={() => undefined}
                      label={translate('common.output')}
                      className="w-full"
                      options={EXTRACTOR_OUTPUT_OPTIONS.map((option) => ({ value: option.value, label: option.label }))}
                    />
                  </label>
                </div>
                <ExtractorPostProcessingEditor recipe={recipe} onChange={onChange} />
                <div className="space-y-2">
                  <div className="flex items-center justify-between gap-2">
                    <span className="theme-text-muted font-semibold">{translate('component.contentExtractorManagerDialog.commands')}</span>
                    <AppDialogButton type="button" onClick={() => onChange((current) => ({ ...current, steps: [...current.steps, { ...emptyRecipe().steps[0], id: `step-${current.steps.length + 1}` }] }))}>
                      <Plus className="h-3.5 w-3.5" /> {translate('common.new')}
                    </AppDialogButton>
                  </div>
                  {recipe.steps.map((step, index) => <div key={`${step.id}-${index}`} className="theme-surface space-y-3 rounded-lg border p-3">
                    <div className="grid grid-cols-1 gap-2 @md:grid-cols-[minmax(0,1fr)_120px_auto]">
                      <label className="space-y-1">
                        <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.stepId')}</span>
                        <input value={step.id} onChange={(event) => updateStep(index, { id: event.target.value })} className="theme-input ui-field-radius w-full border px-2.5 py-2 font-mono" />
                      </label>
                      <label className="space-y-1">
                        <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.timeLimit')}</span>
                        <input type="number" min={1} max={600} value={step.timeoutSeconds} onChange={(event) => updateStep(index, { timeoutSeconds: Number(event.target.value) || 1 })} className="theme-input ui-field-radius w-full border px-2.5 py-2 font-mono" />
                      </label>
                      <AppDialogButton variant="danger" className="self-end" onClick={() => onChange((current) => ({ ...current, steps: current.steps.filter((_, stepIndex) => stepIndex !== index) }))} disabled={recipe.steps.length === 1} title={translate('component.contentExtractorManagerDialog.removeCommand')}>
                        <Trash2 className="h-3.5 w-3.5" />
                      </AppDialogButton>
                    </div>
                    <label className="space-y-1">
                      <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.executable')}</span>
                      <span className="flex gap-2">
                        <input value={step.executable.path ?? ''} onChange={(event) => updateStepExecutable(index, { path: event.target.value || null })} placeholder={translate('component.contentExtractorManagerDialog.pathToExecutable')} className="theme-input ui-field-radius min-w-0 flex-1 border px-2.5 py-2 font-mono" />
                        <AppDialogButton type="button" onClick={() => void onChooseExecutable(index)} title={translate('component.contentExtractorManagerDialog.chooseALocalExecutable')}>
                          <FolderOpen className="h-3.5 w-3.5" /> {translate('component.contentExtractorManagerDialog.choose')}
                        </AppDialogButton>
                      </span>
                    </label>
                    <label className="space-y-1">
                      <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.discoveryCommands')}</span>
                      <input value={step.executable.discover.join(', ')} onChange={(event) => updateStepExecutable(index, { discover: event.target.value.split(',').map((value) => value.trim()).filter(Boolean) })} placeholder={['pdftotext', 'zbarimg'].join(', ')} className="theme-input ui-field-radius w-full border px-2.5 py-2 font-mono" />
                    </label>
                    <label className="space-y-1">
                      <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.versionArguments')}</span>
                      <input value={step.executable.versionArguments.join(' ')} onChange={(event) => updateStepExecutable(index, { versionArguments: event.target.value.split(/\s+/).map((value) => value.trim()).filter(Boolean) })} className="theme-input ui-field-radius w-full border px-2.5 py-2 font-mono" />
                    </label>
                    <label className="space-y-1">
                      <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.noOutputExitCodes')}</span>
                      <input value={step.noOutputExitCodes.join(', ')} onChange={(event) => updateStep(index, { noOutputExitCodes: event.target.value.split(',').map((value) => Number(value.trim())).filter((value) => Number.isInteger(value) && value > 0) })} placeholder="4" className="theme-input ui-field-radius w-full border px-2.5 py-2 font-mono" />
                    </label>
                    <label className="space-y-1">
                      <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.arguments')}</span>
                      <textarea dir="auto" value={step.arguments.join('\n')} onChange={(event) => updateStep(index, { arguments: event.target.value.split('\n') })} className="theme-input ui-field-radius min-h-20 w-full resize-y border px-2.5 py-2 font-mono" />
                    </label>
                    <div className="grid grid-cols-1 gap-2 @md:grid-cols-3">
                      <label className="space-y-1">
                        <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.run')}</span>
                        <MenuSelect value={step.mode} onChange={(value) => updateStep(index, { mode: value as 'once' | 'each_input' })} label={translate('component.contentExtractorManagerDialog.run')} className="w-full" options={[
                          { value: 'once', label: translate('component.contentExtractorManagerDialog.once') },
                          { value: 'each_input', label: translate('component.contentExtractorManagerDialog.forEachInput') },
                        ]} />
                      </label>
                      <label className="space-y-1">
                        <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.capture')}</span>
                        <MenuSelect value={step.capture} onChange={(value) => updateStep(index, { capture: value as ExtractorCapture })} label={translate('component.contentExtractorManagerDialog.capture')} className="w-full" options={[
                          { value: 'stdout_text', label: translate('component.contentExtractorManagerDialog.standardOutput') },
                          { value: 'file_text', label: translate('component.contentExtractorManagerDialog.outputFileText') },
                          { value: 'pasted_json_v1', label: translate('component.contentExtractorManagerDialog.pastedJson') },
                          { value: 'ignore', label: translate('component.contentExtractorManagerDialog.ignoreOutput') },
                        ]} />
                      </label>
                      <label className="space-y-1">
                        <span className="theme-text-muted block font-semibold">{translate('component.binModal.fileExtension')}</span>
                        <input value={step.outputExtension ?? ''} onChange={(event) => updateStep(index, { outputExtension: event.target.value.replace(/^\./, '') || null })} className="theme-input ui-field-radius w-full border px-2.5 py-2 font-mono" />
                      </label>
                    </div>
                  </div>)}
                </div>
                <div className="space-y-2">
                  <div className="flex items-center justify-between gap-2">
                    <span className="theme-text-muted font-semibold">{translate('component.contentExtractorManagerDialog.resources')}</span>
                    <AppDialogButton type="button" onClick={() => onChange((current) => ({ ...current, resources: [...current.resources, { id: `resource-${current.resources.length + 1}`, label: translate('component.contentExtractorManagerDialog.resource'), kind: 'file', required: true, path: null }] }))}>
                      <Plus className="h-3.5 w-3.5" /> {translate('common.new')}
                    </AppDialogButton>
                  </div>
                  {recipe.resources.length === 0 && <p className="theme-text-muted">{translate('component.contentExtractorManagerDialog.noAdditionalResourcesAreRequired')}</p>}
                  {recipe.resources.map((resource, index) => <div key={`${resource.id}-${index}`} className="theme-surface space-y-2 rounded-lg border p-3">
                    <div className="grid grid-cols-1 gap-2 @md:grid-cols-[minmax(0,1fr)_160px]">
                      <input value={resource.label} onChange={(event) => updateResource(index, { label: event.target.value })} aria-label={translate('component.contentExtractorManagerDialog.resourceName')} className="theme-input ui-field-radius border px-2.5 py-2" />
                      <input value={resource.id} onChange={(event) => updateResource(index, { id: event.target.value })} aria-label={translate('component.contentExtractorManagerDialog.resourceId')} className="theme-input ui-field-radius border px-2.5 py-2 font-mono" />
                    </div>
                    <div className="grid grid-cols-1 gap-2 @md:grid-cols-[minmax(0,1fr)_160px]">
                      <MenuSelect value={resource.kind} onChange={(value) => updateResource(index, { kind: value as 'file' | 'directory' })} label={translate('component.contentExtractorManagerDialog.resourceKind')} className="w-full" options={[
                        { value: 'file', label: translate('component.contentExtractorManagerDialog.resourceFile') },
                        { value: 'directory', label: translate('component.contentExtractorManagerDialog.resourceDirectory') },
                      ]} />
                      <span className="theme-text-muted flex items-center justify-between gap-2 font-semibold">
                        {resourceRequiredLabel}
                        <SettingsSwitch checked={resource.required} onClick={() => updateResource(index, { required: !resource.required })} label={resourceRequiredLabel} />
                      </span>
                    </div>
                    <span className="flex gap-2">
                      <input value={resource.path ?? ''} onChange={(event) => updateResource(index, { path: event.target.value || null })} aria-label={translate('component.contentExtractorManagerDialog.resourcePath')} className="theme-input ui-field-radius min-w-0 flex-1 border px-2.5 py-2 font-mono" />
                      <AppDialogButton type="button" onClick={() => void onChooseResource(index)} title={translate('component.contentExtractorManagerDialog.chooseResource')}><FolderOpen className="h-3.5 w-3.5" /> {translate('component.contentExtractorManagerDialog.choose')}</AppDialogButton>
                      <AppDialogButton variant="danger" onClick={() => onChange((current) => ({ ...current, resources: current.resources.filter((_, resourceIndex) => resourceIndex !== index) }))} title={translate('component.contentExtractorManagerDialog.removeResource')}><Trash2 className="h-3.5 w-3.5" /></AppDialogButton>
                    </span>
                  </div>)}
                </div>
                <div className="theme-divider flex flex-wrap items-start justify-between gap-3 border-t pt-3">
                  <div className="min-w-0 flex-1">
                    {testOutcome?.outcome === 'produced' && <textarea dir="auto" readOnly value={testOutcome.text} aria-label={translate('component.contentExtractorManagerDialog.testOutput')} className="theme-input ui-field-radius min-h-20 w-full resize-y border px-2.5 py-2" />}
                    {testOutcome?.outcome === 'no_output' && <p className="theme-text-muted">{translate('component.contentExtractorManagerDialog.testProducedNoText')}</p>}
                    {testOutcome?.outcome === 'failed' && <p className="theme-danger-text">{testOutcome.failure.message}</p>}
                  </div>
                  <AppDialogButton type="button" onClick={() => void onTest()} disabled={testing || !canSave}>
                    <ScanText className="h-3.5 w-3.5" /> {testing ? translate('component.contentExtractorManagerDialog.testing') : translate('component.contentExtractorManagerDialog.test')}
                  </AppDialogButton>
                </div>
              </div>
            </details>
  </>;
}
