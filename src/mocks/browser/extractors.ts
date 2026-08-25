import { appleDefaults, appleImageFormats, appleLabelsDefaults, audioFormats,
  llamaImageFormats, llamaLabelsDefaults, tesseractDefaults, tesseractImageFormats,
  whisperDefaults } from './extractorDefaults';

export type MockExtractorRecipe = {
  definitionVersion: 1;
  accepts: Array<'image' | 'file_references'>;
  acceptedFileFormats: string[];
  minimumVisualLabelConfidence: number;
  output: 'searchable_text';
  steps: Array<{ id: string; executable: { path: string | null; discover: string[]; versionArguments: string[] }; arguments: string[]; mode: 'once' | 'each_input'; capture: 'ignore' | 'stdout_text' | 'file_text' | 'pasted_json_v1'; outputExtension: string | null; timeoutSeconds: number }>;
  resources: Array<{ id: string; label: string; kind: 'file' | 'directory'; required: boolean; path: string | null }>;
};

export const mockExtractorRecipe = (input: 'image' | 'file_references' | Array<'image' | 'file_references'>, command: string, acceptedFileFormats = ['*']): MockExtractorRecipe => ({
  definitionVersion: 1, accepts: Array.isArray(input) ? input : [input], acceptedFileFormats, minimumVisualLabelConfidence: 80, output: 'searchable_text',
  steps: [{ id: 'extract', executable: { path: null, discover: [command], versionArguments: ['--version'] }, arguments: ['{input.path}'], mode: 'once', capture: 'stdout_text', outputExtension: null, timeoutSeconds: 60 }],
  resources: [],
});

export type MockExtractor = {
  id: number; stableRef: string; name: string; description: string; engine: string;
  executablePath: string | null; modelPath: string | null; revision: number;
  inputContract: string; outputContract: string; enabled: boolean; priority: number;
  isBuiltin: boolean; isAvailable: boolean; unavailableReason: string | null;
  runtime: { method: string; location: string | null; version: string | null; usesAutomaticDiscovery: boolean; dependencies: Array<{ name: string; location: string | null; version: string | null; isAvailable: boolean; unavailableReason: string | null }> };
  recipe: MockExtractorRecipe; recipeHash: string; defaultRecipe: MockExtractorRecipe | null;
  defaults: typeof appleDefaults | typeof llamaLabelsDefaults | null;
};

const builtin = (id: number, stableRef: string, defaults: typeof appleDefaults | typeof llamaLabelsDefaults, command: string, formats: string[]): MockExtractor => ({
  id, stableRef, ...defaults, revision: 1,
  runtime: { method: stableRef.startsWith('extractor:apple-vision') ? 'system' : 'command', location: stableRef.startsWith('extractor:apple-vision') ? 'macOS Vision framework' : null, version: null, usesAutomaticDiscovery: !stableRef.startsWith('extractor:apple-vision'), dependencies: stableRef === 'extractor:whisper-transcription' ? [{ name: 'FFmpeg', location: '/mock/bin/ffmpeg', version: 'ffmpeg mock', isAvailable: true, unavailableReason: null }] : [] },
  isBuiltin: true, isAvailable: stableRef.startsWith('extractor:apple-vision'),
  unavailableReason: stableRef.startsWith('extractor:apple-vision')
    ? null
    : stableRef === 'extractor:tesseract-ocr'
      ? 'Tesseract OCR is not installed. Install Tesseract 5, then check again.'
      : stableRef === 'extractor:llama-cpp-labels'
        ? 'llama.cpp is not installed. Install llama.cpp, then check again.'
        : 'Whisper.cpp is not installed. Install whisper-cpp, then check again.',
  recipe: mockExtractorRecipe(stableRef === 'extractor:whisper-transcription' ? 'file_references' : ['image', 'file_references'], command, formats),
  recipeHash: `mock-${id}`,
  defaultRecipe: mockExtractorRecipe(stableRef === 'extractor:whisper-transcription' ? 'file_references' : ['image', 'file_references'], command, formats),
  defaults: { ...defaults },
});

export function mockBuiltinExtractors() {
  return [
    builtin(1, 'extractor:apple-vision-ocr', appleDefaults, 'pasted-bundled-extractor', appleImageFormats),
    builtin(2, 'extractor:apple-vision-labels', appleLabelsDefaults, 'pasted-bundled-extractor', appleImageFormats),
    builtin(3, 'extractor:llama-cpp-labels', llamaLabelsDefaults, 'llama-cli', llamaImageFormats),
    builtin(4, 'extractor:tesseract-ocr', tesseractDefaults, 'tesseract', tesseractImageFormats),
    builtin(5, 'extractor:whisper-transcription', whisperDefaults, 'whisper-cli', audioFormats),
  ];
}
