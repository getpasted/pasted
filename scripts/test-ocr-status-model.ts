import assert from 'node:assert/strict';

import { actionableOcrCount, ocrScanCommand } from '../src/components/ocrStatusModel.ts';
import type { OcrBackfillStatus } from '../src/types.ts';

const status = (eligibleCount: number, failedCount: number): OcrBackfillStatus => ({
  totalImages: eligibleCount + failedCount,
  eligibleCount,
  queuedCount: 0,
  runningCount: 0,
  completedCount: 0,
  noTextCount: 0,
  failedCount,
});

assert.equal(actionableOcrCount(status(4, 6)), 10,
  'waiting and failed images must form one actionable scan pool');
assert.equal(ocrScanCommand(status(4, 6)), 'retry_failed_ocr',
  'a unified scan must reset failed attempts before scanning the whole pool');
assert.equal(ocrScanCommand(status(4, 0)), 'start_ocr_backfill',
  'a waiting-only scan must start without resetting history');

console.log('OCR status model tests passed.');
