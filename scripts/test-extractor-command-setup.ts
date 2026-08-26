import assert from 'node:assert/strict';

import { extractorSetupCommands } from '../src/components/extractorCommandSetupModel.ts';

const recipe = (discover: string[], args: string[] = []) => ({
  steps: [{ executable: { discover }, arguments: args }],
  resources: [] as Array<{ required: boolean; path: string | null }>,
});
const llamaRecipe = recipe(['llama-cli'], ['-hf', 'example/model']);
assert.deepEqual(
  extractorSetupCommands(llamaRecipe, 'macos').map(({ command }) => command),
  ['brew install llama.cpp', 'llama-cli -hf example/model -p "" -n 0 --no-warmup'],
  'llama-compatible recipes should receive install and model-cache commands',
);

const tesseractRecipe = recipe(['tesseract']);
assert.deepEqual(
  extractorSetupCommands(tesseractRecipe, 'windows').map(({ command }) => command),
  ['winget install UB-Mannheim.TesseractOCR'],
  'known dependencies should receive platform-specific setup without Extractor identity checks',
);

const customRecipe = recipe(['custom-tool']);
assert.deepEqual(extractorSetupCommands(customRecipe, 'linux'), []);

llamaRecipe.resources = [{ required: true, path: null }];
assert.deepEqual(
  extractorSetupCommands(llamaRecipe, 'macos'),
  [],
  'partial deterministic guidance should defer to the general diagnosis workflow',
);

console.log('Extractor command setup tests passed.');
