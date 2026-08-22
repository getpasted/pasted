const imageFormats = ['bmp', 'gif', 'heif', 'jpg', 'png', 'tif', 'webp'];
const audioFormats = ['aac', 'flac', 'm4a', 'mp3', 'ogg', 'wav'];
const appleDefaults = { name: 'Apple Vision OCR', description: 'Extracts searchable text from images locally with Apple Vision.', engine: 'recipe-v1', executablePath: null, modelPath: null, inputContract: 'image', outputContract: 'searchable_text', enabled: true, priority: 10 };
const tesseractDefaults = { name: 'Tesseract OCR', description: 'Extracts searchable text from images locally with Tesseract.', engine: 'recipe-v1', executablePath: null, modelPath: null, inputContract: 'image', outputContract: 'searchable_text', enabled: true, priority: 20 };
const whisperDefaults = { name: 'Whisper Transcription', description: 'Extracts searchable text from local audio files with whisper.cpp.', engine: 'recipe-v1', executablePath: null, modelPath: null, inputContract: 'file_references', outputContract: 'searchable_text', enabled: true, priority: 30 };

export type MockExtractorRecipe = {
  definitionVersion: 1;
  accepts: Array<'image' | 'file_references'>;
  acceptedFileFormats: string[];
  output: 'searchable_text';
  steps: Array<{ id: string; executable: { path: string | null; discover: string[]; versionArguments: string[] }; arguments: string[]; mode: 'once' | 'each_input'; capture: 'ignore' | 'stdout_text' | 'file_text' | 'pasted_json_v1'; outputExtension: string | null; timeoutSeconds: number }>;
  resources: Array<{ id: string; label: string; kind: 'file' | 'directory'; required: boolean; path: string | null }>;
};

export const mockExtractorRecipe = (input: 'image' | 'file_references', command: string, acceptedFileFormats = ['*']): MockExtractorRecipe => ({
  definitionVersion: 1, accepts: [input], acceptedFileFormats, output: 'searchable_text',
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
  defaults: typeof appleDefaults | null;
};

const builtin = (id: number, stableRef: string, defaults: typeof appleDefaults, command: string, formats: string[]): MockExtractor => ({
  id, stableRef, ...defaults, revision: 1,
  runtime: { method: id === 1 ? 'system' : 'command', location: id === 1 ? 'macOS Vision framework' : null, version: null, usesAutomaticDiscovery: id !== 1, dependencies: id === 3 ? [{ name: 'FFmpeg', location: '/mock/bin/ffmpeg', version: 'ffmpeg mock', isAvailable: true, unavailableReason: null }] : [] },
  isBuiltin: true, isAvailable: id === 1,
  unavailableReason: id === 1
    ? null
    : id === 2
      ? 'Tesseract OCR is not installed. Install Tesseract 5, then check again.'
      : 'Whisper.cpp is not installed. Install whisper-cpp, then check again.',
  recipe: mockExtractorRecipe(defaults.inputContract as 'image' | 'file_references', command, formats),
  recipeHash: `mock-${id}`,
  defaultRecipe: mockExtractorRecipe(defaults.inputContract as 'image' | 'file_references', command, formats),
  defaults: { ...defaults },
});

export function mockBuiltinExtractors() {
  return [
    builtin(1, 'extractor:apple-vision-ocr', appleDefaults, 'pasted-bundled-extractor', imageFormats),
    builtin(2, 'extractor:tesseract-ocr', tesseractDefaults, 'tesseract', imageFormats),
    builtin(3, 'extractor:whisper-transcription', whisperDefaults, 'whisper-cli', audioFormats),
  ];
}
