import { safeInvoke as invoke } from '../utils/tauri';

export const analysisApi = {
  analyze: <T>(request: Record<string, unknown>) => invoke<T>('analyze_content', { request }),
  analyticsSummary: <T>() => invoke<T>('get_analytics_summary'),
  listClassifiers: <T>() => invoke<T[]>('get_content_classifiers'),
  listExtractors: <T>() => invoke<T[]>('get_content_extractors'),
  listInspectors: <T>() => invoke<T[]>('get_content_inspectors'),
  listContentTypes: <T>(includeArchived = true) => invoke<T[]>('get_content_types', { includeArchived }),
  listContentTypeGroups: <T>(includeArchived = true) => invoke<T[]>('get_content_type_groups', { includeArchived }),
  testClassifier: <T>(input: unknown, sample: string) => invoke<T>('test_content_classifier', { input, sample }),
  rescanClassifications: <T>() => invoke<T>('rescan_content_classification_history', { confirmed: true }),
  rescanFileFormats: <T>() => invoke<T>('rescan_file_format_history', { confirmed: true }),
};
