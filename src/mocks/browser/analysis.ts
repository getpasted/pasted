import { handled, unhandled, type BrowserMockResult } from './result';

export function handleAnalysisBrowserMock(command: string): BrowserMockResult {
  if (command !== 'get_content_inspectors') return unhandled;
  return handled([
    { stableRef: 'inspector:structure-v1', name: 'Structure', description: 'Measures stable clip structure without retaining clipboard contents.', inputContract: 'clip', outputContract: 'structural_metadata', priority: 0, isBuiltin: true, engine: null, isAvailable: true, unavailableReason: null },
    { stableRef: 'inspector:media-metadata-v1', name: 'Media Metadata', description: 'Reads bounded audio and video metadata locally.', inputContract: 'file_references', outputContract: 'media_metadata', priority: 10, isBuiltin: true, engine: 'ffprobe-cli-v1', isAvailable: true, unavailableReason: null },
  ]);
}
