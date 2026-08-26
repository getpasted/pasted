import { Check, ClipboardPaste, Copy, Lightbulb, RotateCcw, Sliders } from 'lucide-react';

import type { IntelligenceRequestStatus } from '../hooks/useIntelligenceRequestStatus';
import { translate } from '../localization/runtime';
import type { ManualTransform, SavedTransform } from '../types';
import type { SmartActionSuggestion } from './clipPreviewModel';
import { ActionButton } from './AppDialogLayout';
import { ClipTransformBar } from './ClipTransformBar';
import { MenuSelect } from './MenuSelect';

export function ClipPreviewTransformControls({
  activeManualTransformRef,
  activeTransformName,
  activeTransformRef,
  canRunManualTransforms,
  canTransformContent,
  hasTransformPreview,
  isManualTransformRunning,
  manualTransforms,
  onApplyTransform,
  onManualTransformOutput,
  onPreviewManualTransform,
  onPreviewTransform,
  onResetTransform,
  onRetryTransform,
  pipelineAction,
  pipelineError,
  requestStatus,
  showSmartActions,
  smartActions,
  transformedText,
  transforms,
}: {
  activeManualTransformRef: string | null;
  activeTransformName: string | null;
  activeTransformRef: string | null;
  canRunManualTransforms: boolean;
  canTransformContent: boolean;
  hasTransformPreview: boolean;
  isManualTransformRunning: boolean;
  manualTransforms: ManualTransform[];
  onApplyTransform: () => void;
  onManualTransformOutput: (destination: 'copy' | 'paste') => void;
  onPreviewManualTransform: (transform: ManualTransform) => void;
  onPreviewTransform: (transform: SavedTransform) => void;
  onResetTransform: () => void;
  onRetryTransform: () => void;
  pipelineAction: 'copied' | 'pasted' | null;
  pipelineError: string | null;
  requestStatus?: IntelligenceRequestStatus;
  showSmartActions: boolean;
  smartActions: SmartActionSuggestion | null;
  transformedText: string | null;
  transforms: SavedTransform[];
}) {
  if (!canRunManualTransforms || !canTransformContent) return null;

  return <>
    {showSmartActions && smartActions && smartActions.result.actions.length > 0 && <div className="smart-actions-bar px-4 py-2 flex items-center justify-between text-xs space-x-2 overflow-x-auto">
      <div className="smart-actions-heading flex items-center space-x-1.5 shrink-0 font-semibold text-[11px]">
        <Lightbulb className="w-3.5 h-3.5" />
        <span>{translate('component.clipPreview.smartActionsSignals', { signals: smartActions.result.signalLabels.join(', ') })}</span>
      </div>
      <div className="flex items-center space-x-1.5 overflow-x-auto scrollbar-none py-0.5">
        {smartActions.result.actions.map((action) => {
          const transform = transforms.find((candidate) => candidate.stableRef === action.transformRef);
          const manualTransform = manualTransforms.find((candidate) => candidate.stableRef === action.transformRef);
          if (!transform && !manualTransform) return null;
          return <button
            key={action.transformRef}
            onClick={() => transform ? onPreviewTransform(transform) : onPreviewManualTransform(manualTransform!)}
            className="smart-action-button whitespace-nowrap"
            title={translate('component.clipPreview.previewTransformname', { transformName: action.transformName })}
          >
            <span>{action.transformName}</span>
          </button>;
        })}
      </div>
    </div>}

    {activeTransformRef && activeTransformName && <ClipTransformBar
      activeTransformName={activeTransformName}
      isRunning={isManualTransformRunning}
      hasPreview={hasTransformPreview}
      error={pipelineError}
      onApply={onApplyTransform}
      onRetry={onRetryTransform}
      onReset={onResetTransform}
      requestStatus={requestStatus}
    />}

    {manualTransforms.length > 0 && <div className="preview-filter-bar px-4 py-2.5 border-t select-none">
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center space-x-2 shrink-0">
          <Sliders className="preview-filter-accent w-4 h-4" />
          <span className="theme-text-main text-xs font-semibold">{translate('component.clipPreview.advancedTransform')}</span>
        </div>
        <div className="max-w-xs flex-1">
          <MenuSelect
            value={activeManualTransformRef || ''}
            onChange={(selectedRef) => {
              if (!selectedRef) onResetTransform();
              else {
                const found = manualTransforms.find((transform) => transform.stableRef === selectedRef);
                if (found) onPreviewManualTransform(found);
              }
            }}
            label={translate('component.clipPreview.chooseAdvancedTransform')}
            className={`w-full ${activeManualTransformRef ? 'preview-filter-select-active' : 'form-field-valid'}`}
            searchable
            searchPlaceholder={translate('component.clipPreview.searchManualTransforms')}
            options={[
              { value: '', get label() { return translate('component.clipPreview.originalClip'); } },
              ...manualTransforms.map((transform) => ({ value: transform.stableRef, label: transform.name })),
            ]}
          />
        </div>
        {activeManualTransformRef && <div className="flex items-center gap-1.5 shrink-0">
          <ActionButton
            onClick={() => onManualTransformOutput('copy')}
            disabled={isManualTransformRunning || transformedText === null}
            title={translate('component.clipPreview.copyResult')}
          >
            {pipelineAction === 'copied' ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
            <span>{pipelineAction === 'copied' ? translate('action.copied') : translate('action.copy')}</span>
          </ActionButton>
          <button
            type="button"
            onClick={() => onManualTransformOutput('paste')}
            disabled={isManualTransformRunning || transformedText === null}
            className="transform-workspace-action manual-transforms"
            title={translate('component.clipPreview.pasteResult')}
          >
            <ClipboardPaste className="h-3.5 w-3.5" />
            <span>{pipelineAction === 'pasted' ? translate('component.clipPreview.pasted') : translate('component.clipPreview.paste')}</span>
          </button>
          <ActionButton
            onClick={onResetTransform}
            title={translate('common.reset')}
          >
            <span>{translate('common.reset')}</span>
          </ActionButton>
        </div>}
      </div>
      {pipelineError && <div role="status" className="theme-status-error mt-2 flex items-center gap-2 rounded-lg border px-2.5 py-1.5 text-[11px]">
        <span className="min-w-0 flex-1">{pipelineError}</span>
        <button
          type="button"
          onClick={onRetryTransform}
          className="playground-run-status-action inline-flex shrink-0 items-center gap-1 rounded-md px-2 py-1 font-semibold"
        >
          <RotateCcw className="h-3 w-3" /> {translate('common.retry')}
        </button>
      </div>}
    </div>}
  </>;
}
