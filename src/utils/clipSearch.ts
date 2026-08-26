import type { ClipItem } from '../types';
import { getClipFilePaths, getClipNoteSummary } from '../types';
import type { FeatureId } from './features';
import { parseClipSearch, type ClipSearchPlan } from './clipSearchGrammar';

export { parseClipSearch } from './clipSearchGrammar';
export type { ClipSearchPlan } from './clipSearchGrammar';

export type ClipSearchHighlightField = 'source' | 'content' | 'name' | 'note';

export type ClipSearchFeaturePolicy = Pick<
  Record<FeatureId, boolean>,
  'clipTypes' | 'fileFormats' | 'types' | 'sources' | 'naming' | 'notes' | 'pinning' | 'protection'
>;

function searchableValues(clip: ClipItem, features?: ClipSearchFeaturePolicy) {
  const values = [
    clip.text_content,
    ...getClipFilePaths(clip),
  ];
  if (!features || features.sources) values.push(clip.source);
  if (!features || features.naming) values.push(clip.name ?? '');
  if (!features || features.notes) values.push(getClipNoteSummary(clip.note));
  if (!features || features.clipTypes) values.push(clip.content_type);
  if (!features || features.types) values.push(...(clip.content_types ?? []));
  if (!features || features.fileFormats) values.push(...(clip.file_formats ?? []));
  return values.filter((value): value is string => Boolean(value));
}

export function clipMatchesSearch(
  clip: ClipItem,
  plan: ClipSearchPlan,
  features?: ClipSearchFeaturePolicy,
) {
  if (plan.hasIncompleteFilter) return false;
  const values = searchableValues(clip, features);
  if (plan.regex) return values.some((value) => plan.regex?.test(value));
  if (plan.regexFallback !== null) {
    return values.some((value) => value.toLowerCase().includes(plan.regexFallback ?? ''));
  }

  const source = clip.source.toLowerCase();
  const clipType = clip.content_type.toLowerCase();
  const contentTypes = (clip.content_types ?? []).map((value) => value.toLowerCase());
  const formats = (clip.file_formats ?? []).map((value) => value.toLowerCase());
  if (plan.clipIds.length > 0 && !plan.clipIds.includes(clip.id)) return false;
  if (plan.sources.length > 0 && features && !features.sources) return false;
  if (plan.clipTypes.length > 0 && features && !features.clipTypes) return false;
  if (plan.contentTypes.length > 0 && features && !features.types) return false;
  if (plan.formats.length > 0 && features && !features.fileFormats) return false;
  if (!plan.sources.every((value) => source.includes(value))) return false;
  if (!plan.clipTypes.every((value) => clipType.includes(value))) return false;
  if (!plan.contentTypes.every((value) => contentTypes.some((type) => type.includes(value)))) return false;
  if (!plan.formats.every((value) => formats.some((format) => format.includes(value)))) return false;
  if (plan.requiresNote && !clip.note?.trim()) return false;
  if (plan.requiresNamed && (!features || features.naming) && !clip.name?.trim()) return false;
  if (plan.requiresNamed && features && !features.naming) return false;
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
