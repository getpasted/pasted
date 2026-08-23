export function lastEmojiGrapheme(value: string): string | null {
  if (!value) return null;
  try {
    const IntlWithSegmenter = Intl as typeof Intl & {
      Segmenter?: new (locale?: string, options?: { granularity: string }) => {
        segment: (input: string) => Iterable<{ segment: string }>;
      };
    };
    if (typeof IntlWithSegmenter.Segmenter === 'function') {
      const segmenter = new IntlWithSegmenter.Segmenter(undefined, { granularity: 'grapheme' });
      const segments = Array.from(segmenter.segment(value));
      const lastGrapheme = segments[segments.length - 1]?.segment;
      if (lastGrapheme?.trim()) return lastGrapheme;
    }
  } catch (error) {
    console.error(error);
  }
  const characters = Array.from(value);
  return characters[characters.length - 1] ?? null;
}
