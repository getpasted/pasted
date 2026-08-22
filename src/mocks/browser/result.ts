export type BrowserMockResult =
  | { matched: true; value: unknown }
  | { matched: false };

export const unhandled: BrowserMockResult = { matched: false };
export const handled = (value: unknown): BrowserMockResult => ({ matched: true, value });

export const unhandledValue = Symbol('unhandled browser command');
