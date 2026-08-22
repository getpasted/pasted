import type { OcrBackfillStatus } from '../types';

export const OCR_STATUS_CARD_KEYS = [
  ['images', 'totalImages'],
  ['waiting', 'eligibleCount'],
  ['running', 'runningCount'],
  ['complete', 'completedCount'],
  ['noText', 'noTextCount'],
  ['failed', 'failedCount'],
] as const;

export function actionableOcrCount(status: OcrBackfillStatus) {
  return status.eligibleCount + status.failedCount;
}

export function ocrScanCommand(status: OcrBackfillStatus) {
  return status.failedCount > 0 ? 'retry_failed_ocr' : 'start_ocr_backfill';
}
