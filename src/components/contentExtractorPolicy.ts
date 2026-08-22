import type {
  ContentExtractor,
  ExtractorInput,
  ExtractorRecipe,
} from './contentExtractorModel.ts';

export function visibleContentExtractors(
  extractors: ContentExtractor[],
  features: { ocrEnabled: boolean; transcriptionsEnabled: boolean },
) {
  return extractors.filter((extractor) => (
    extractor.stableRef === 'extractor:apple-vision-ocr'
      || extractor.stableRef === 'extractor:tesseract-ocr'
      ? features.ocrEnabled
      : extractor.stableRef === 'extractor:whisper-transcription'
        ? features.transcriptionsEnabled
        : true
  ));
}

export function canSaveExtractorRecipe(recipe: ExtractorRecipe) {
  return recipe.accepts.length > 0
    && recipe.acceptedFileFormats.length > 0
    && !(recipe.acceptedFileFormats.length > 1 && recipe.acceptedFileFormats.includes('*'))
    && recipe.steps.length > 0
    && recipe.steps.every((step) => (
      Boolean(step.executable.path || step.executable.discover.length > 0)
      && step.id.trim().length > 0
    ));
}

export function extractorDraftIsDirty({
  draft,
  recipe,
  baselineDraft,
  baselineRecipe,
  hasAuthoredChanges,
}: {
  draft: ExtractorInput;
  recipe: ExtractorRecipe;
  baselineDraft: ExtractorInput | null;
  baselineRecipe: ExtractorRecipe | null;
  hasAuthoredChanges: boolean;
}) {
  return baselineDraft !== null && (
    JSON.stringify(draft) !== JSON.stringify(baselineDraft)
    || JSON.stringify(recipe) !== JSON.stringify(baselineRecipe)
    || hasAuthoredChanges
  );
}

export function resetExtractorRecipePreservingLocalPaths(
  current: ExtractorRecipe,
  defaults: ExtractorRecipe,
) {
  return {
    ...defaults,
    steps: defaults.steps.map((step) => ({
      ...step,
      executable: {
        ...step.executable,
        path: current.steps.find((candidate) => candidate.id === step.id)?.executable.path
          ?? step.executable.path,
      },
    })),
    resources: defaults.resources.map((resource) => ({
      ...resource,
      path: current.resources.find((candidate) => candidate.id === resource.id)?.path
        ?? resource.path,
    })),
  };
}
