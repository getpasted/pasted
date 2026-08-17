import fs from 'node:fs';
import assert from 'node:assert/strict';

const read = (path) => fs.readFileSync(path, 'utf8');
const part = (value) => value
  .split(/[^A-Za-z0-9]+/)
  .filter(Boolean)
  .map((segment, index) => index === 0 ? segment : segment[0].toUpperCase() + segment.slice(1))
  .join('');

const messages = {};
const contentTypes = read('src/utils/contentTypes.ts');
for (const match of contentTypes.matchAll(/\{ value: '([^']+)', label: '([^']+)'/g)) {
  messages[`registry.contentType.${part(match[1])}.label`] = match[2];
}

const operations = read('src-tauri/src/operation_registry.rs');
for (const match of operations.matchAll(/(?:operation|configured_operation)\(\s*"([^"]+)",\s*"([^"]+)"/g)) {
  messages[`registry.operation.${part(match[1])}.name`] = match[2];
}

for (const [kind, path] of [
  ['classifier', 'src-tauri/src/content_classification.rs'],
  ['extractor', 'src-tauri/src/content_extraction.rs'],
]) {
  const source = read(path);
  for (const match of source.matchAll(/stable_ref:\s*(?:[A-Z_]+|"([^"]+)")[\s\S]{0,180}?name:\s*"([^"]+)"[\s\S]{0,220}?description:\s*"([^"]+)"/g)) {
    const stableRef = match[1];
    if (!stableRef) continue;
    messages[`registry.${kind}.${part(stableRef)}.name`] = match[2];
    messages[`registry.${kind}.${part(stableRef)}.description`] = match[3];
  }
}

Object.assign(messages, {
  'registry.extractor.extractorAppleVisionOcr.name': 'Apple Vision OCR',
  'registry.extractor.extractorAppleVisionOcr.description': 'Extracts searchable text from images locally with Apple Vision.',
  'registry.extractor.extractorTesseractOcr.name': 'Tesseract OCR',
  'registry.extractor.extractorTesseractOcr.description': 'Extracts searchable text from images locally with Tesseract.',
  'registry.extractor.extractorWhisperTranscription.name': 'Whisper Transcription',
  'registry.extractor.extractorWhisperTranscription.description': 'Extracts searchable text from local audio files with whisper.cpp.',
  'registry.library.captureClipTypeV1.name': 'Clip Type',
  'registry.library.captureClipTypeV1.description': 'Assigns exactly one structural Text, Image, or Files type from the captured clipboard representation.',
  'registry.library.captureSourceAttributionV1.name': 'Source Attribution',
  'registry.library.captureSourceAttributionV1.description': 'Records the application associated with a clipboard capture and resolves its icon when shown.',
  'registry.library.inspectorStructureV1.name': 'Structure',
  'registry.library.inspectorStructureV1.description': 'Measures stable clip structure without retaining clipboard contents.',
  'registry.library.inspectorMediaMetadataV1.name': 'Media Metadata',
  'registry.library.inspectorMediaMetadataV1.description': 'Reads bounded audio and video metadata locally.',
  'registry.library.suggestionSmartActionsV1.name': 'Smart Actions',
  'registry.library.suggestionSmartActionsV1.description': 'Suggests saved Transforms from content-free analysis signals.',
});

const localeDirectory = 'src/locales';
const checkOnly = process.argv.includes('--check');
const localeFiles = fs.readdirSync(localeDirectory).filter((file) => file.endsWith('.json') && file !== 'manifest.json');
for (const file of localeFiles) {
  const path = `${localeDirectory}/${file}`;
  const catalog = JSON.parse(read(path));
  if (checkOnly) {
    for (const [key, value] of Object.entries(messages)) {
      assert.ok(key in catalog, `${file} is missing registry presentation key ${key}`);
      if (file === 'en.json') assert.equal(catalog[key], value, `${key} must match its canonical registry value`);
    }
    continue;
  }
  for (const legacyKey of Object.keys(catalog).filter((key) => /^registry\.extractor\.extractor(?:AppleVisionOcrV1|TesseractOcrV1|WhisperCppV1)\./.test(key))) {
    delete catalog[legacyKey];
  }
  for (const [key, value] of Object.entries(messages)) {
    if (!(key in catalog)) catalog[key] = value;
  }
  const ordered = Object.fromEntries(Object.entries(catalog).sort(([left], [right]) => left.localeCompare(right)));
  fs.writeFileSync(path, `${JSON.stringify(ordered, null, 2)}\n`);
}

console.log(`${checkOnly ? 'Verified' : 'Synchronized'} ${Object.keys(messages).length} localized registry messages.`);
