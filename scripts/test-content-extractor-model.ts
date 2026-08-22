import assert from 'node:assert/strict';

import {
  canSaveExtractorRecipe,
  extractorDraftIsDirty,
  visibleContentExtractors,
} from '../src/components/contentExtractorPolicy.ts';
import type {
  ContentExtractor,
  ExtractorInput,
  ExtractorRecipe,
} from '../src/components/contentExtractorModel.ts';

const extractor = (stableRef: string) => ({ stableRef } as ContentExtractor);
const apple = extractor('extractor:apple-vision-ocr');
const tesseract = extractor('extractor:tesseract-ocr');
const whisper = extractor('extractor:whisper-transcription');
const custom = extractor('extractor:custom');

assert.deepEqual(
  visibleContentExtractors([apple, tesseract, whisper, custom], {
    ocrEnabled: false,
    transcriptionsEnabled: true,
  }),
  [whisper, custom],
  'OCR and transcription gates must hide only their shipped Extractors',
);

const validRecipe: ExtractorRecipe = {
  definitionVersion: 1,
  accepts: ['image'],
  output: 'searchable_text',
  steps: [{
    id: 'extract',
    executable: { path: null, discover: [], versionArguments: ['--version'] },
    arguments: ['--pasted-extract-v1', '{request.path}'],
    mode: 'once',
    capture: 'pasted_json_v1',
    outputExtension: null,
    timeoutSeconds: 60,
  }],
  resources: [],
};
assert.equal(canSaveExtractorRecipe(validRecipe), false,
  'a recipe without an executable path or discovery command must not save');
assert.equal(canSaveExtractorRecipe({
  ...validRecipe,
  steps: validRecipe.steps.map((step) => ({
    ...step,
    executable: { ...step.executable, discover: ['extractor'] },
  })),
}), true, 'a named step with an executable discovery command may save');
assert.equal(canSaveExtractorRecipe({ ...validRecipe, accepts: [] }), false,
  'a recipe without an accepted input must not save');

const baselineDraft: ExtractorInput = {
  name: 'Extractor',
  description: 'Description',
  engine: 'recipe-v1',
  executablePath: null,
  modelPath: null,
  inputContract: 'image',
  outputContract: 'searchable_text',
  enabled: false,
  priority: 100,
};
assert.equal(extractorDraftIsDirty({
  draft: baselineDraft,
  recipe: validRecipe,
  baselineDraft,
  baselineRecipe: validRecipe,
  hasAuthoredChanges: false,
}), false, 'an unchanged Extractor draft must remain clean');
assert.equal(extractorDraftIsDirty({
  draft: { ...baselineDraft, enabled: true },
  recipe: validRecipe,
  baselineDraft,
  baselineRecipe: validRecipe,
  hasAuthoredChanges: false,
}), true, 'a changed Extractor field must make the draft dirty');
assert.equal(extractorDraftIsDirty({
  draft: baselineDraft,
  recipe: validRecipe,
  baselineDraft,
  baselineRecipe: validRecipe,
  hasAuthoredChanges: true,
}), true, 'generated authoring metadata must make the draft dirty');

console.log('Content Extractor model tests passed.');
