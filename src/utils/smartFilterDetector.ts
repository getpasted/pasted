import { FilterRule } from '../types';

export interface SmartDetectionResult {
  detectedTypes: string[];
  recommendedFilterIds: Set<number>;
  recommendedFilters: FilterRule[];
}

export function detectSmartFilterRecommendations(
  text: string,
  filters: FilterRule[]
): SmartDetectionResult {
  if (!text || typeof text !== 'string') {
    return { detectedTypes: [], recommendedFilterIds: new Set(), recommendedFilters: [] };
  }

  const detectedTypes: string[] = [];
  const recommendedFilterIds = new Set<number>();
  const trimmed = text.trim();

  // 1. Detect URLs
  const hasUrl = /https?:\/\/[^\s]+/i.test(text);
  if (hasUrl) {
    detectedTypes.push('URL Link');
  }

  // 2. Detect JSON
  let isJson = false;
  if ((trimmed.startsWith('{') && trimmed.endsWith('}')) || (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
    try {
      JSON.parse(trimmed);
      isJson = true;
      detectedTypes.push('JSON Data');
    } catch {}
  }

  // 3. Detect HTML / Markup
  const hasHtml = /<[a-z][\s\S]*>/i.test(text);
  if (hasHtml && !isJson) {
    detectedTypes.push('HTML Markup');
  }

  // 4. Detect Markdown
  const hasMarkdown = /(^|\s)(#{1,6}\s|\*\*|__|```|\[.+\]\(.+\))/m.test(text);
  if (hasMarkdown && !hasHtml && !isJson) {
    detectedTypes.push('Markdown Text');
  }

  // 5. Detect Multi-line
  const lines = text.split(/\r?\n/).filter((l) => l.trim().length > 0);
  const isMultiLine = lines.length > 1;
  if (isMultiLine) {
    detectedTypes.push(`${lines.length} Lines`);
  }

  // 6. Detect Emails
  const hasEmails = /[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/.test(text);
  if (hasEmails) {
    detectedTypes.push('Email Address');
  }

  // 7. Detect Phones
  const hasPhones = /\b(?:\+?\d{1,3}[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b/.test(text);
  if (hasPhones) {
    detectedTypes.push('Phone Number');
  }

  // Find matching filters based on name or pipeline config step types
  for (const filter of filters) {
    const nameLower = filter.name.toLowerCase();
    const configLower = (filter.config || '').toLowerCase();

    if (hasUrl && (nameLower.includes('url') || configLower.includes('clean_url_tracking') || configLower.includes('extract_urls'))) {
      recommendedFilterIds.add(filter.id);
    }
    if (isJson && (nameLower.includes('json') || configLower.includes('json_format') || configLower.includes('json_minify'))) {
      recommendedFilterIds.add(filter.id);
    }
    if (hasHtml && (nameLower.includes('html') || nameLower.includes('tag') || configLower.includes('strip_html') || configLower.includes('wrap_tags'))) {
      recommendedFilterIds.add(filter.id);
    }
    if (hasMarkdown && (nameLower.includes('markdown') || configLower.includes('strip_markdown'))) {
      recommendedFilterIds.add(filter.id);
    }
    if (isMultiLine && (nameLower.includes('sort') || nameLower.includes('dedupe') || nameLower.includes('line') || configLower.includes('sort_lines') || configLower.includes('dedupe_lines'))) {
      recommendedFilterIds.add(filter.id);
    }
    if (hasEmails && (nameLower.includes('email') || configLower.includes('extract_emails'))) {
      recommendedFilterIds.add(filter.id);
    }
    if (hasPhones && (nameLower.includes('phone') || configLower.includes('extract_phones'))) {
      recommendedFilterIds.add(filter.id);
    }
  }

  const recommendedFilters = filters.filter((f) => recommendedFilterIds.has(f.id));

  return {
    detectedTypes,
    recommendedFilterIds,
    recommendedFilters,
  };
}
