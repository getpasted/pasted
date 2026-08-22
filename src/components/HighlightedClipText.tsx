import React from 'react';

import { getClipSearchHighlightTerms, type ClipSearchHighlightField } from '../utils/clipSearch';

export function HighlightedClipText({
  text,
  query,
  field,
}: {
  text: string;
  query?: string;
  field: ClipSearchHighlightField;
}) {
  if (!query) return <bdi>{text}</bdi>;
  const terms = getClipSearchHighlightTerms(query, field);
  if (terms.length === 0) return <bdi>{text}</bdi>;
  const escaped = terms.map((term) => term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
  const expression = new RegExp(`(${escaped.join('|')})`, 'gi');
  return <bdi>
    {text.split(expression).map((part, index) => (
      terms.some((term) => term.toLowerCase() === part.toLowerCase())
        ? <mark className="clip-search-match" key={`${part}:${index}`}>{part}</mark>
        : <React.Fragment key={`${part}:${index}`}>{part}</React.Fragment>
    ))}
  </bdi>;
}
