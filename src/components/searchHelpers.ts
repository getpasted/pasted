import { translate, type TranslationKey } from '../localization/runtime';
import type { FeatureId } from '../utils/features';

interface SearchHelperDefinition {
  prefix: string;
  descriptionKey: TranslationKey;
  feature?: FeatureId;
}

const SEARCH_HELPERS: readonly SearchHelperDefinition[] = [
  { prefix: 'regex:', descriptionKey: 'component.sidebar.regex' },
  { prefix: 'clip:', descriptionKey: 'component.sidebar.clipTypes', feature: 'clipTypes' },
  { prefix: 'content:', descriptionKey: 'component.sidebar.contentTypes', feature: 'types' },
  { prefix: 'format:', descriptionKey: 'component.sidebar.fileFormats', feature: 'fileFormats' },
  { prefix: 'source:', descriptionKey: 'component.sidebar.sources', feature: 'sources' },
  { prefix: 'has:note', descriptionKey: 'feature.notes.label', feature: 'notes' },
  { prefix: 'has:name', descriptionKey: 'feature.naming.label', feature: 'naming' },
  { prefix: 'is:pinned', descriptionKey: 'collection.pinned', feature: 'pinning' },
  { prefix: 'is:protected', descriptionKey: 'collection.protected', feature: 'protection' },
  { prefix: 'is:trashed', descriptionKey: 'collection.trashed', feature: 'trash' },
];

export function getSearchHelpers(features: Record<FeatureId, boolean>) {
  return SEARCH_HELPERS
    .filter(({ feature }) => !feature || features[feature])
    .map(({ prefix, descriptionKey }) => ({ prefix, description: translate(descriptionKey) }));
}
