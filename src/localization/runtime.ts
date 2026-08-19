import manifestData from '../locales/manifest.json';
import englishCatalogData from '../locales/en.json';
import { setPresentationLocale } from './state';
import { isolateBidi, isRtlLocale } from './bidi';

type MessageVariables = Record<string, string | number>;
type PluralMessage = Partial<Record<Intl.LDMLPluralRule, string>> & { other: string };
type Message = string | PluralMessage;
type Catalog = Record<string, Message>;

export interface LocaleDefinition {
  code: string;
  name: string;
  nativeName: string;
  direction: 'ltr' | 'rtl';
  catalog: string;
}

interface LocaleManifest {
  schemaVersion: number;
  defaultLocale: string;
  locales: LocaleDefinition[];
}

export type TranslationKey = keyof typeof englishCatalogData;

export function hasTranslationKey(key: string): key is TranslationKey {
  return Object.prototype.hasOwnProperty.call(englishCatalogData, key);
}

const manifest = manifestData as LocaleManifest;
const rtlLocales = manifest.locales.filter(({ direction }) => direction === 'rtl').map(({ code }) => code);
const localeNameCollator = new Intl.Collator('en', { sensitivity: 'base' });
const sortedLocales = [...manifest.locales].sort((left, right) => (
  localeNameCollator.compare(left.nativeName, right.nativeName)
));
const catalogModules = import.meta.glob([
  '../locales/*.json',
  '!../locales/en.json',
  '!../locales/manifest.json',
], {
  import: 'default',
}) as Record<string, () => Promise<Catalog>>;
const catalogs: Record<string, Catalog | undefined> = {
  [manifest.defaultLocale]: englishCatalogData as Catalog,
};
const catalogLoads = new Map<string, Promise<void>>();
const listeners = new Set<() => void>();
const warnedKeys = new Set<string>();

function readCachedLanguage(): string {
  try {
    return localStorage.getItem('pasted_cache_language') ?? 'system';
  } catch {
    return 'system';
  }
}

let configuredLanguage = readCachedLanguage();

export function availableLocales(): readonly LocaleDefinition[] {
  return sortedLocales;
}

export function isSupportedLocale(value: string): boolean {
  return manifest.locales.some(({ code }) => code === value);
}

export function isConfiguredLanguage(value: string): boolean {
  return value === 'system' || isSupportedLocale(value);
}

function browserLocales(): readonly string[] {
  if (typeof navigator === 'undefined') return [];
  return navigator.languages?.length ? navigator.languages : [navigator.language];
}

export function resolveLocale(
  language = configuredLanguage,
  preferredLocales: readonly string[] = browserLocales(),
): string {
  if (language !== 'system' && isSupportedLocale(language)) return language;
  for (const preferred of preferredLocales) {
    const normalized = preferred.toLowerCase();
    const exact = manifest.locales.find(({ code }) => code.toLowerCase() === normalized);
    if (exact) return exact.code;
    const base = normalized.split('-')[0];
    const baseMatch = manifest.locales.find(({ code }) => code.toLowerCase().split('-')[0] === base);
    if (baseMatch) return baseMatch.code;
  }
  return manifest.defaultLocale;
}

export interface LocalizationSnapshot {
  configuredLanguage: string;
  locale: string;
  direction: 'ltr' | 'rtl';
  catalogReady: boolean;
}

let snapshot: LocalizationSnapshot;

function createSnapshot(): LocalizationSnapshot {
  const locale = resolveLocale();
  return {
    configuredLanguage,
    locale,
    direction: manifest.locales.find(({ code }) => code === locale)?.direction ?? 'ltr',
    catalogReady: Boolean(catalogs[locale]),
  };
}

snapshot = createSnapshot();
setPresentationLocale(snapshot.locale);
loadCurrentCatalog();

function publish() {
  snapshot = createSnapshot();
  setPresentationLocale(snapshot.locale);
  listeners.forEach((listener) => listener());
}

function loadCatalog(locale: string): Promise<void> {
  if (catalogs[locale]) return Promise.resolve();
  const existing = catalogLoads.get(locale);
  if (existing) return existing;
  const definition = manifest.locales.find(({ code }) => code === locale);
  const loader = definition ? catalogModules[`../locales/${definition.catalog}`] : undefined;
  if (!loader) return Promise.resolve();
  const load = loader()
    .then((catalog) => {
      catalogs[locale] = catalog;
    })
    .catch((error) => {
      console.error(`Failed to load locale ${locale}:`, error);
    })
    .finally(() => catalogLoads.delete(locale));
  catalogLoads.set(locale, load);
  return load;
}

function loadCurrentCatalog() {
  const locale = snapshot.locale;
  void loadCatalog(locale).then(() => {
    if (snapshot.locale === locale) publish();
  });
}

export function setConfiguredLanguage(language: string) {
  const next = isConfiguredLanguage(language) ? language : 'system';
  if (next === configuredLanguage) return;
  configuredLanguage = next;
  try {
    localStorage.setItem('pasted_cache_language', next);
  } catch {
    // SQLite remains authoritative when browser storage is unavailable.
  }
  publish();
  loadCurrentCatalog();
}

export function refreshSystemLocale() {
  if (configuredLanguage === 'system') {
    publish();
    loadCurrentCatalog();
  }
}

export function subscribeLocalization(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function getLocalizationSnapshot(): LocalizationSnapshot {
  return snapshot;
}

function interpolate(template: string, variables: MessageVariables, locale: string): string {
  const rtl = isRtlLocale(locale, rtlLocales);
  return template.replace(/\{([A-Za-z][A-Za-z0-9_]*)\}/g, (placeholder, name: string) => (
    Object.prototype.hasOwnProperty.call(variables, name)
      ? rtl
        ? isolateBidi(typeof variables[name] === 'number'
          ? new Intl.NumberFormat(locale).format(variables[name])
          : String(variables[name]))
        : typeof variables[name] === 'number'
        ? new Intl.NumberFormat(locale).format(variables[name])
        : String(variables[name])
      : placeholder
  ));
}

export function translate(
  key: TranslationKey,
  variables: MessageVariables = {},
  locale = snapshot.locale,
): string {
  const defaultCatalog = catalogs[manifest.defaultLocale] ?? englishCatalogData as Catalog;
  const selectedCatalog = catalogs[locale] ?? defaultCatalog;
  const message = selectedCatalog[key] ?? defaultCatalog[key];
  if (message === undefined) {
    if (import.meta.env.DEV && !warnedKeys.has(key)) {
      warnedKeys.add(key);
      console.warn(`Missing localization key: ${key}`);
    }
    return key;
  }
  if (typeof message === 'string') return interpolate(message, variables, locale);
  const count = Number(variables.count ?? 0);
  const category = new Intl.PluralRules(locale).select(count);
  return interpolate(message[category] ?? message.other, variables, locale);
}

export function formatTransformRequestPhase({
  phase,
  connectionName,
  didFallback = false,
  label,
  ellipsis = false,
}: {
  phase: 'starting' | 'queued' | 'running';
  connectionName?: string | null;
  didFallback?: boolean;
  label?: string | null;
  ellipsis?: boolean;
}) {
  let message: string;
  if (phase === 'starting') {
    message = label
      ? translate('transformRequest.startingLabel', { label })
      : translate('transformRequest.starting');
  } else if (phase === 'queued') {
    message = connectionName
      ? translate('transformRequest.queuedForConnection', { connection: connectionName })
      : translate('transformRequest.queued');
  } else if (connectionName && label && didFallback) {
    message = translate('transformRequest.runningLabelWithConnectionFallback', { label, connection: connectionName });
  } else if (connectionName && label) {
    message = translate('transformRequest.runningLabelWithConnection', { label, connection: connectionName });
  } else if (connectionName && didFallback) {
    message = translate('transformRequest.runningWithConnectionFallback', { connection: connectionName });
  } else if (connectionName) {
    message = translate('transformRequest.runningWithConnection', { connection: connectionName });
  } else if (label) {
    message = translate('transformRequest.runningLabel', { label });
  } else {
    message = translate('transformRequest.running');
  }
  return ellipsis ? `${message}…` : message;
}

export function formatNumber(value: number, options?: Intl.NumberFormatOptions, locale = snapshot.locale) {
  return new Intl.NumberFormat(locale, options).format(value);
}

export function formatDateTime(
  value: Date | number | string,
  options?: Intl.DateTimeFormatOptions,
  locale = snapshot.locale,
) {
  const date = value instanceof Date ? value : new Date(value);
  return new Intl.DateTimeFormat(locale, options).format(date);
}
