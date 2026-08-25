export const appleImageFormats = ['bmp', 'gif', 'heif', 'jpg', 'png', 'tif', 'webp'];
export const tesseractImageFormats = ['bmp', 'gif', 'jpg', 'png', 'tif', 'webp'];
export const llamaImageFormats = ['bmp', 'jpg', 'png', 'webp'];
export const audioFormats = ['aac', 'flac', 'm4a', 'mp3', 'ogg', 'wav'];

export const appleDefaults = { name: 'Apple Vision OCR', description: 'Extracts searchable text from images locally with Apple Vision.', engine: 'recipe-v1', executablePath: null, modelPath: null, inputContract: 'image', outputContract: 'searchable_text', enabled: true, priority: 10 };
export const appleLabelsDefaults = { name: 'Apple Vision Labels', description: 'Finds searchable subjects and objects in images locally with Apple Vision.', engine: 'recipe-v1', executablePath: null, modelPath: null, inputContract: 'image', outputContract: 'searchable_text', enabled: true, priority: 15 };
export const llamaLabelsDefaults = { name: 'llama.cpp Labels', description: 'Finds searchable subjects and objects in images locally with llama.cpp.', engine: 'recipe-v1', executablePath: null, modelPath: null, inputContract: 'image', outputContract: 'searchable_text', enabled: true, priority: 20 };
export const tesseractDefaults = { name: 'Tesseract OCR', description: 'Extracts searchable text from images locally with Tesseract.', engine: 'recipe-v1', executablePath: null, modelPath: null, inputContract: 'image', outputContract: 'searchable_text', enabled: true, priority: 30 };
export const whisperDefaults = { name: 'Whisper Transcription', description: 'Extracts searchable text from local audio files with whisper.cpp.', engine: 'recipe-v1', executablePath: null, modelPath: null, inputContract: 'file_references', outputContract: 'searchable_text', enabled: true, priority: 40 };
