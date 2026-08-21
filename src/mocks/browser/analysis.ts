import { handled, unhandled, type BrowserMockResult } from './result';

export function handleAnalysisBrowserMock(command: string): BrowserMockResult {
  if (command === 'get_content_inspectors') return handled([
      { stableRef: 'inspector:structure-v1', name: 'Structure', description: 'Measures stable clip structure without retaining clipboard contents.', inputContract: 'clip', outputContract: 'structural_metadata', priority: 0, isBuiltin: true, engine: null, isAvailable: true, unavailableReason: null },
      { stableRef: 'inspector:media-metadata-v1', name: 'Media Metadata', description: 'Reads bounded audio and video metadata locally.', inputContract: 'file_references', outputContract: 'media_metadata', priority: 10, isBuiltin: true, engine: 'ffprobe-cli-v1', isAvailable: true, unavailableReason: null },
    ]);
  if (command === 'get_search_index_status' || command === 'rebuild_search_index') return handled({
    schemaVersion: 1,
    indexes: [
      { stableRef: 'index:captured-clips-v1', canonicalCount: 2, indexedCount: 2, healthy: true, engine: 'SQLite FTS5', includedFields: ['content', 'name', 'note', 'source'] },
      { stableRef: 'index:extracted-text-v1', canonicalCount: 0, indexedCount: 0, healthy: true, engine: 'SQLite FTS5', includedFields: ['extractedText'] },
    ],
  });
  return unhandled;
}
