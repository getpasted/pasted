export interface ExtractionResult {
  extractorRef: string;
  extractorName: string;
  engine: string;
  priority: number;
  duplicateOf?: string;
  outcome: 'produced' | 'no_output' | 'failed';
  text?: string;
  labels?: Array<{ value: string; confidenceBasisPoints?: number }>;
  failure?: { code: string; message: string };
  updatedAt: string;
}

export interface ExtractionAttempt extends ExtractionResult {
  runId: string;
  runAt: string;
  inputFingerprint: string;
  failureClass: 'terminal' | 'dependency' | 'transient' | null;
  retryAfter: string | null;
}
