export interface ContentClassifier {
  id: number;
  stable_ref: string;
  name: string;
  content_type: string;
  description: string;
  patterns: string[];
  validator: string | null;
  enabled: boolean;
  priority: number;
  is_builtin: boolean;
  defaults: ClassifierInput | null;
}

export interface ClassifierInput {
  name: string;
  content_type: string;
  description: string;
  patterns: string[];
  validator: string | null;
  enabled: boolean;
  priority: number;
}

export type ClassifierModifiedFields = Record<keyof ClassifierInput, boolean>;

export function toClassifierInput(classifier: ContentClassifier): ClassifierInput {
  return {
    name: classifier.name,
    content_type: classifier.content_type,
    description: classifier.description,
    patterns: classifier.patterns,
    validator: classifier.validator,
    enabled: classifier.enabled,
    priority: classifier.priority,
  };
}

export function emptyClassifierInput(name: string): ClassifierInput {
  return {
    name,
    content_type: 'prose',
    description: '',
    patterns: ['^.+$'],
    validator: null,
    enabled: true,
    priority: 200,
  };
}

export function normalizedClassifierInput(
  draft: ClassifierInput,
  patternsText: string,
): ClassifierInput {
  return {
    ...draft,
    name: draft.name.trim(),
    content_type: draft.content_type.trim(),
    description: draft.description.trim(),
    patterns: patternsText.split('\n').map((pattern) => pattern.trim()).filter(Boolean),
  };
}

export function classifierModifiedFields(
  current: ClassifierInput,
  comparison: ClassifierInput | null,
  isNew: boolean,
): ClassifierModifiedFields {
  return {
    name: !isNew && comparison !== null && current.name !== comparison.name,
    content_type: !isNew && comparison !== null && current.content_type !== comparison.content_type,
    description: !isNew && comparison !== null && current.description !== comparison.description,
    patterns: !isNew && comparison !== null
      && JSON.stringify(current.patterns) !== JSON.stringify(comparison.patterns),
    validator: !isNew && comparison !== null && current.validator !== comparison.validator,
    enabled: !isNew && comparison !== null && current.enabled !== comparison.enabled,
    priority: !isNew && comparison !== null && current.priority !== comparison.priority,
  };
}

export function classifierDraftIsDirty(
  current: ClassifierInput,
  baseline: ClassifierInput | null,
) {
  return baseline !== null && JSON.stringify(current) !== JSON.stringify(baseline);
}

export function nextClassifierSelection(
  classifiers: ContentClassifier[],
  current: number | 'new' | null,
): number | 'new' {
  return typeof current === 'number' && classifiers.some(({ id }) => id === current)
    ? current
    : classifiers[0]?.id ?? 'new';
}
