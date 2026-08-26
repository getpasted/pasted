import type { ExtractorRecipe } from './contentExtractorModel';

export const DEFAULT_LABEL_CONFIDENCE_PERCENT = 80;

export function replaceExtractorPostProcessing(
  recipe: ExtractorRecipe,
  postProcessing: ExtractorRecipe['postProcessing'],
) {
  const updated = { ...recipe } as ExtractorRecipe & {
    minimumVisualLabelConfidence?: number;
  };
  delete updated.minimumVisualLabelConfidence;
  return { ...updated, postProcessing };
}

export function addLabelConfidencePostProcessing(recipe: ExtractorRecipe) {
  return replaceExtractorPostProcessing(recipe, [
    ...recipe.postProcessing,
    {
      kind: 'filter_labels_by_confidence',
      minimumPercent: DEFAULT_LABEL_CONFIDENCE_PERCENT,
    },
  ]);
}
