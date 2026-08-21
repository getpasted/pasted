interface MaskableClip {
  content_type: string;
  content_types?: string[];
  text_content: string | null;
}

export function concealedClipMask(clip: MaskableClip, text = clip.text_content): string {
  const trimmed = text?.trim() ?? '';
  const revealsLastFour = (clip.content_types ?? [clip.content_type]).includes('payment_card');
  return revealsLastFour && trimmed.length > 8
    ? `•••• •••• •••• ${trimmed.slice(-4)}`
    : '•••• ••••';
}
