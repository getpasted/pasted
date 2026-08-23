import { Minus, Plus } from 'lucide-react';

import type { BinModalFormController } from '../hooks/useBinModalForm';
import { translate } from '../localization/runtime';
import type {
  SmartConditionRow,
  SmartConditionTarget,
  SmartTargetSection,
} from './binModalModel';
import {
  SmartConditionTargetSelect,
  SmartConditionValueInput,
} from './BinModalSmartConditionInputs';
import { MenuSelect } from './MenuSelect';

interface BinModalSmartRulesProps {
  form: BinModalFormController;
  targetLabels: Record<SmartConditionTarget, string>;
  targetSectionsFor: (condition: SmartConditionRow) => SmartTargetSection[];
}

export function BinModalSmartRules({
  form,
  targetLabels,
  targetSectionsFor,
}: BinModalSmartRulesProps) {
  const {
    conditions,
    matchCondition,
    setMatchCondition,
    addCondition,
    removeCondition,
    updateCondition,
  } = form;

  return (
    <div className="flex items-start gap-3">
      <span className="w-20 shrink-0 pt-0.5 text-end text-xs font-semibold theme-text-muted">
        {translate('component.binModal.filter')}
      </span>
      <div className="min-w-0 flex-1 space-y-2">
        <div className="p-4 theme-surface rounded-2xl border space-y-3">
          {conditions.map((condition) => (
            <div key={condition.id} className="flex items-center space-x-2">
              <SmartConditionTargetSelect
                condition={condition}
                sections={targetSectionsFor(condition)}
                onSelect={(target, value) => updateCondition(condition.id, {
                  target,
                  value,
                  operator: target === 'contains' || target === 'file_extension' || target === 'file_path'
                    ? 'contains'
                    : 'is',
                })}
              />
              <MenuSelect
                value={condition.operator}
                onChange={(value) => updateCondition(condition.id, {
                  operator: value as SmartConditionRow['operator'],
                })}
                options={[
                  { value: 'is', get label() { return translate('component.binModal.is'); } },
                  { value: 'contains', get label() { return translate('component.pipelineEditorModal.contains'); } },
                ]}
                label={translate('component.binModal.conditionOperator')}
                className="w-24"
                compact
              />
              <BinModalSmartConditionValue
                condition={condition}
                targetLabels={targetLabels}
                targetSectionsFor={targetSectionsFor}
                onChange={(value) => updateCondition(condition.id, { value })}
              />
              <div className="flex items-center space-x-1">
                <button
                  type="button"
                  onClick={() => removeCondition(condition.id)}
                  disabled={conditions.length <= 1}
                  className={`theme-icon-button p-1.5 rounded border transition-[background-color,border-color,color,transform] ${conditions.length <= 1 ? 'opacity-40 cursor-not-allowed' : 'hover:scale-105 active:scale-95'}`}
                  title={translate('component.binModal.removeCondition')}
                >
                  <Minus className="w-3.5 h-3.5" />
                </button>
                <button
                  type="button"
                  onClick={addCondition}
                  className="theme-icon-button p-1.5 rounded border transition-[background-color,border-color,color,transform] hover:scale-105 active:scale-95"
                  title={translate('component.binModal.addCondition')}
                >
                  <Plus className="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
          ))}
          <div className="flex items-center space-x-2 pt-1 theme-text-muted">
            <span>{translate('component.binModal.containClipsThatMatch')}</span>
            <MenuSelect
              value={matchCondition}
              onChange={(value) => setMatchCondition(value as 'any' | 'all')}
              options={[
                { value: 'any', get label() { return translate('component.binModal.any'); } },
                { value: 'all', get label() { return translate('component.binModal.all'); } },
              ]}
              label={translate('component.binModal.conditionMatching')}
              className="w-24"
              compact
            />
            <span>{translate('component.binModal.conditions')}</span>
          </div>
        </div>
        <p className="text-[10px] theme-text-muted">
          {translate('component.binModal.chooseWhichClipsAutomaticallyEnterThisSmartBin')}
        </p>
      </div>
    </div>
  );
}

function BinModalSmartConditionValue({
  condition,
  targetLabels,
  targetSectionsFor,
  onChange,
}: {
  condition: SmartConditionRow;
  targetLabels: Record<SmartConditionTarget, string>;
  targetSectionsFor: (condition: SmartConditionRow) => SmartTargetSection[];
  onChange: (value: string) => void;
}) {
  if (condition.target === 'clip_type') {
    return <MenuSelect
      value={condition.value}
      onChange={onChange}
      options={[
        { value: 'text', get label() { return translate('component.analyticsView.text'); } },
        { value: 'image', get label() { return translate('component.analyticsView.image'); } },
        { value: 'file', get label() { return translate('component.analyticsView.files'); } },
      ]}
      label={translate('component.binModal.clipType')}
      className="min-w-0 flex-1"
      compact
    />;
  }
  if (condition.target === 'file_format' || condition.target === 'source' || condition.target === 'content_type') {
    return <SmartConditionValueInput
      label={targetLabels[condition.target]}
      value={condition.value}
      choices={targetSectionsFor(condition).find((section) => section.target === condition.target)?.choices ?? []}
      onChange={onChange}
    />;
  }
  if (condition.target === 'origin_kind') {
    return <MenuSelect
      value={condition.value}
      onChange={onChange}
      options={[
        { value: 'clipboard_content', get label() { return translate('component.binModal.clipboardContent'); } },
        { value: 'file_reference', get label() { return translate('component.binModal.fileReference'); } },
        { value: 'screenshot', get label() { return translate('component.binModal.screenshot'); } },
        { value: 'command_line', get label() { return translate('component.binModal.commandLine'); } },
      ]}
      label={translate('component.binModal.captureMethod')}
      className="min-w-0 flex-1"
      compact
    />;
  }
  const placeholder = condition.target === 'file_extension'
    ? translate('component.binModal.eGPdfZipPng')
    : condition.target === 'file_path'
      ? translate('component.binModal.eGProjectsOrDownloads')
      : translate('component.binModal.eGHttpFunction');
  return <input
    type="text"
    placeholder={placeholder}
    value={condition.value}
    onChange={(event) => onChange(
      condition.target === 'file_extension'
        ? event.target.value.replace(/^\./, '')
        : event.target.value,
    )}
    className="flex-1 theme-input form-field-valid border rounded-lg px-3 py-1.5 text-xs font-semibold focus:outline-none"
  />;
}
