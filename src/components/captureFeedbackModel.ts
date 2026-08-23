export type CaptureFeedbackKind = 'success' | 'ignored' | 'failure';

export interface CaptureFeedbackEvent {
  kind: CaptureFeedbackKind;
  clip_id?: number;
}

export interface CaptureFeedbackClip {
  id: number;
  contentType: string;
  previewText: string | null;
  source: string;
  isPinned: boolean;
  isProtected: boolean;
  isTrashed: boolean;
}

export interface CaptureFeedbackItem {
  id: number;
  kind: CaptureFeedbackKind;
  clip: CaptureFeedbackClip | null;
  image: string | null;
  entering?: boolean;
  exiting?: boolean;
  fading?: boolean;
  collapsing?: boolean;
  exitDirection?: -1 | 1;
}

export const CAPTURE_FEEDBACK_LAYOUT = {
  windowWidth: 340,
  previewHeight: 118,
  noticeHeight: 72,
  stackGap: 6,
  windowPadding: 6,
  maxStackItems: 4,
} as const;

export const MAX_CAPTURE_FEEDBACK_WINDOW_HEIGHT =
  CAPTURE_FEEDBACK_LAYOUT.previewHeight * CAPTURE_FEEDBACK_LAYOUT.maxStackItems
  + CAPTURE_FEEDBACK_LAYOUT.stackGap * (CAPTURE_FEEDBACK_LAYOUT.maxStackItems - 1)
  + CAPTURE_FEEDBACK_LAYOUT.windowPadding * 2;
