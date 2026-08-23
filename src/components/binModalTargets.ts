import { localizedContentTypeGroupLabel } from '../localization/presentation';
import { translate } from '../localization/runtime';
import type {
  RegisteredContentType,
  RegisteredContentTypeGroup,
} from './ContentTypeProvider';
import {
  STRUCTURAL_CLIP_TYPES,
  type SmartBinFeatures,
  type SmartConditionRow,
  type SmartConditionTarget,
  type SmartTargetSection,
} from './binModalModel';
import { contentTypeLabel } from '../utils/contentTypes';

interface UseBinModalTargetsInput {
  contentTypes: RegisteredContentType[];
  contentTypeGroups: RegisteredContentTypeGroup[];
  features: SmartBinFeatures;
  fileFormats: string[];
  sources: string[];
  installedApps: string[];
}

export function buildBinModalTargets({
  contentTypes,
  contentTypeGroups,
  features,
  fileFormats,
  sources,
  installedApps,
}: UseBinModalTargetsInput) {
  const activeContentTypes = contentTypes.filter((type) => (
    !type.isArchived && !STRUCTURAL_CLIP_TYPES.has(type.id)
  ));
  const targetLabels: Record<SmartConditionTarget, string> = {
    clip_type: translate('component.binModal.clipType'),
    file_format: translate('component.binModal.fileFormat'),
    source: translate('component.binModal.source'),
    content_type: translate('component.binModal.contentType2'),
    origin_kind: translate('component.binModal.captureMethod'),
    contains: translate('component.binModal.textContent'),
    file_extension: translate('component.binModal.fileExtension'),
    file_path: translate('component.binModal.filePath'),
  };
  const targetSectionsFor = (condition: SmartConditionRow): SmartTargetSection[] => {
    const contentTypeChoices = activeContentTypes.map((type) => ({
      value: type.id,
      label: contentTypeLabel(type.id),
      group: (() => {
        const group = contentTypeGroups.find(({ id }) => id === type.group);
        return group
          ? localizedContentTypeGroupLabel(group.id, group.label, group.isBuiltin, group.defaults?.label)
          : type.group;
      })(),
    }));
    const formatChoices = fileFormats.map((format) => ({
      value: format,
      label: format.toUpperCase(),
    }));
    const sourceChoices = [...new Set([
      ...(condition.target === 'source' && condition.value ? [condition.value] : []),
      ...sources,
      ...installedApps,
    ])].map((source) => ({ value: source, label: source }));
    return [
      ...(features.clipTypes || condition.target === 'clip_type' ? [{
        target: 'clip_type' as const,
        label: targetLabels.clip_type,
        choices: [
          { value: 'text', label: translate('component.analyticsView.text'), disabled: !features.clipTypes },
          { value: 'image', label: translate('component.analyticsView.image'), disabled: !features.clipTypes },
          { value: 'file', label: translate('component.analyticsView.files'), disabled: !features.clipTypes },
        ],
      }] : []),
      ...(features.types || condition.target === 'content_type' ? [{
        target: 'content_type' as const,
        label: targetLabels.content_type,
        choices: contentTypeChoices.length > 0
          ? contentTypeChoices.map((choice) => ({ ...choice, disabled: !features.types }))
          : [{ value: condition.value, label: contentTypeLabel(condition.value), disabled: true }],
      }] : []),
      ...(features.fileFormats || condition.target === 'file_format' ? [{
        target: 'file_format' as const,
        label: targetLabels.file_format,
        choices: formatChoices.length > 0
          ? formatChoices.map((choice) => ({ ...choice, disabled: !features.fileFormats }))
          : [{
            value: condition.value,
            label: condition.value.toUpperCase() || translate('component.binModal.noDetectedFileFormats'),
            disabled: true,
          }],
      }] : []),
      ...(features.sources || condition.target === 'source' ? [{
        target: 'source' as const,
        label: targetLabels.source,
        choices: sourceChoices.length > 0
          ? sourceChoices.map((choice) => ({ ...choice, disabled: !features.sources }))
          : [{
            value: condition.value,
            label: condition.value || translate('component.binModal.noDetectedApps'),
            disabled: true,
          }],
      }] : []),
    ];
  };

  return { targetLabels, targetSectionsFor };
}
