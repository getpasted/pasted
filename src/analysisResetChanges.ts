import { analysisApi } from './api/analysis';
import type { ContentClassifier } from './components/classifierModel';
import type { ContentExtractor } from './components/contentExtractorModel';
import type { RegisteredContentType, RegisteredContentTypeGroup } from './components/ContentTypeProvider';
import type { SettingsResetChange } from './components/SettingsResetChanges';
import { translate } from './localization/runtime';

export async function analysisResetChanges(): Promise<SettingsResetChange[]> {
  const [extractors, classifiers, types, groups] = await Promise.all([
    analysisApi.listExtractors<ContentExtractor>(),
    analysisApi.listClassifiers<ContentClassifier>(),
    analysisApi.listContentTypes<RegisteredContentType>(),
    analysisApi.listContentTypeGroups<RegisteredContentTypeGroup>(),
  ]);
  const modified = translate('component.settingsResetChanges.modified');
  const shippedDefault = translate('component.settingsResetChanges.shippedDefault');
  return [
    ...extractors.filter((item) => item.defaults && (
      JSON.stringify(extractorValues(item)) !== JSON.stringify(item.defaults)
      || (item.defaultRecipe !== null && JSON.stringify(item.recipe) !== JSON.stringify(item.defaultRecipe))
    )).map((item) => ({ label: item.name, before: modified, after: shippedDefault })),
    ...classifiers.filter((item) => item.defaults
      && JSON.stringify(classifierValues(item)) !== JSON.stringify(item.defaults))
      .map((item) => ({ label: item.name, before: modified, after: shippedDefault })),
    ...types.filter((item) => item.defaults && (item.isArchived
      || JSON.stringify(typeValues(item)) !== JSON.stringify(item.defaults)))
      .map((item) => ({ label: item.label, before: modified, after: shippedDefault })),
    ...groups.filter((item) => item.defaults && (item.isArchived
      || JSON.stringify(groupValues(item)) !== JSON.stringify(item.defaults)))
      .map((item) => ({ label: item.label, before: modified, after: shippedDefault })),
  ];
}

function extractorValues(item: ContentExtractor) {
  const { name, description, engine, executablePath, modelPath, inputContract, outputContract, enabled, priority } = item;
  return { name, description, engine, executablePath, modelPath, inputContract, outputContract, enabled, priority };
}

function classifierValues(item: ContentClassifier) {
  const { name, content_type, description, patterns, validator, enabled, priority } = item;
  return { name, content_type, description, patterns, validator, enabled, priority };
}

function typeValues(item: RegisteredContentType) {
  const { label, icon, group, concealClips } = item;
  return { label, icon, group, concealClips };
}

function groupValues(item: RegisteredContentTypeGroup) {
  const { label, sortOrder } = item;
  return { label, sortOrder };
}
