export interface PastePart {
  kind: string;
  role: string | null;
  aliases: string[];
  startOffset: number;
  endOffset: number;
  confidence: number;
  analyzerRef: string;
}

export interface PastePartsAnalysis {
  formatVersion: number;
  parts: PastePart[];
}

export interface ClipDetectedContent {
  classificationMatches: ClipContentMatch[];
  pasteParts: PastePartsAnalysis | null;
}
import type { ClipContentMatch } from './clipPreviewModel';
