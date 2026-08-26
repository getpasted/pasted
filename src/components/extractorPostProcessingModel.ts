import type { ExtractorRecipe } from './contentExtractorModel';

export const DEFAULT_LABEL_CONFIDENCE_PERCENT = 80;

export function replaceExtractorPostProcessing(
  recipe: ExtractorRecipe,
  postProcessing: ExtractorRecipe['postProcessing'],
) {
  return { ...recipe, postProcessing };
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
