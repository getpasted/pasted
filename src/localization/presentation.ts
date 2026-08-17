import { hasTranslationKey, translate } from './runtime';

function keyPart(value: string): string {
  return value
    .split(/[^A-Za-z0-9]+/)
    .filter(Boolean)
    .map((segment, index) => index === 0 ? segment : segment[0].toUpperCase() + segment.slice(1))
    .join('');
}

function translatedRegistryValue(kind: string, stableId: string, field: 'name' | 'description' | 'label', fallback: string): string {
  const key = `registry.${kind}.${keyPart(stableId)}.${field}`;
  return hasTranslationKey(key) ? translate(key) : fallback;
}

export function localizedContentTypeLabel(
  id: string,
  label: string,
  isBuiltin = true,
  defaultLabel: string | undefined = label,
): string {
  if (!isBuiltin || label !== defaultLabel) return label;
  return translatedRegistryValue('contentType', id, 'label', label);
}

export function localizedContentTypeGroupLabel(
  id: string,
  label: string,
  isBuiltin = true,
  defaultLabel: string | undefined = label,
): string {
  if (!isBuiltin || label !== defaultLabel) return label;
  const keyById: Record<string, string> = {
    general: 'component.contentTypeProvider.general',
    developer: 'component.contentTypeProvider.developer',
    personal_financial: 'component.contentTypeProvider.personalAndFinancial',
    identifiers: 'component.contentTypeProvider.identifiers',
    custom: 'common.custom',
  };
  const key = keyById[id];
  return key && hasTranslationKey(key) ? translate(key) : label;
}

export function localizedBuiltinName(
  kind: 'classifier' | 'extractor' | 'library' | 'operation',
  stableId: string,
  name: string,
  isBuiltin: boolean,
  defaultName: string | undefined = name,
): string {
  if (!isBuiltin || name !== defaultName) return name;
  const normalizedId = kind === 'operation' ? stableId.replace(/^builtin:/, '') : stableId;
  return translatedRegistryValue(kind, normalizedId, 'name', name);
}

export function localizedBuiltinDescription(
  kind: 'classifier' | 'extractor' | 'library',
  stableId: string,
  description: string,
  isBuiltin: boolean,
  defaultDescription: string | undefined = description,
): string {
  if (!isBuiltin || description !== defaultDescription) return description;
  return translatedRegistryValue(kind, stableId, 'description', description);
}

export function localizedSourceName(source: string): string {
  return !source || source === 'Unknown' ? translate('common.unknown') : source;
}
