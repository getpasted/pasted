import { CONTENT_TYPES } from '../../utils/contentTypes';
import type { MockClip } from './models';
import { mockManualTransforms } from './manualTransforms';
import { mockBuiltinExtractors, mockExtractorRecipe, type MockExtractor, type MockExtractorRecipe } from './extractors';
import { unhandledValue } from './result';

function mockClassifier<T extends { id: number; patterns: string[] }>(classifier: T) {
  return { ...classifier, defaults: { ...classifier, patterns: [...classifier.patterns] } };
}
let mockClassifiers = [
  mockClassifier({ id: 1, stable_ref: 'email', name: 'Email Addresses', content_type: 'email', description: 'Individual email addresses', patterns: [String.raw`(?i)^[^\s@]+@[^\s@]+\.[^\s@]+$`], validator: null, enabled: true, priority: 30, is_builtin: true }),
  mockClassifier({ id: 2, stable_ref: 'credential', name: 'Credentials', content_type: 'credential', description: 'Known API-key formats and secret assignments', patterns: [String.raw`^(?:sk_|ghp_).+$`], validator: null, enabled: true, priority: 60, is_builtin: true }),
  mockClassifier({ id: 3, stable_ref: 'phone', name: 'Phone Numbers', content_type: 'phone', description: 'Formatted international and local phone numbers', patterns: [String.raw`^\+?[0-9 ()-]{7,}$`], validator: 'phone', enabled: true, priority: 160, is_builtin: true }),
];
let nextMockExtractorId = 4;
let mockExtractors: MockExtractor[] = mockBuiltinExtractors();
const mockFileSearchableText = new Map<number, {
  clipId: number;
  extractorRef: string;
  extractorName: string;
  engine: string;
  inputHash: string;
  searchableText: string;
  updatedAt: string;
}>();

let mockContentTypes: Array<{
  id: string; label: string; icon: string; group: string; concealClips: boolean; isBuiltin: boolean; isArchived: boolean;
  defaults: { label: string; icon: string; group: string; concealClips: boolean } | null;
}> = CONTENT_TYPES.map(({ value, label, icon, group, concealClips = false }) => {
  const groupId = group.toLowerCase().replace(/ & /g, '_').replace(/ /g, '_');
  return { id: value as string, label, icon, group: groupId, concealClips, isBuiltin: true, isArchived: false, defaults: { label, icon, group: groupId, concealClips } };
});
let mockContentTypeGroups: Array<{
  id: string; label: string; sortOrder: number; isBuiltin: boolean; isArchived: boolean;
  defaults: { label: string; sortOrder: number } | null;
}> = [
  { id: 'general', label: 'General', sortOrder: 10, isBuiltin: true, isArchived: false, defaults: { label: 'General', sortOrder: 10 } },
  { id: 'developer', label: 'Developer', sortOrder: 20, isBuiltin: true, isArchived: false, defaults: { label: 'Developer', sortOrder: 20 } },
  { id: 'personal_financial', label: 'Personal and financial', sortOrder: 30, isBuiltin: true, isArchived: false, defaults: { label: 'Personal and financial', sortOrder: 30 } },
  { id: 'identifiers', label: 'Identifiers', sortOrder: 40, isBuiltin: true, isArchived: false, defaults: { label: 'Identifiers', sortOrder: 40 } },
  { id: 'custom', label: 'Custom', sortOrder: 50, isBuiltin: true, isArchived: false, defaults: { label: 'Custom', sortOrder: 50 } },
];


export function getMockFileSearchableText(clipId: number) {
  return mockFileSearchableText.get(clipId)?.searchableText;
}

export async function invokeContentBrowserMock<T>(
  cmd: string,
  args: Record<string, unknown> | undefined,
  mockClips: MockClip[],
): Promise<T | typeof unhandledValue> {
  switch (cmd) {
    case 'get_content_classifiers':
      return mockClassifiers.map((classifier) => ({ ...classifier, patterns: [...classifier.patterns] })) as unknown as T;
    case 'get_content_extractors':
      return mockExtractors.map((extractor) => ({ ...extractor, recipe: structuredClone(extractor.recipe), defaultRecipe: extractor.defaultRecipe ? structuredClone(extractor.defaultRecipe) : null, defaults: extractor.defaults ? { ...extractor.defaults } : null })) as unknown as T;
    case 'get_content_extractor_runtime': {
      const extractor = mockExtractors.find((item) => item.stableRef === args?.reference);
      if (!extractor) throw new Error('Extractor not found.');
      return structuredClone(extractor.runtime) as unknown as T;
    }
    case 'choose_extractor_executable':
      return '/mock/bin/custom-extractor' as unknown as T;
    case 'choose_extractor_resource_file':
      return '/mock/input/sample.dat' as unknown as T;
    case 'test_content_extractor_recipe':
      return { outcome: 'produced', text: 'Mock extracted text' } as unknown as T;
    case 'diagnose_content_extractor_recipe': {
      const recipe = args?.recipe as MockExtractorRecipe;
      const issues = [
        ...recipe.steps.filter((step) => !step.executable.path && step.executable.discover.length === 0)
          .map((step) => ({ code: 'executable_not_configured', subjectId: step.id, label: step.id, detail: 'No executable is configured.' })),
        ...recipe.resources.filter((resource) => resource.required && !resource.path)
          .map((resource) => ({ code: 'resource_not_configured', subjectId: resource.id, label: resource.label, detail: 'A required resource is not configured.' })),
      ];
      return { version: 1, isAvailable: issues.length === 0, platform: 'browser', architecture: 'mock', packageManagers: [], issues } as unknown as T;
    }
    case 'extract_ocr_from_clip': {
      const clipId = Number(args?.clipId);
      if (!Number.isInteger(clipId) || clipId <= 0) throw new Error('A valid clip ID is required.');
      return {
        formatVersion: 1,
        policy: 'interactive',
        through: 'suggest',
        targetKind: 'extractor',
        targetRef: 'extractor:apple-vision-ocr',
        outcome: 'produced',
        output: 'Recognized text',
        classificationMatches: [],
        failure: null,
        participants: [{ stableRef: 'extractor:apple-vision-ocr', pass: 'extract', outcome: 'produced' }],
        appliedClipId: clipId,
        ocrUpdated: true,
        searchableTextUpdated: false,
        classificationUpdated: false,
      } as unknown as T;
    }
    case 'get_clip_searchable_text':
      return (mockFileSearchableText.get(Number(args?.clipId)) ?? null) as unknown as T;
    case 'get_clip_content_matches':
      return [] as unknown as T;
    case 'extract_text_from_file_clip': {
      const clipId = Number(args?.clipId);
      if (!Number.isInteger(clipId) || clipId <= 0) throw new Error('A valid clip ID is required.');
      const searchableText = 'Locally transcribed audio';
      mockFileSearchableText.set(clipId, {
        clipId,
        extractorRef: 'extractor:whisper-transcription',
        extractorName: 'Whisper Transcription',
        engine: 'whisper-cpp-cli-v1',
        inputHash: `mock-file-${clipId}`,
        searchableText,
        updatedAt: new Date().toISOString(),
      });
      return {
        formatVersion: 1,
        policy: 'interactive',
        through: 'suggest',
        targetKind: 'extractor',
        targetRef: 'extractor:whisper-transcription',
        outcome: 'produced',
        output: searchableText,
        classificationMatches: [{
          classifierRef: 'classifier:prose',
          classifierName: 'Prose',
          contentType: 'prose',
          priority: 200,
          startOffset: 0,
          endOffset: searchableText.length,
        }],
        failure: null,
        participants: [{ stableRef: 'extractor:whisper-transcription', pass: 'extract', outcome: 'produced' }],
        appliedClipId: clipId,
        ocrUpdated: false,
        searchableTextUpdated: true,
        classificationUpdated: true,
      } as unknown as T;
    }
    case 'create_content_extractor_recipe': {
      const input = args?.input as { name: string; description: string; enabled: boolean; priority: number; recipe: MockExtractorRecipe };
      const id = nextMockExtractorId++;
      const created: MockExtractor = { id, stableRef: `extractor:custom:${id}`, name: input.name, description: input.description, engine: 'recipe-v1', executablePath: input.recipe.steps[0]?.executable.path ?? null, modelPath: null, inputContract: input.recipe.accepts[0] ?? 'image', outputContract: 'searchable_text', enabled: input.enabled, priority: input.priority, recipe: structuredClone(input.recipe), recipeHash: `mock-${id}`, defaultRecipe: null, revision: 1, runtime: { method: 'recipe', location: input.recipe.steps[0]?.executable.path ?? null, version: null, usesAutomaticDiscovery: !input.recipe.steps[0]?.executable.path, dependencies: [] }, isBuiltin: false, isAvailable: true, unavailableReason: null, defaults: null };
      mockExtractors = [...mockExtractors, created];
      return created as unknown as T;
    }
    case 'update_content_extractor_recipe': {
      const id = Number(args?.id);
      const input = args?.input as { name: string; description: string; enabled: boolean; priority: number; recipe: MockExtractorRecipe };
      mockExtractors = mockExtractors.map((extractor) => extractor.id === id ? { ...extractor, ...input, recipe: structuredClone(input.recipe), recipeHash: `mock-${id}-${extractor.revision + 1}`, revision: extractor.revision + 1 } : extractor);
      return mockExtractors.find((extractor) => extractor.id === id) as unknown as T;
    }
    case 'get_extractor_authoring_sessions':
      return [] as unknown as T;
    case 'get_clip_extraction_results':
      return [] as unknown as T;
    case 'get_clip_extraction_history':
      return [] as unknown as T;
    case 'propose_extractor_recipe': {
      const recipe = mockExtractorRecipe('file_references', 'pdftotext');
      return { name: 'PDF Text', description: 'Extracts searchable text from PDF files.', recipe, setupGuidance: ['Install Poppler.'], authoring: { manifestVersion: 1, source: 'ai', originalPrompt: String((args?.request as { prompt?: unknown } | undefined)?.prompt ?? ''), provider: 'Mock AI', model: 'mock', messages: [] }, connectionId: 'mock', connectionName: 'Mock AI', durationMs: 1 } as unknown as T;
    }
    case 'repair_extractor_recipe': {
      const request = args?.request as { name: string; description: string; recipe: MockExtractorRecipe; prompt?: string | null };
      const recipe = structuredClone(request.recipe);
      const issues = recipe.resources.filter((resource) => resource.required && !resource.path)
        .map((resource) => ({ code: 'resource_not_configured', subjectId: resource.id, label: resource.label, detail: 'A required resource is not configured.' }));
      return { name: request.name, description: request.description, recipe, setupGuidance: issues.length > 0 ? ['Choose the required local resource, then diagnose again.'] : [], authoring: { manifestVersion: 1, source: 'ai', originalPrompt: request.prompt ?? null, provider: 'Mock AI', model: 'mock', messages: [] }, diagnostic: { version: 1, isAvailable: issues.length === 0, platform: 'browser', architecture: 'mock', packageManagers: [], issues }, status: issues.length === 0 ? 'ready' : 'setup_required', attempts: 1, connectionId: 'mock', connectionName: 'Mock AI', durationMs: 1 } as unknown as T;
    }
    case 'duplicate_content_extractor': {
      const reference = String(args?.reference ?? '');
      const source = mockExtractors.find((extractor) => extractor.stableRef === reference || String(extractor.id) === reference);
      if (!source) throw new Error('Extractor was not found.');
      const id = nextMockExtractorId++;
      const created = { ...source, id, stableRef: `extractor:custom:${id}`, name: String(args?.name ?? `${source.name} Copy`), priority: source.priority + 1, revision: 1, isBuiltin: false, defaults: null };
      mockExtractors = [...mockExtractors, created];
      return created as unknown as T;
    }
    case 'delete_content_extractor':
      mockExtractors = mockExtractors.filter((extractor) => extractor.id !== Number(args?.id));
      return undefined as T;
    case 'restore_default_content_extractors': {
      const builtins = mockBuiltinExtractors();
      const builtinRefs = new Set(builtins.map(({ stableRef }) => stableRef));
      mockExtractors = [
        ...builtins,
        ...mockExtractors.filter((extractor) => !builtinRefs.has(extractor.stableRef)),
      ];
      return mockExtractors as unknown as T;
    }
    case 'get_library_items': {
      const kind = String(args?.kind ?? '');
      const items = [
        { stableRef: 'capture:clip-type-v1', kind: 'capture', name: 'Clip Type', description: 'Assigns exactly one structural Text, Image, or Files type from the captured clipboard representation.', groupLabel: 'Capture', icon: 'Shapes', enabled: null, isBuiltin: true, isArchived: false, sortOrder: 0, revision: 1, inputContract: 'clipboard_representation', outputContract: 'clip_type', analysisPass: null, typeRelations: [], createdAt: '', updatedAt: '', capabilities: { canEdit: false, canDuplicate: false, canDelete: false, canDisable: false, canRestore: false } },
        { stableRef: 'capture:source-attribution-v1', kind: 'capture', name: 'Source Attribution', description: 'Records the application associated with a clipboard capture and resolves its icon when shown.', groupLabel: 'Capture', icon: 'AppWindow', enabled: null, isBuiltin: true, isArchived: false, sortOrder: 10, revision: 1, inputContract: 'clipboard_event', outputContract: 'source_attribution', analysisPass: null, typeRelations: [], createdAt: '', updatedAt: '', capabilities: { canEdit: false, canDuplicate: false, canDelete: false, canDisable: false, canRestore: false } },
        { stableRef: 'inspector:structure-v1', kind: 'inspector', name: 'Structure', description: 'Measures stable clip structure without retaining clipboard contents.', groupLabel: 'Content Analysis', icon: 'ScanSearch', enabled: null, isBuiltin: true, isArchived: false, sortOrder: 0, revision: 1, inputContract: 'clip', outputContract: 'structural_metadata', analysisPass: 'inspect', participantContract: { stableRef: 'inspector:structure-v1', name: 'Structure', pass: 'inspect', priority: 0, requires: ['clip_kind'], provides: ['structural_metadata'] }, typeRelations: [], createdAt: '', updatedAt: '', capabilities: { canEdit: false, canDuplicate: false, canDelete: false, canDisable: false, canRestore: false } },
        { stableRef: 'inspector:media-metadata-v1', kind: 'inspector', name: 'Media Metadata', description: 'Reads bounded audio and video metadata locally.', groupLabel: 'Content Analysis', icon: 'FileAudio', enabled: null, isBuiltin: true, isArchived: false, sortOrder: 10, revision: 1, inputContract: 'file_references', outputContract: 'media_metadata', analysisPass: 'inspect', participantContract: { stableRef: 'inspector:media-metadata-v1', name: 'Media Metadata', pass: 'inspect', priority: 10, requires: ['file_references'], provides: ['media_metadata'] }, typeRelations: [{ kind: 'accepts', typeId: 'file' }], createdAt: '', updatedAt: '', capabilities: { canEdit: false, canDuplicate: false, canDelete: false, canDisable: false, canRestore: false } },
        { stableRef: 'suggestion:smart-actions-v1', kind: 'suggestion', name: 'Smart Actions', description: 'Suggests saved Transforms from content-free analysis signals.', groupLabel: 'Content Analysis', icon: 'Lightbulb', enabled: null, isBuiltin: true, isArchived: false, sortOrder: 0, revision: 1, inputContract: 'analyzable_text+structural_metadata', outputContract: 'suggestions', analysisPass: 'suggest', participantContract: { stableRef: 'suggestion:smart-actions-v1', name: 'Smart Actions', pass: 'suggest', priority: 0, requires: ['analyzable_text', 'structural_metadata'], provides: ['suggestions'] }, typeRelations: [], createdAt: '', updatedAt: '', capabilities: { canEdit: false, canDuplicate: false, canDelete: false, canDisable: false, canRestore: false } },
        ...mockExtractors.map((extractor) => ({ stableRef: extractor.stableRef, kind: 'extractor', name: extractor.name, description: extractor.description, groupLabel: 'Content Analysis', icon: 'ScanText', enabled: extractor.enabled, isBuiltin: extractor.isBuiltin, isArchived: false, sortOrder: extractor.priority, revision: 1, inputContract: extractor.inputContract, outputContract: extractor.outputContract, analysisPass: 'extract', participantContract: { stableRef: extractor.stableRef, name: extractor.name, pass: 'extract', priority: extractor.priority, requires: [extractor.inputContract], provides: [extractor.outputContract, 'analyzable_text'] }, typeRelations: extractor.inputContract === 'image' ? [{ kind: 'accepts', typeId: 'image' }] : extractor.inputContract === 'file_references' ? [{ kind: 'accepts', typeId: 'file' }] : [], createdAt: '', updatedAt: '', capabilities: { canEdit: true, canDuplicate: true, canDelete: true, canDisable: true, canRestore: extractor.isBuiltin } })),
        ...mockClassifiers.map((classifier) => ({ stableRef: classifier.stable_ref, kind: 'classifier', name: classifier.name, description: classifier.description, groupLabel: null, icon: 'FileText', enabled: classifier.enabled, isBuiltin: classifier.is_builtin, isArchived: false, sortOrder: classifier.priority, revision: 1, inputContract: 'text', outputContract: `set_type:${classifier.content_type}`, analysisPass: 'classify', participantContract: { stableRef: classifier.stable_ref, name: classifier.name, pass: 'classify', priority: classifier.priority, requires: ['analyzable_text'], provides: ['classification'] }, typeRelations: [{ kind: 'classifies_as', typeId: classifier.content_type }], createdAt: '', updatedAt: '', capabilities: { canEdit: true, canDuplicate: true, canDelete: true, canDisable: true, canRestore: classifier.is_builtin } })),
        ...mockManualTransforms.map((manualTransform) => ({ stableRef: manualTransform.stableRef, kind: 'transform', name: manualTransform.name, description: '', groupLabel: 'Manual Transforms', icon: 'Workflow', enabled: null, isBuiltin: false, isArchived: false, sortOrder: manualTransform.id, revision: manualTransform.revision, inputContract: 'text', outputContract: 'preserve_type', analysisPass: null, createdAt: manualTransform.createdAt, updatedAt: manualTransform.updatedAt, capabilities: { canEdit: true, canDuplicate: true, canDelete: true, canDisable: false, canRestore: false } })),
      ];
      return items.filter((item) => !kind || item.kind === kind) as unknown as T;
    }
    case 'set_library_item_enabled': {
      const kind = String(args?.kind ?? '');
      const stableRef = String(args?.stableRef ?? '');
      const enabled = Boolean(args?.enabled);
      let matched = false;
      if (kind === 'extractor') {
        mockExtractors = mockExtractors.map((extractor) => {
          if (extractor.stableRef !== stableRef) return extractor;
          matched = true;
          return { ...extractor, enabled };
        });
      } else if (kind === 'classifier') {
        mockClassifiers = mockClassifiers.map((classifier) => {
          if (classifier.stable_ref !== stableRef) return classifier;
          matched = true;
          return { ...classifier, enabled };
        });
      } else if (kind === 'capture' || kind === 'inspector' || kind === 'suggestion') {
        throw new Error('Built-in lifecycle capabilities cannot be disabled.');
      } else if (kind !== 'operation') {
        throw new Error('Unknown library item kind.');
      }
      if (!matched) throw new Error('Library item was not found.');
      return undefined as T;
    }
    case 'get_content_types':
      return mockContentTypes
        .filter((type) => Boolean(args?.includeArchived) || !type.isArchived)
        .map((type) => ({ ...type })) as unknown as T;
    case 'get_content_type_groups':
      return mockContentTypeGroups.filter((group) => Boolean(args?.includeArchived) || !group.isArchived).map((group) => ({ ...group })) as unknown as T;
    case 'create_content_type_group': {
      const input = args?.input as { id: string; label: string; sortOrder: number };
      const created = { ...input, isBuiltin: false, isArchived: false, defaults: null };
      mockContentTypeGroups.push(created);
      return created as unknown as T;
    }
    case 'update_content_type_group': {
      const index = mockContentTypeGroups.findIndex(({ id }) => id === String(args?.id));
      if (index >= 0) mockContentTypeGroups[index] = { ...mockContentTypeGroups[index], ...(args?.input as Record<string, string | number>) };
      return mockContentTypeGroups[index] as unknown as T;
    }
    case 'set_content_type_group_archived': {
      const index = mockContentTypeGroups.findIndex(({ id }) => id === String(args?.id));
      if (index >= 0) mockContentTypeGroups[index] = { ...mockContentTypeGroups[index], isArchived: Boolean(args?.archived) };
      return null as unknown as T;
    }
    case 'delete_content_type_group':
      mockContentTypeGroups = mockContentTypeGroups.filter(({ id }) => id !== String(args?.id));
      return null as unknown as T;
    case 'restore_default_content_type_groups':
      return mockContentTypeGroups as unknown as T;
    case 'create_content_type': {
      const input = args?.input as { id: string; label: string; icon: string; group: string; concealClips: boolean };
      const created = { ...input, isBuiltin: false, isArchived: false, defaults: null };
      mockContentTypes.push(created);
      return created as unknown as T;
    }
    case 'update_content_type': {
      const index = mockContentTypes.findIndex(({ id }) => id === String(args?.id));
      if (index >= 0) mockContentTypes[index] = { ...mockContentTypes[index], ...(args?.input as Record<string, string | boolean>) };
      return mockContentTypes[index] as unknown as T;
    }
    case 'set_content_type_archived': {
      const index = mockContentTypes.findIndex(({ id }) => id === String(args?.id));
      if (index >= 0) mockContentTypes[index] = { ...mockContentTypes[index], isArchived: Boolean(args?.archived) };
      return mockContentTypes[index] as unknown as T;
    }
    case 'restore_default_content_types':
      return mockContentTypes as unknown as T;
    case 'create_content_classifier': {
      const input = args?.input as Record<string, unknown>;
      const classifier = { ...input, id: Math.max(0, ...mockClassifiers.map(({ id }) => Number(id))) + 1, stable_ref: `custom-${Date.now()}`, is_builtin: false, defaults: null } as unknown as typeof mockClassifiers[number];
      mockClassifiers.push(classifier);
      return classifier as unknown as T;
    }
    case 'update_content_classifier': {
      const index = mockClassifiers.findIndex(({ id }) => id === Number(args?.id));
      if (index >= 0) mockClassifiers[index] = { ...mockClassifiers[index], ...(args?.input as Record<string, unknown>) } as typeof mockClassifiers[number];
      return mockClassifiers[index] as unknown as T;
    }
    case 'duplicate_content_classifier': {
      const reference = String(args?.reference ?? '');
      const source = mockClassifiers.find((classifier) => classifier.stable_ref === reference || String(classifier.id) === reference);
      if (!source) throw new Error('Classifier was not found.');
      const classifier = { ...source, id: Math.max(0, ...mockClassifiers.map(({ id }) => Number(id))) + 1, stable_ref: `custom-${Date.now()}`, name: String(args?.name ?? `${source.name} Copy`), priority: source.priority + 1, is_builtin: false, defaults: null } as unknown as typeof mockClassifiers[number];
      mockClassifiers.push(classifier);
      return classifier as unknown as T;
    }
    case 'delete_content_classifier':
      mockClassifiers = mockClassifiers.filter(({ id }) => id !== Number(args?.id));
      return null as unknown as T;
    case 'restore_default_content_classifiers':
      return mockClassifiers as unknown as T;
    case 'rescan_content_classification_history':
      return { scannedCount: mockClips.length, changedCount: 0, unchangedCount: mockClips.length, failedCount: 0 } as unknown as T;
    case 'rescan_file_format_history': {
      const fileCount = mockClips.filter(({ content_type }) => content_type === 'file').length;
      return { scannedCount: fileCount, changedCount: 0, unchangedCount: fileCount, missingCount: 0, failedCount: 0 } as unknown as T;
    }
    case 'test_content_classifier': {
      const input = args?.input as { patterns?: string[] };
      const sample = String(args?.sample ?? '');
      const matched = Boolean(input.patterns?.some((pattern) => {
        try { return new RegExp(pattern.replace(/^\(\?i\)/, ''), pattern.startsWith('(?i)') ? 'i' : '').test(sample); } catch { return false; }
      }));
      return {
        formatVersion: 1,
        policy: 'interactive',
        through: 'suggest',
        targetKind: 'classifier',
        targetRef: 'preview',
        outcome: matched ? 'matched' : 'no_match',
        matched,
        contentTypes: matched ? [String((args?.input as { content_type?: string })?.content_type ?? 'text')] : [],
        matches: matched ? [{
          classifierRef: 'preview',
          classifierName: String((args?.input as { name?: string })?.name ?? 'Preview'),
          contentType: String((args?.input as { content_type?: string })?.content_type ?? 'text'),
          priority: Number((args?.input as { priority?: number })?.priority ?? 100),
          startOffset: 0,
          endOffset: sample.length,
        }] : [],
        failure: null,
        participants: [{ stableRef: 'analysis:content-classifiers', pass: 'classify', outcome: 'produced' }],
      } as unknown as T;
    }
  }
  return unhandledValue;
}
