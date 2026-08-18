import type { ClipItem } from '../types';
import { getClipFilePaths, getClipNoteSummary } from '../types';

export type ClipSearchHighlightField = 'source' | 'content' | 'note';

export interface ClipSearchPlan {
  sources: string[];
  types: string[];
  terms: string[];
  requiresNote: boolean;
  requiresPinned: boolean;
  requiresProtected: boolean;
  requiresTrashed: boolean;
  hasIncompleteFilter: boolean;
  regex: RegExp | null;
  regexFallback: string | null;
}

function tokenizeSearch(query: string) {
  const tokens: string[] = [];
  let token = '';
  let quote: '"' | "'" | null = null;

  for (const character of query) {
    if (quote) {
      if (character === quote) quote = null;
      else token += character;
      continue;
    }
    if (character === '"' || character === "'") {
      quote = character;
    } else if (/\s/.test(character)) {
      if (token) tokens.push(token);
      token = '';
    } else {
      token += character;
    }
  }
  if (token) tokens.push(token);
  return tokens;
}

export function parseClipSearch(rawQuery: string): ClipSearchPlan {
  const trimmed = rawQuery.trim();
  const plan: ClipSearchPlan = {
    sources: [],
    types: [],
    terms: [],
    requiresNote: false,
    requiresPinned: false,
    requiresProtected: false,
    requiresTrashed: false,
    hasIncompleteFilter: false,
    regex: null,
    regexFallback: null,
  };

  // Preserve the original behavior: everything after a leading regex: is the
  // expression, including whitespace and characters that resemble filters.
  if (trimmed.toLowerCase().startsWith('regex:')) {
    const pattern = trimmed.slice(6);
    if (!pattern.trim()) {
      plan.hasIncompleteFilter = true;
      return plan;
    }
    try {
      plan.regex = new RegExp(pattern, 'i');
    } catch {
      plan.regexFallback = pattern.toLowerCase();
    }
    return plan;
  }

  tokenizeSearch(trimmed).forEach((token) => {
    const lower = token.toLowerCase();
    if (lower.startsWith('source:')) {
      const value = lower.slice(7).trim();
      if (value) plan.sources.push(value);
      else plan.hasIncompleteFilter = true;
    } else if (lower.startsWith('type:')) {
      const value = lower.slice(5).trim();
      if (value) plan.types.push(value);
      else plan.hasIncompleteFilter = true;
    } else if (lower === 'has:note') {
      plan.requiresNote = true;
    } else if (lower === 'is:pinned') {
      plan.requiresPinned = true;
    } else if (lower === 'is:protected') {
      plan.requiresProtected = true;
    } else if (lower === 'is:trashed') {
      plan.requiresTrashed = true;
    } else if (lower) {
      plan.terms.push(lower);
    }
  });

  return plan;
}

function searchableValues(clip: ClipItem) {
  return [
    clip.text_content,
    clip.source,
    getClipNoteSummary(clip.note),
    clip.content_type,
    ...(clip.content_types ?? []),
    ...getClipFilePaths(clip),
  ].filter((value): value is string => Boolean(value));
}

export function clipMatchesSearch(clip: ClipItem, plan: ClipSearchPlan) {
  if (plan.hasIncompleteFilter) return false;
  const values = searchableValues(clip);
  if (plan.regex) return values.some((value) => plan.regex?.test(value));
  if (plan.regexFallback !== null) {
    return values.some((value) => value.toLowerCase().includes(plan.regexFallback ?? ''));
  }

  const source = clip.source.toLowerCase();
  const types = [clip.content_type, ...(clip.content_types ?? [])].map((value) => value.toLowerCase());
  if (!plan.sources.every((value) => source.includes(value))) return false;
  if (!plan.types.every((value) => types.some((type) => type.includes(value)))) return false;
  if (plan.requiresNote && !clip.note?.trim()) return false;
  if (plan.requiresPinned && !clip.is_pinned) return false;
  if (plan.requiresProtected && !clip.is_protected) return false;
  if (plan.requiresTrashed && !clip.is_trashed) return false;

  const normalizedValues = values.map((value) => value.toLowerCase());
  return plan.terms.every((term) => normalizedValues.some((value) => value.includes(term)));
}

export function getClipSearchHighlightTerms(
  rawQuery: string,
  field: ClipSearchHighlightField,
) {
  const plan = parseClipSearch(rawQuery);
  if (plan.regex || plan.regexFallback !== null) return [];
  const terms = field === 'source' ? [...plan.terms, ...plan.sources] : [...plan.terms];
  return [...new Set(terms.filter(Boolean))].sort((left, right) => right.length - left.length);
}
