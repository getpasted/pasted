import type { Dispatch, SetStateAction } from 'react';
import { Trash2 } from 'lucide-react';

import { translate } from '../localization/runtime';
import { AppDialogButton } from './AppDialogLayout';
import type { ExtractorRecipe } from './contentExtractorModel';
import {
  addLabelConfidencePostProcessing,
  replaceExtractorPostProcessing,
} from './extractorPostProcessingModel';
import { MenuSelect } from './MenuSelect';

export function ExtractorPostProcessingEditor({
  recipe,
  onChange,
}: {
  recipe: ExtractorRecipe;
  onChange: Dispatch<SetStateAction<ExtractorRecipe>>;
}) {
  const confidenceIndex = recipe.postProcessing.findIndex(
    ({ kind }) => kind === 'filter_labels_by_confidence',
  );
  const replacePostProcessing = (postProcessing: ExtractorRecipe['postProcessing']) => {
    onChange((current) => replaceExtractorPostProcessing(current, postProcessing));
  };

  return <div className="space-y-2">
    <div className="flex items-center justify-between gap-2">
      <span className="theme-text-muted font-semibold">
        {translate('component.contentExtractorManagerDialog.postProcessing')}
      </span>
      {confidenceIndex < 0 && <MenuSelect
        value=""
        label={translate('component.contentExtractorManagerDialog.addPostProcessingOperation')}
        className="w-48"
        onChange={(kind) => {
          if (kind === 'filter_labels_by_confidence') {
            onChange((current) => addLabelConfidencePostProcessing(current));
          }
        }}
        options={[
          { value: '', label: translate('component.contentExtractorManagerDialog.addPostProcessingOperation'), disabled: true },
          { value: 'filter_labels_by_confidence', label: translate('component.contentExtractorManagerDialog.labelConfidence') },
        ]}
      />}
    </div>
    {recipe.postProcessing.length === 0 && <p className="theme-text-muted">
      {translate('component.contentExtractorManagerDialog.noPostProcessing')}
    </p>}
    {recipe.postProcessing.map((operation, index) => <div
      key={`${operation.kind}-${index}`}
      className="theme-surface flex items-end gap-2 rounded-lg border p-3"
    >
      <label className="min-w-0 flex-1 space-y-1">
        <span className="theme-text-muted block font-semibold">
          {translate('component.contentExtractorManagerDialog.minimumLabelConfidencePercent')}
        </span>
        <span className="theme-input ui-field-radius flex items-center border">
          <input
            type="number"
            min={0}
            max={100}
            value={operation.minimumPercent}
            onChange={(event) => replacePostProcessing(recipe.postProcessing.map((candidate, candidateIndex) => (
              candidateIndex === index
                ? { ...candidate, minimumPercent: Number(event.target.value) || 0 }
                : candidate
            )))}
            className="min-w-0 flex-1 bg-transparent px-2.5 py-2 font-mono outline-none"
          />
          <span className="theme-text-muted pe-2.5">%</span>
        </span>
      </label>
      <AppDialogButton
        type="button"
        variant="danger"
        onClick={() => replacePostProcessing(recipe.postProcessing.filter((_, candidateIndex) => candidateIndex !== index))}
        title={translate('component.contentExtractorManagerDialog.removeLabelConfidence')}
      >
        <Trash2 className="h-3.5 w-3.5" />
      </AppDialogButton>
    </div>)}
  </div>;
}
