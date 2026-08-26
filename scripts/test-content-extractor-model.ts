import assert from 'node:assert/strict';

import {
  canSaveExtractorRecipe,
  extractorDraftIsDirty,
  resetExtractorRecipePreservingLocalPaths,
  visibleContentExtractors,
} from '../src/components/contentExtractorPolicy.ts';
import type {
  ContentExtractor,
  ExtractorInput,
  ExtractorRecipe,
} from '../src/components/contentExtractorModel.ts';
import { EXTRACTOR_FILE_FORMAT_GROUPS } from '../src/components/extractorFileFormats.ts';
import { addLabelConfidencePostProcessing } from '../src/components/extractorPostProcessingModel.ts';
import { groupSelectionState, initialMultiSelectScrollKey, toggleMultiSelectGroup } from '../src/components/menuMultiSelectModel.ts';

const extractor = (stableRef: string) => ({ stableRef } as ContentExtractor);
const apple = extractor('extractor:apple-vision-ocr');
const tesseract = extractor('extractor:tesseract-ocr');
const whisper = extractor('extractor:whisper-transcription');
const custom = extractor('extractor:custom');

assert.deepEqual(EXTRACTOR_FILE_FORMAT_GROUPS.map(({ id }) => id),
  ['any', 'audio', 'images', 'video', 'documents'],
  'Extractor File Formats must remain grouped by their detected media type');
assert.deepEqual(EXTRACTOR_FILE_FORMAT_GROUPS.find(({ id }) => id === 'documents')?.formats, ['pdf'],
  'PDF must be presented as a Document format');
const audioFormats = EXTRACTOR_FILE_FORMAT_GROUPS.find(({ id }) => id === 'audio')?.formats
  .map((value) => ({ value })) ?? [];
assert.deepEqual(groupSelectionState(['mp3'], audioFormats), { all: false, some: true },
  'a partially selected File Format group must expose its mixed state');
assert.deepEqual(toggleMultiSelectGroup(['pdf'], audioFormats), ['pdf', 'aac', 'flac', 'm4a', 'mp3', 'ogg', 'wav'],
  'selecting a File Format group must preserve other groups and select every member');
assert.deepEqual(toggleMultiSelectGroup(['pdf', ...audioFormats.map(({ value }) => value)], audioFormats), ['pdf'],
  'clearing a File Format group must preserve selections in other groups');
assert.equal(initialMultiSelectScrollKey(['mp3'], audioFormats.map((option) => ({ ...option, group: 'Audio' }))), 'group:Audio',
  'a grouped menu must initially orient the first selected group toward the top');

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
  acceptedFileFormats: ['*'],
  postProcessing: [],
  output: 'searchable_text',
  steps: [{
    id: 'extract',
    executable: { path: null, discover: [], versionArguments: ['--version'] },
    arguments: ['--pasted-extract-v1', '{request.path}'],
    mode: 'once',
    capture: 'pasted_json_v1',
    outputExtension: null,
    noOutputExitCodes: [],
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
assert.equal(canSaveExtractorRecipe({ ...validRecipe, acceptedFileFormats: [] }), false,
  'a recipe without accepted file formats must not save');
assert.equal(canSaveExtractorRecipe({ ...validRecipe, acceptedFileFormats: ['*', 'pdf'] }), false,
  'the any-format selector cannot be combined with a specific format');
assert.equal(canSaveExtractorRecipe({
  ...validRecipe,
  postProcessing: [{ kind: 'filter_labels_by_confidence', minimumPercent: 80 }],
}), false, 'post-processing must not bypass command validation');
assert.equal(canSaveExtractorRecipe({
  ...validRecipe,
  steps: validRecipe.steps.map((step) => ({
    ...step,
    executable: { ...step.executable, discover: ['extractor'] },
  })),
  postProcessing: [{ kind: 'filter_labels_by_confidence', minimumPercent: 101 }],
}), false, 'label confidence must use the same bounded validation in the GUI');
const confidenceRecipe = addLabelConfidencePostProcessing(validRecipe);
assert.equal(confidenceRecipe.postProcessing[0]?.minimumPercent, 80,
  'custom Extractors must receive the same default confidence as shipped recipes');
const configuredRecipe: ExtractorRecipe = {
  ...validRecipe,
  steps: validRecipe.steps.map((step) => ({
    ...step,
    executable: { ...step.executable, path: '/local/extractor' },
  })),
  resources: [{ id: 'model', label: 'Model', kind: 'file', required: true, path: '/local/model.bin' }],
};
const resetRecipe = resetExtractorRecipePreservingLocalPaths(configuredRecipe, {
  ...validRecipe,
  resources: [{ id: 'model', label: 'Default Model', kind: 'file', required: true, path: null }],
});
assert.equal(resetRecipe.steps[0]?.executable.path, '/local/extractor',
  'resetting an Extractor must preserve its local executable binding');
assert.equal(resetRecipe.resources[0]?.path, '/local/model.bin',
  'resetting an Extractor must preserve its local model binding');

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
