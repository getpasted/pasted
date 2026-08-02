/**
 * Formats and extracts clean Unicode Emoji grapheme clusters (supporting VS-16 modifiers like ⭐/⭐️, ZWJ sequences, smileys, and skin tones).
 */
export const formatEmojiIcon = (icon: string | null | undefined): string => {
  if (!icon || !icon.trim()) return '📂';
  
  // Legacy string mappings for backward compatibility
  switch (icon.trim()) {
    case 'Code':
      return '💻';
    case 'MessageSquare':
      return '💬';
    case 'Palette':
      return '🎨';
    case 'Link':
      return '🔗';
    case 'Folder':
      return '📂';
    case 'Lock':
      return '🔒';
    case 'Key':
      return '🔑';
    case 'Star':
      return '⭐';
    case 'Heart':
      return '❤️';
    case 'FileText':
      return '📄';
    case 'Terminal':
      return '🖥️';
    case 'Zap':
      return '⚡';
    case 'Tag':
      return '🏷️';
  }

  try {
    const IntlAny = Intl as any;
    if (typeof IntlAny.Segmenter === 'function') {
      const segmenter = new IntlAny.Segmenter(undefined, { granularity: 'grapheme' });
      const segments = Array.from(segmenter.segment(icon)) as Array<{ segment: string }>;
      if (segments.length > 0) {
        return segments[0].segment || '📂';
      }
    }
  } catch (e) {
    console.error(e);
  }

  return icon || '📂';
};
