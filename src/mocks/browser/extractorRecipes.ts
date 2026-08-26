export type MockExtractorRecipe = {
  definitionVersion: 1;
  accepts: Array<'image' | 'file_references'>;
  acceptedFileFormats: string[];
  postProcessing: Array<{ kind: 'filter_labels_by_confidence'; minimumPercent: number }>;
  output: 'searchable_text';
  steps: Array<{ id: string; executable: { path: string | null; discover: string[]; versionArguments: string[] }; arguments: string[]; mode: 'once' | 'each_input'; capture: 'ignore' | 'stdout_text' | 'file_text' | 'pasted_json_v1'; outputExtension: string | null; timeoutSeconds: number }>;
  resources: Array<{ id: string; label: string; kind: 'file' | 'directory'; required: boolean; path: string | null }>;
};

export const mockExtractorRecipe = (
  input: 'image' | 'file_references' | Array<'image' | 'file_references'>,
  command: string,
  acceptedFileFormats = ['*'],
  args = ['{input.path}'],
): MockExtractorRecipe => ({
  definitionVersion: 1,
  accepts: Array.isArray(input) ? input : [input],
  acceptedFileFormats,
  postProcessing: [],
  output: 'searchable_text',
  steps: [{ id: 'extract', executable: { path: null, discover: [command], versionArguments: ['--version'] }, arguments: args, mode: 'once', capture: 'stdout_text', outputExtension: null, timeoutSeconds: 60 }],
  resources: [],
});
