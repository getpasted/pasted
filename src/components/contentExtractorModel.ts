import { translate } from '../localization/runtime';

export type ExtractorInputKind = 'image' | 'file_references';
export type ExtractorCapture = 'ignore' | 'stdout_text' | 'file_text' | 'pasted_json_v1';

export interface ExtractorRecipe {
  definitionVersion: 1;
  accepts: ExtractorInputKind[];
  acceptedFileFormats: string[];
  output: 'searchable_text';
  steps: Array<{
    id: string;
    executable: { path: string | null; discover: string[]; versionArguments: string[] };
    arguments: string[];
    mode: 'once' | 'each_input';
    capture: ExtractorCapture;
    outputExtension: string | null;
    timeoutSeconds: number;
  }>;
  resources: Array<{
    id: string;
    label: string;
    kind: 'file' | 'directory';
    required: boolean;
    path: string | null;
  }>;
}

export interface ExtractorAuthoringManifest {
  manifestVersion: 1;
  source: 'ai' | 'manual' | 'shipped' | 'migrated';
  originalPrompt: string | null;
  provider: string | null;
  model: string | null;
  messages: Array<{
    role: 'user' | 'assistant' | 'tool' | 'system';
    content: string;
    createdAt: string;
    structuredContent: unknown | null;
  }>;
}

export interface ExtractorRuntimeStatus {
  method: string;
  location: string | null;
  version: string | null;
  usesAutomaticDiscovery: boolean;
  dependencies: Array<{
    name: string;
    location: string | null;
    version: string | null;
    isAvailable: boolean;
    unavailableReason: string | null;
  }>;
}

export interface ContentExtractor {
  id: number;
  stableRef: string;
  name: string;
  description: string;
  engine: string;
  executablePath: string | null;
  modelPath: string | null;
  inputContract: string;
  outputContract: string;
  enabled: boolean;
  priority: number;
  revision: number;
  isBuiltin: boolean;
  isAvailable: boolean;
  unavailableReason: string | null;
  runtime: ExtractorRuntimeStatus;
  recipe: ExtractorRecipe;
  recipeHash: string;
  defaultRecipe: ExtractorRecipe | null;
  defaults: ExtractorInput | null;
}

export interface ExtractorInput {
  name: string;
  description: string;
  engine: string;
  executablePath: string | null;
  modelPath: string | null;
  inputContract: string;
  outputContract: string;
  enabled: boolean;
  priority: number;
}

export const EXTRACTOR_INPUT_OPTIONS = [
  { value: 'original_text', get label() { return translate('component.contentExtractorManagerDialog.text'); }, disabled: true },
  { value: 'image', get label() { return translate('component.contentExtractorManagerDialog.image'); }, disabled: false },
  { value: 'file_references', get label() { return translate('component.contentExtractorManagerDialog.file'); }, disabled: false },
] as const;

export const EXTRACTOR_OUTPUT_OPTIONS = [
  { value: 'searchable_text', get label() { return translate('component.contentExtractorManagerDialog.searchableText'); } },
] as const;

export const emptyRecipe = (): ExtractorRecipe => ({
  definitionVersion: 1,
  accepts: ['image'],
  acceptedFileFormats: ['*'],
  output: 'searchable_text',
  steps: [{
    id: 'extract',
    executable: { path: null, discover: [], versionArguments: ['--version'] },
    arguments: ['--pasted-extract-v1', '{request.path}'],
    mode: 'once',
    capture: 'pasted_json_v1',
    outputExtension: null,
    timeoutSeconds: 60,
  }],
  resources: [],
});

export function toInput(extractor?: ContentExtractor): ExtractorInput {
  return extractor ? {
    name: extractor.name,
    description: extractor.description,
    engine: extractor.engine,
    executablePath: extractor.executablePath,
    modelPath: extractor.modelPath,
    inputContract: extractor.inputContract,
    outputContract: extractor.outputContract,
    enabled: extractor.enabled,
    priority: extractor.priority,
  } : {
    name: 'Custom Extractor',
    get description() { return translate('component.contentExtractorManagerDialog.extractsSearchableTextWithALocalCommand'); },
    engine: 'recipe-v1',
    executablePath: null,
    modelPath: null,
    inputContract: 'image',
    outputContract: 'searchable_text',
    enabled: false,
    priority: 100,
  };
}

export interface ExtractorRecipeProposal {
  name: string;
  description: string;
  recipe: ExtractorRecipe;
  setupGuidance: string[];
  authoring: ExtractorAuthoringManifest;
  connectionName: string;
}

export type ExtractorDiagnosticCode =
  | 'invalid_recipe'
  | 'executable_not_configured'
  | 'executable_unavailable'
  | 'resource_not_configured'
  | 'resource_unavailable';

export interface ExtractorDiagnosticReport {
  version: 1;
  isAvailable: boolean;
  platform: string;
  architecture: string;
  packageManagers: string[];
  issues: Array<{
    code: ExtractorDiagnosticCode;
    subjectId: string;
    label: string;
    detail: string;
  }>;
}

export type ExtractorRepairStatus = 'ready' | 'setup_required' | 'guidance_incomplete';

export interface ExtractorRepairOutcome extends ExtractorRecipeProposal {
  diagnostic: ExtractorDiagnosticReport;
  status: ExtractorRepairStatus;
  attempts: number;
  connectionId: string;
  durationMs: number;
}

export type ExtractorTestOutcome =
  | { outcome: 'produced'; text: string }
  | { outcome: 'no_output' }
  | { outcome: 'failed'; failure: { code: string; message: string } };

export interface ExtractorAuthoringSession {
  id: number;
  extractorId: number;
  source: 'ai' | 'manual' | 'shipped' | 'migrated';
  provider: string | null;
  model: string | null;
  originalPrompt: string | null;
  createdAt: string;
  messages: ExtractorAuthoringManifest['messages'];
}

export function authoringRoleLabel(role: ExtractorAuthoringManifest['messages'][number]['role']) {
  switch (role) {
    case 'user': return translate('component.contentExtractorManagerDialog.roleUser');
    case 'assistant': return translate('component.contentExtractorManagerDialog.roleAssistant');
    case 'tool': return translate('component.contentExtractorManagerDialog.roleTool');
    case 'system': return translate('component.contentExtractorManagerDialog.roleSystem');
  }
}
