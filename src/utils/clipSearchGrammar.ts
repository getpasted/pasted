export interface ClipSearchPlan {
  sources: string[];
  clipTypes: string[];
  contentTypes: string[];
  formats: string[];
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
    clipTypes: [],
    contentTypes: [],
    formats: [],
    terms: [],
    requiresNote: false,
    requiresPinned: false,
    requiresProtected: false,
    requiresTrashed: false,
    hasIncompleteFilter: false,
    regex: null,
    regexFallback: null,
  };

  // A leading regex owns the remainder, including whitespace and filter-like text.
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
    } else if (lower.startsWith('clip:')) {
      const value = lower.slice(5).trim();
      if (value) plan.clipTypes.push(value);
      else plan.hasIncompleteFilter = true;
    } else if (lower.startsWith('content:')) {
      const value = lower.slice(8).trim();
      if (value) plan.contentTypes.push(value);
      else plan.hasIncompleteFilter = true;
    } else if (lower.startsWith('format:')) {
      const value = lower.slice(7).trim();
      if (value) plan.formats.push(value);
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
