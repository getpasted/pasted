const FIRST_STRONG_ISOLATE = '\u2068';
const POP_DIRECTIONAL_ISOLATE = '\u2069';

/** Keeps interpolated user data and technical identifiers from reordering RTL copy. */
export function isolateBidi(value: string): string {
  return `${FIRST_STRONG_ISOLATE}${value}${POP_DIRECTIONAL_ISOLATE}`;
}

export function isRtlLocale(locale: string, rtlLocales: readonly string[]): boolean {
  return rtlLocales.includes(locale);
}
