export interface EffectiveVisualLabel {
  value: string;
  confidenceBasisPoints?: number;
  source: 'detected' | 'manual';
}

export interface EffectiveVisualLabels {
  clipId: number;
  labels: EffectiveVisualLabel[];
  hasOverrides: boolean;
}
