import { createContext, useContext, useEffect, useMemo, useSyncExternalStore, type ReactNode } from 'react';
import {
  availableLocales,
  formatDateTime,
  formatNumber,
  getLocalizationSnapshot,
  refreshSystemLocale,
  subscribeLocalization,
  translate,
  type LocaleDefinition,
  type TranslationKey,
} from './runtime';

interface LocalizationContextValue {
  configuredLanguage: string;
  locale: string;
  direction: 'ltr' | 'rtl';
  catalogReady: boolean;
  locales: readonly LocaleDefinition[];
  t: typeof translate;
  formatNumber: (value: number, options?: Intl.NumberFormatOptions) => string;
  formatDateTime: (value: Date | number | string, options?: Intl.DateTimeFormatOptions) => string;
}

const LocalizationContext = createContext<LocalizationContextValue | null>(null);

export function LocalizationProvider({ children }: { children: ReactNode }) {
  const snapshot = useSyncExternalStore(
    subscribeLocalization,
    getLocalizationSnapshot,
    getLocalizationSnapshot,
  );

  useEffect(() => {
    document.documentElement.lang = snapshot.locale;
    document.documentElement.dir = snapshot.direction;
  }, [snapshot.direction, snapshot.locale]);

  useEffect(() => {
    window.addEventListener('languagechange', refreshSystemLocale);
    return () => window.removeEventListener('languagechange', refreshSystemLocale);
  }, []);

  const value = useMemo<LocalizationContextValue>(() => ({
    ...snapshot,
    locales: availableLocales(),
    t: (key: TranslationKey, variables = {}) => translate(key, variables, snapshot.locale),
    formatNumber: (number, options) => formatNumber(number, options, snapshot.locale),
    formatDateTime: (date, options) => formatDateTime(date, options, snapshot.locale),
  }), [snapshot]);

  return <LocalizationContext.Provider value={value}>{children}</LocalizationContext.Provider>;
}

export function useLocalization(): LocalizationContextValue {
  const value = useContext(LocalizationContext);
  if (!value) throw new Error('useLocalization must be used inside LocalizationProvider.');
  return value;
}
