import type { ClipSearchRequest } from '../types';

function quotedFilterValue(value: string): string | null {
  if (!value || (value.includes('"') && value.includes("'"))) return null;
  if (value.includes('"')) return `'${value}'`;
  if (/\s/.test(value) || value.includes("'")) return `"${value}"`;
  return value;
}

function hasBalancedQuotes(value: string) {
  let quote: '"' | "'" | null = null;
  for (const character of value) {
    if (quote === character) quote = null;
    else if (!quote && (character === '"' || character === "'")) quote = character;
  }
  return quote === null;
}

function filterTerms(prefix: string, values?: string[]): string[] | null {
  const terms: string[] = [];
  for (const value of values ?? []) {
    const serialized = quotedFilterValue(value);
    if (!serialized) return null;
    terms.push(`${prefix}:${serialized}`);
  }
  return terms;
}

export function searchHistoryRequestQuery(request: ClipSearchRequest): string | null {
  const explicitFilters = Boolean(
    request.clipIds?.length || request.clipTypes?.length || request.contentTypes?.length
    || request.fileFormats?.length || request.sources?.length || request.trash,
  );
  // A leading regex owns the remainder of Search grammar, so filters cannot be appended faithfully.
  if (explicitFilters && (
    request.query.trim().toLowerCase().startsWith('regex:') || !hasBalancedQuotes(request.query)
  )) return null;
  const clipTypes = filterTerms('clip', request.clipTypes);
  const contentTypes = filterTerms('content', request.contentTypes);
  const fileFormats = filterTerms('format', request.fileFormats);
  const sources = filterTerms('source', request.sources);
  if (!clipTypes || !contentTypes || !fileFormats || !sources) return null;
  const serialized = [
    request.query.trim(),
    request.clipIds?.length ? `id:${request.clipIds.join(',')}` : '',
    ...clipTypes,
    ...contentTypes,
    ...fileFormats,
    ...sources,
    request.trash ? 'is:trashed' : '',
  ].filter(Boolean).join(' ');
  return serialized.length <= 16_384 ? serialized : null;
}

export function searchHistoryRequestSummary(request: ClipSearchRequest): string {
  return [
    request.clipIds?.length ? `id:${request.clipIds.join(',')}` : '',
    ...(request.clipTypes ?? []).map((value) => `clip:${value}`),
    ...(request.contentTypes ?? []).map((value) => `content:${value}`),
    ...(request.fileFormats ?? []).map((value) => `format:${value}`),
    ...(request.sources ?? []).map((value) => `source:${value}`),
    request.trash ? 'is:trashed' : '',
  ].filter(Boolean).join(' · ');
}
