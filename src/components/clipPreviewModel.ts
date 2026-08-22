export interface StructuralInspection {
  formatVersion: number;
  policy: 'capture' | 'background' | 'interactive' | 'rescan';
  through: 'inspect' | 'extract' | 'classify' | 'suggest';
  result: {
    origin: 'clipboard_content' | 'file_reference' | 'screenshot' | 'command_line';
    byteCount: number;
    text?: { characterCount: number; wordCount: number; lineCount: number };
    image?: { width: number; height: number };
    files?: { itemCount: number; extensions: string[] };
  };
  appliedClipId: number | null;
  mediaMetadata?: {
    examinedFileCount: number;
    mediaFileCount: number;
    audioStreamCount: number;
    videoStreamCount: number;
    totalDurationMs: number;
    containers: string[];
    codecs: string[];
  };
  fileFormats?: {
    formats: Array<{ format: string; mimeType: string; count: number }>;
    inspectedCount: number;
    unknownCount: number;
    unavailableCount: number;
  };
  liveFileObservations?: {
    availableCount: number;
    fileCount: number;
    directoryCount: number;
    totalSizeBytes: number;
  };
}

export interface SmartActionSuggestion {
  formatVersion: number;
  policy: 'interactive';
  through: 'suggest';
  result: {
    signals: Array<'url' | 'json' | 'html' | 'markdown' | 'multi_line' | 'email' | 'phone'>;
    signalLabels: string[];
    actions: Array<{
      transformRef: string;
      transformName: string;
      transformRevision: number;
      reasons: string[];
    }>;
  };
  appliedClipId: null;
}

export interface AnalyzerPreview {
  formatVersion: number;
  policy: 'capture' | 'background' | 'interactive' | 'rescan';
  through: 'inspect' | 'extract' | 'classify' | 'suggest';
  result: {
    clipKind: string;
    structure?: StructuralInspection['result'];
    mediaMetadata?: StructuralInspection['mediaMetadata'];
    classificationMatches?: Array<{
      classifierRef: string;
      classifierName: string;
      contentType: string;
      priority: number;
      startOffset: number;
      endOffset: number;
    }>;
    searchableTextAvailable: boolean;
    suggestions?: SmartActionSuggestion['result'];
  };
  appliedClipId: null;
  liveFileObservations?: StructuralInspection['liveFileObservations'];
}

export interface ExtractionApplicationResult {
  formatVersion: number;
  policy: 'capture' | 'background' | 'interactive' | 'rescan';
  through: 'inspect' | 'extract' | 'classify' | 'suggest';
  outcome: 'produced' | 'no_output' | 'failed';
  output: string | null;
  classificationMatches: AnalyzerPreview['result']['classificationMatches'];
  failure: { code: string; message: string } | null;
  appliedClipId: number | null;
  ocrUpdated: boolean;
  searchableTextUpdated: boolean;
  classificationUpdated: boolean;
}

export interface ExtractionResult {
  extractorRef: string;
  extractorName: string;
  engine: string;
  priority: number;
  duplicateOf?: string;
  outcome: 'produced' | 'no_output' | 'failed';
  text?: string;
  failure?: { code: string; message: string };
  updatedAt: string;
}

export interface ExtractionAttempt extends ExtractionResult {
  runId: string;
  runAt: string;
}

export interface ClipSearchableText {
  clipId: number;
  extractorRef: string;
  extractorName: string;
  engine: string;
  inputHash: string;
  searchableText: string;
  updatedAt: string;
}

export interface ClipContentMatch {
  id: number;
  clipId: number;
  contentType: string;
  classifierRef: string;
  classifierName: string;
  priority: number;
  sourceRepresentation: 'original_text' | 'searchable_text';
  inputHash: string;
  startOffset: number | null;
  endOffset: number | null;
  updatedAt: string;
}

export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = bytes / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && value >= 1024; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value >= 10 ? value.toFixed(0) : value.toFixed(1)} ${unit}`;
}

export function formatMediaDuration(milliseconds: number): string {
  const totalSeconds = Math.floor(milliseconds / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
    : `${minutes}:${String(seconds).padStart(2, '0')}`;
}

export function contentMatchTitle(contentType: string, matches: ClipContentMatch[]): string | undefined {
  const counts = matches
    .filter((match) => match.contentType === contentType)
    .reduce((result, match) => {
      result.set(match.classifierName, (result.get(match.classifierName) ?? 0) + 1);
      return result;
    }, new Map<string, number>());
  if (counts.size === 0) return undefined;
  return [...counts].map(([name, count]) => count > 1 ? `${name} ×${count}` : name).join(', ');
}
