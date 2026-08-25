import type { ClipItem } from '../types';
import type { ColorFormats } from '../utils/color';
import type { EffectiveVisualLabels, ExtractionAttempt, ExtractionResult } from './clipPreviewModel';
import type { FileClipPreview } from './fileClipPreviewModel';

export interface ClipPreviewContentProps {
  clip: ClipItem;
  displayText: string;
  previewingRevision: boolean;
  colorData: ColorFormats | null;
  resolvedImageBase64: string | null;
  filePreviews: FileClipPreview[];
  isFilePreviewLoading: boolean;
  fileSearchableText: { extractorName: string; searchableText: string } | null;
  extractionResults: ExtractionResult[];
  visualLabels: EffectiveVisualLabels | null;
  extractionHistory: ExtractionAttempt[];
  extractionHistoryHasMore: boolean;
  isExtractionHistoryLoading: boolean;
  isFileExtractionLoading: boolean;
  copiedFormat: string | null;
  isOcrLoading: boolean;
  ocrEnabled: boolean;
  transcriptionsEnabled: boolean;
  readOnly?: boolean;
  onColorChange: (value: string) => void;
  onCopyFormat: (label: string, value: string) => void;
  onRunOCR: () => void;
  onRunFileExtraction: () => void;
  onLoadExtractionHistory: (reset: boolean) => void;
  onRecheckFileReference: (index: number) => Promise<void>;
  onAddVisualLabel: (label: string) => void | Promise<void>;
  onRemoveVisualLabel: (label: string) => void | Promise<void>;
  onResetVisualLabels: () => void | Promise<void>;
}
