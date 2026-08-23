import type { FC } from 'react';
import { ArrowDown, ArrowUp, Trash2 } from 'lucide-react';
import type { Operation } from '../types';
import { localizedBuiltinName } from '../localization/presentation';
import { translate } from '../localization/runtime';
import { MenuSelect, type MenuSelectOption } from './MenuSelect';
import { operationTypeForRef, type ManualTransformEditorStep } from './manualTransformStepModel';

const EXECUTOR_OPTIONS = [
  { value: 'regex', get label() { return translate('component.pipelineEditorModal.findAndReplaceRegexText'); }, category: 'Search' },
];

const OPERATION_CATEGORIES = [
  { key: 'Search', get label() { return translate('component.pipelineEditorModal.searchAndReplace'); }, registryCategory: null },
  { key: 'Cleaners', get label() { return translate('component.pipelineEditorModal.cleanersAndSanitizers'); }, registryCategory: 'Cleaners and sanitizers' },
  { key: 'Format', get label() { return translate('component.pipelineEditorModal.smartFormatting'); }, registryCategory: 'Smart Formatting' },
  { key: 'Case', get label() { return translate('component.pipelineEditorModal.caseTransformations'); }, registryCategory: 'Case Transformations' },
  { key: 'Extract', get label() { return translate('component.pipelineEditorModal.dataExtraction'); }, registryCategory: 'Data Extraction' },
  { key: 'Lines', get label() { return translate('component.pipelineEditorModal.lineOperations'); }, registryCategory: 'Line Operations' },
  { key: 'Structure', get label() { return translate('component.pipelineEditorModal.structureAndFormatting'); }, registryCategory: 'Structure and formatting' },
  { key: 'Encoding', get label() { return translate('component.pipelineEditorModal.encodingsAndDecodings'); }, registryCategory: 'Encodings and decodings' },
  { key: 'Advanced', get label() { return translate('component.pipelineEditorModal.advancedAndShellScripts'); }, registryCategory: null },
];

function operationOptions(operations: Operation[]): MenuSelectOption[] {
  const options = OPERATION_CATEGORIES.flatMap((category) => {
    const executors = EXECUTOR_OPTIONS
      .filter((option) => option.category === category.key)
      .map((option) => ({ value: `builtin:${option.value}`, label: option.label, group: category.label }));
    const builtIns = category.registryCategory
      ? operations
        .filter((operation) => operation.stable_id.startsWith('builtin:') && operation.category === category.registryCategory)
        .map((operation) => ({
          value: operation.stable_id,
          label: localizedBuiltinName('operation', operation.stable_id, operation.name, true),
          group: category.label,
        }))
      : [];
    return [...executors, ...builtIns];
  });
  options.push(...operations
    .filter((operation) => operation.stable_id.startsWith('custom:'))
    .map((operation) => ({
      value: operation.stable_id,
      label: operation.name,
      group: translate('component.pipelineEditorModal.customOperations'),
    })));
  return options;
}

interface ManualTransformStepEditorProps {
  step: ManualTransformEditorStep;
  index: number;
  totalSteps: number;
  operations: Operation[];
  onRemove: () => void;
  onUpdate: (updates: Partial<ManualTransformEditorStep>) => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
}

export const ManualTransformStepEditor: FC<ManualTransformStepEditorProps> = ({
  step,
  index,
  totalSteps,
  operations,
  onRemove,
  onUpdate,
  onMoveUp,
  onMoveDown,
}) => {
  const operationType = operationTypeForRef(step.operation_ref);
  const hasConfig = operationType === 'regex'
    || operationType === 'quote_text'
    || operationType === 'shell_script'
    || operationType === 'wrap_tags';

  return (
    <section className="theme-card-idle border p-2" aria-label={translate('component.pipelineEditorModal.transformStepValue', { value: index + 1 })}>
      <div className="flex flex-wrap items-center gap-2">
        <span className="theme-text-subtle grid h-5 w-5 shrink-0 place-items-center rounded-full border text-[9px] font-bold">{index + 1}</span>
        <MenuSelect
          value={step.operation_ref}
          options={operationOptions(operations)}
          onChange={(value) => onUpdate({ operation_ref: value })}
          label={translate('component.pipelineEditorModal.stepValueOperation', { value: index + 1 })}
          className="min-w-44 flex-1 font-sans"
          compact
          searchable
          searchPlaceholder={translate('component.pipelineEditorModal.searchOperations')}
        />
        <span className="flex shrink-0 items-center gap-1">
          <button type="button" onClick={onMoveUp} disabled={index === 0} className="theme-icon-button rounded-md border p-1.5 disabled:opacity-35" aria-label={translate('component.pipelineEditorModal.moveStepUp')} title={translate('component.pipelineEditorModal.moveStepUp')}><ArrowUp className="h-3.5 w-3.5" /></button>
          <button type="button" onClick={onMoveDown} disabled={index === totalSteps - 1} className="theme-icon-button rounded-md border p-1.5 disabled:opacity-35" aria-label={translate('component.pipelineEditorModal.moveStepDown')} title={translate('component.pipelineEditorModal.moveStepDown')}><ArrowDown className="h-3.5 w-3.5" /></button>
          {totalSteps > 1 && (
            <button type="button" onClick={onRemove} className="theme-icon-button theme-danger-text rounded-md border p-1.5" aria-label={translate('component.pipelineEditorModal.deleteStep')} title={translate('component.pipelineEditorModal.deleteStep')}>
              <Trash2 className="h-3.5 w-3.5" />
            </button>
          )}
        </span>
      </div>

      {hasConfig && <div className="theme-divider mt-2 grid grid-cols-1 gap-3 border-t pt-3 text-xs sm:grid-cols-2">
        {operationType === 'regex' && (
          <div className="space-y-2 sm:col-span-2">
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
              <div>
                <label className="mb-1 block theme-text-muted">{translate('component.pipelineEditorModal.find')}</label>
                <textarea dir="auto" placeholder={translate('component.pipelineEditorModal.textPatternOrRegexPattern')} value={step.findPattern || ''} onChange={(event) => onUpdate({ findPattern: event.target.value })} className="theme-input h-16 w-full rounded-lg border p-2 font-mono text-xs focus:outline-none" />
              </div>
              <div>
                <label className="mb-1 block theme-text-muted">{translate('component.pipelineEditorModal.replaceWith')}</label>
                <textarea dir="auto" placeholder={translate('component.pipelineEditorModal.replacementString')} value={step.replacePattern || ''} onChange={(event) => onUpdate({ replacePattern: event.target.value })} className="theme-input h-16 w-full rounded-lg border p-2 font-mono text-xs focus:outline-none" />
              </div>
            </div>
            <div className="flex flex-wrap items-center gap-3 pt-1">
              <div className="flex items-center space-x-1.5 text-xs theme-text-muted">
                <span>{translate('component.pipelineEditorModal.match')}</span>
                <MenuSelect value={step.matchMode || 'regex'} onChange={(value) => onUpdate({ matchMode: value as ManualTransformEditorStep['matchMode'] })} options={[
                  { value: 'literal', get label() { return translate('component.pipelineEditorModal.contains'); } },
                  { value: 'wildcard', get label() { return translate('component.pipelineEditorModal.wildcard'); } },
                  { value: 'regex', get label() { return translate('component.pipelineEditorModal.regularExpression'); } },
                ]} label={translate('component.pipelineEditorModal.matchMode')} className="w-40" compact />
              </div>
              <label className="flex cursor-pointer items-center space-x-1.5 text-xs theme-text-muted">
                <input type="checkbox" checked={step.caseSensitive || false} onChange={(event) => onUpdate({ caseSensitive: event.target.checked })} className="theme-checkbox rounded focus:ring-0" />
                <span>{translate('component.pipelineEditorModal.caseSensitive')}</span>
              </label>
            </div>
          </div>
        )}

        {operationType === 'quote_text' && (
          <div className="space-y-2 sm:col-span-2">
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
              <div>
                <label className="mb-1 block theme-text-muted">{translate('component.pipelineEditorModal.beforeContent')}</label>
                <textarea dir="auto" value={step.quoteBefore ?? '> '} onChange={(event) => onUpdate({ quoteBefore: event.target.value })} className="theme-input h-16 w-full rounded-lg border p-2 font-mono text-xs focus:outline-none" />
              </div>
              <div>
                <label className="mb-1 block theme-text-muted">{translate('component.pipelineEditorModal.afterContent')}</label>
                <textarea dir="auto" value={step.quoteAfter ?? ''} onChange={(event) => onUpdate({ quoteAfter: event.target.value })} className="theme-input h-16 w-full rounded-lg border p-2 font-mono text-xs focus:outline-none" />
              </div>
            </div>
            <label className="flex cursor-pointer items-center space-x-1.5 text-xs theme-text-muted">
              <input type="checkbox" checked={step.applyToEachLine ?? true} onChange={(event) => onUpdate({ applyToEachLine: event.target.checked })} className="theme-checkbox rounded focus:ring-0" />
              <span>{translate('component.pipelineEditorModal.applyToEachLine')}</span>
            </label>
          </div>
        )}

        {operationType === 'shell_script' && (
          <div className="sm:col-span-2">
            <label className="mb-1 block theme-text-muted">{translate('component.pipelineEditorModal.shellCommandStdinStdout')}</label>
            <input type="text" placeholder={translate('component.pipelineEditorModal.eGTrAZAZ')} value={step.shellCommand || ''} onChange={(event) => onUpdate({ shellCommand: event.target.value })} className="theme-input w-full rounded-lg border p-2 font-mono text-xs focus:outline-none" />
          </div>
        )}

        {operationType === 'wrap_tags' && (
          <div>
            <label className="mb-1 block theme-text-muted">{translate('component.pipelineEditorModal.htmlTagName')}</label>
            <input type="text" placeholder={translate('component.pipelineEditorModal.codeBBlockquote')} value={step.tagName || ''} onChange={(event) => onUpdate({ tagName: event.target.value })} className="theme-input w-full rounded-lg border p-2 font-mono text-xs focus:outline-none" />
          </div>
        )}
      </div>}
    </section>
  );
};
