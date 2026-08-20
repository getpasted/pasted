import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { CONTENT_TYPES } from './contentTypes';
import { getClipFilePaths, getClipOriginKind } from '../types';
import { clipMatchesSearch, parseClipSearch } from './clipSearch';
import { handleActivityBrowserMock } from '../mocks/browser/activity';
import { handleBackupBrowserMock } from '../mocks/browser/backup';
import { handleAnalyticsBrowserMock } from '../mocks/browser/analytics';
import { handleClipBrowserMock } from '../mocks/browser/clips';
import { handleBinBrowserMock } from '../mocks/browser/bins';
import { handleAnalysisBrowserMock } from '../mocks/browser/analysis';
import { handleQueueBrowserMock } from '../mocks/browser/queue';
import { handleAppStateBrowserMock } from '../mocks/browser/appState';
import {
  handleManualTransformBrowserMock,
  mockManualTransforms,
} from '../mocks/browser/manualTransforms';

type MockClip = {
  id: number;
  text_content: string;
  content_type: string;
  content_types?: string[];
  file_formats?: string[];
  source: string;
  created_at: string;
  char_count: number;
  word_count: number;
  line_count: number;
  is_pinned: number;
  is_protected: number;
  is_explicitly_protected?: boolean;
  protecting_bin_ids?: number[];
  hotkey?: string | null;
  is_transformed?: number;
  pin_order: number;
  is_trashed: number;
  trashed_at?: string | null;
  bin_id: number | null;
  bin_ids: number[];
  note?: string | null;
};

type MockBin = {
  id: number;
  name: string;
  icon: string;
  color: string;
  smart_rule: string | null;
  bin_type: string;
  clip_order?: number[];
  protect_clips?: boolean;
  hotkey?: string | null;
};

let mockClips: MockClip[] = [
  {
    id: 101,
    text_content: 'Sample Clip 1 for Drag Testing',
    content_type: 'text',
    source: 'Safari',
    created_at: new Date().toISOString(),
    char_count: 30,
    word_count: 6,
    line_count: 1,
    is_pinned: 0,
    is_protected: 0,
    pin_order: 0,
    is_trashed: 0,
    bin_id: null,
    bin_ids: [],
  },
  {
    id: 102,
    text_content: 'Sample Clip 2 for Drag Testing',
    content_type: 'text',
    source: 'VS Code',
    created_at: new Date().toISOString(),
    char_count: 30,
    word_count: 6,
    line_count: 1,
    is_pinned: 0,
    is_protected: 0,
    pin_order: 0,
    is_trashed: 0,
    bin_id: null,
    bin_ids: [],
  },
];

let mockBins: MockBin[] = [
  { id: 1, name: 'My Manual Bin', icon: '📂', color: 'default', smart_rule: null, bin_type: 'category' },
  { id: 2, name: 'Work Bin', icon: '💼', color: '#10b981', smart_rule: '', bin_type: 'category' },
];

function withMockProtection(clip: MockClip) {
  const protectingBinIds = clip.bin_ids.filter((id) => (
    mockBins.find((bin) => bin.id === id)?.protect_clips
  ));
  const explicitlyProtected = Boolean(clip.is_protected);
  return {
    ...clip,
    is_explicitly_protected: explicitlyProtected,
    is_protected: explicitlyProtected || Boolean(clip.hotkey) || protectingBinIds.length > 0,
    protecting_bin_ids: protectingBinIds,
  };
}

function mockClassifier<T extends { id: number; patterns: string[] }>(classifier: T) {
  return { ...classifier, defaults: { ...classifier, patterns: [...classifier.patterns] } };
}
let mockClassifiers = [
  mockClassifier({ id: 1, stable_ref: 'email', name: 'Email Addresses', content_type: 'email', description: 'Individual email addresses', patterns: [String.raw`(?i)^[^\s@]+@[^\s@]+\.[^\s@]+$`], validator: null, enabled: true, priority: 30, is_builtin: true }),
  mockClassifier({ id: 2, stable_ref: 'credential', name: 'Credentials', content_type: 'credential', description: 'Known API-key formats and secret assignments', patterns: [String.raw`^(?:sk_|ghp_).+$`], validator: null, enabled: true, priority: 60, is_builtin: true }),
  mockClassifier({ id: 3, stable_ref: 'phone', name: 'Phone Numbers', content_type: 'phone', description: 'Formatted international and local phone numbers', patterns: [String.raw`^\+?[0-9 ()-]{7,}$`], validator: 'phone', enabled: true, priority: 160, is_builtin: true }),
];
const mockAppleExtractorDefaults = { name: 'Apple Vision OCR', description: 'Extracts searchable text from images locally with Apple Vision.', engine: 'recipe-v1', executablePath: null, modelPath: null, inputContract: 'image', outputContract: 'searchable_text', enabled: true, priority: 10 };
const mockTesseractExtractorDefaults = { name: 'Tesseract OCR', description: 'Extracts searchable text from images locally with Tesseract.', engine: 'recipe-v1', executablePath: null, modelPath: null, inputContract: 'image', outputContract: 'searchable_text', enabled: true, priority: 20 };
const mockWhisperExtractorDefaults = { name: 'Whisper Transcription', description: 'Extracts searchable text from local audio files with whisper.cpp.', engine: 'recipe-v1', executablePath: null, modelPath: null, inputContract: 'file_references', outputContract: 'searchable_text', enabled: true, priority: 30 };
type MockExtractorRecipe = {
  definitionVersion: 1;
  accepts: Array<'image' | 'file_references'>;
  output: 'searchable_text';
  steps: Array<{ id: string; executable: { path: string | null; discover: string[]; versionArguments: string[] }; arguments: string[]; mode: 'once' | 'each_input'; capture: 'ignore' | 'stdout_text' | 'file_text' | 'pasted_json_v1'; outputExtension: string | null; timeoutSeconds: number }>;
  resources: Array<{ id: string; label: string; kind: 'file' | 'directory'; required: boolean; path: string | null }>;
};
const mockExtractorRecipe = (input: 'image' | 'file_references', command: string): MockExtractorRecipe => ({
  definitionVersion: 1,
  accepts: [input],
  output: 'searchable_text',
  steps: [{ id: 'extract', executable: { path: null, discover: [command], versionArguments: ['--version'] }, arguments: ['{input.path}'], mode: 'once', capture: 'stdout_text', outputExtension: null, timeoutSeconds: 60 }],
  resources: [],
});
type MockExtractor = {
  id: number; stableRef: string; name: string; description: string; engine: string;
  executablePath: string | null; modelPath: string | null; revision: number;
  inputContract: string; outputContract: string; enabled: boolean; priority: number;
  isBuiltin: boolean; isAvailable: boolean; unavailableReason: string | null;
  runtime: { method: string; location: string | null; version: string | null; usesAutomaticDiscovery: boolean; dependencies: Array<{ name: string; location: string | null; version: string | null; isAvailable: boolean; unavailableReason: string | null }> };
  recipe: MockExtractorRecipe; recipeHash: string; defaultRecipe: MockExtractorRecipe | null;
  defaults: typeof mockAppleExtractorDefaults | null;
};
function mockBuiltinExtractors(): MockExtractor[] {
  return [{
    id: 1,
    stableRef: 'extractor:apple-vision-ocr',
    ...mockAppleExtractorDefaults,
    revision: 1,
    runtime: { method: 'system', location: 'macOS Vision framework', version: null, usesAutomaticDiscovery: false, dependencies: [] },
    isBuiltin: true,
    isAvailable: true,
    unavailableReason: null,
    recipe: mockExtractorRecipe('image', 'pasted-bundled-extractor'), recipeHash: 'mock-apple', defaultRecipe: mockExtractorRecipe('image', 'pasted-bundled-extractor'), defaults: { ...mockAppleExtractorDefaults },
  }, {
    id: 2,
    stableRef: 'extractor:tesseract-ocr',
    ...mockTesseractExtractorDefaults,
    revision: 1,
    runtime: { method: 'command', location: null, version: null, usesAutomaticDiscovery: true, dependencies: [] },
    isBuiltin: true,
    isAvailable: false,
    unavailableReason: 'Tesseract OCR is not installed. Install Tesseract 5, then check again.',
    recipe: mockExtractorRecipe('image', 'tesseract'), recipeHash: 'mock-tesseract', defaultRecipe: mockExtractorRecipe('image', 'tesseract'), defaults: { ...mockTesseractExtractorDefaults },
  }, {
    id: 3,
    stableRef: 'extractor:whisper-transcription',
    ...mockWhisperExtractorDefaults,
    revision: 1,
    runtime: { method: 'command', location: null, version: null, usesAutomaticDiscovery: true, dependencies: [{ name: 'FFmpeg', location: '/mock/bin/ffmpeg', version: 'ffmpeg mock', isAvailable: true, unavailableReason: null }] },
    isBuiltin: true,
    isAvailable: false,
    unavailableReason: 'Whisper.cpp is not installed. Install whisper-cpp, then check again.',
    recipe: mockExtractorRecipe('file_references', 'whisper-cli'), recipeHash: 'mock-whisper', defaultRecipe: mockExtractorRecipe('file_references', 'whisper-cli'), defaults: { ...mockWhisperExtractorDefaults },
  }];
}
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
  id: string; label: string; icon: string; group: string; isBuiltin: boolean; isArchived: boolean;
  defaults: { label: string; icon: string; group: string } | null;
}> = CONTENT_TYPES.map(({ value, label, icon, group }) => {
  const groupId = group.toLowerCase().replace(/ & /g, '_').replace(/ /g, '_');
  return { id: value as string, label, icon, group: groupId, isBuiltin: true, isArchived: false, defaults: { label, icon, group: groupId } };
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

let mockLibraryLocation = {
  path: '/mock/Pasted/pasted.db',
  directory: '/mock/Pasted',
  isDefault: true,
};

let mockIntelligenceConnections: Array<{
  id: string;
  name: string;
  providerKind: string;
  endpoint: string | null;
  model: string | null;
  credentialRef: string | null;
  enabled: boolean;
  priority: number;
  createdAt: string;
  updatedAt: string;
}> = [];

let mockSavedTransforms: Array<Record<string, unknown>> = [];
let mockClipTransformations = new Map<number, Record<string, unknown>>();
let mockBinTransforms = new Map<number, string>();
function assignMockClips(ids: number[], binId: number | null) {
  for (const clip of mockClips) {
    if (!ids.includes(clip.id) || clip.is_trashed !== 0) continue;
    clip.bin_id = binId;
    const tagIds = clip.bin_ids.filter((id) => mockBins.find((bin) => bin.id === id)?.bin_type === 'tag');
    clip.bin_ids = binId === null ? tagIds : [...tagIds, binId];
  }
}

function mockSmartActionSuggestions(text: string) {
  const signals: string[] = [];
  if (/https?:\/\/[^\s]+/i.test(text)) signals.push('url');
  let isJson = false;
  try {
    const trimmed = text.trim();
    if ((trimmed.startsWith('{') && trimmed.endsWith('}')) || (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
      JSON.parse(trimmed);
      isJson = true;
      signals.push('json');
    }
  } catch {}
  const hasHtml = /<[a-z][^>]*>.*<\/[a-z][^>]*>|<[a-z][^>]*\/?>/is.test(text);
  if (hasHtml && !isJson) signals.push('html');
  if (/(^|\s)(#{1,6}\s|\*\*|__|```|\[[^\]]+\]\([^\)]+\))/m.test(text) && !hasHtml && !isJson) signals.push('markdown');
  const lineCount = text.length === 0 ? 0 : text.split(/\r?\n/).length - (/\r?\n$/.test(text) ? 1 : 0);
  if (lineCount > 1) signals.push('multi_line');
  if (/[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}/i.test(text)) signals.push('email');
  if (/(?:^|[^0-9])(?:\+?[0-9]{1,3}[-. ]?)?\(?[0-9]{3}\)?[-. ]?[0-9]{3}[-. ]?[0-9]{4}(?:$|[^0-9])/.test(text)) signals.push('phone');
  const signalPatterns: Record<string, RegExp> = {
    url: /url|link|tracking|clean_url_tracking|extract_urls/,
    json: /json|json_format|json_minify/,
    html: /html|markup|tag|strip_html|wrap_tags/,
    markdown: /markdown|strip_markdown/,
    multi_line: /line|list|sort|dedupe|sort_lines|dedupe_lines/,
    email: /email|extract_emails/,
    phone: /phone|extract_phones/,
  };
  const actions = mockManualTransforms
    .slice(0, 256)
    .flatMap((manualTransform) => {
      const searchable = `${manualTransform.name} ${manualTransform.steps.map((step) => step.operationRef).join(' ')}`.toLowerCase();
      const reasons = signals.filter((signal) => signalPatterns[signal]?.test(searchable));
      return reasons.length ? [{
        transformRef: manualTransform.stableRef,
        transformName: manualTransform.name,
        transformRevision: manualTransform.revision,
        reasons,
      }] : [];
    })
    .slice(0, 12);
  const labels: Record<string, string> = { url: 'URL Link', json: 'JSON Data', html: 'HTML Markup', markdown: 'Markdown Text', multi_line: 'Multiple Lines', email: 'Email Address', phone: 'Phone Number' };
  return { signals, signalLabels: signals.map((signal) => labels[signal]), actions };
}

export async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
    return tauriInvoke<T>(cmd, args);
  }
  console.warn(`[safeInvoke mock] ${cmd}`);
  for (const result of [
    handleActivityBrowserMock(cmd),
    handleBackupBrowserMock(cmd),
    handleAnalyticsBrowserMock(cmd, mockClips),
    handleClipBrowserMock(cmd, args, mockClips, withMockProtection),
    handleBinBrowserMock(cmd, mockBins, mockClips),
    handleAnalysisBrowserMock(cmd),
    handleQueueBrowserMock(cmd, args),
    handleAppStateBrowserMock(cmd, args),
    handleManualTransformBrowserMock(cmd, args),
  ]) {
    if (result.matched) return result.value as T;
  }
  switch (cmd) {
    case 'search_clips': {
      const request = (args?.request ?? {}) as Record<string, unknown>;
      const query = String(request.query ?? '');
      const plan = parseClipSearch(query);
      const appendFilters = (target: string[], value: unknown) => {
        if (Array.isArray(value)) target.push(...value.filter((item): item is string => typeof item === 'string').map((item) => item.toLowerCase()));
      };
      appendFilters(plan.clipTypes, request.clipTypes);
      appendFilters(plan.contentTypes, request.contentTypes);
      appendFilters(plan.formats, request.fileFormats);
      appendFilters(plan.sources, request.sources);
      if (request.trash === true) plan.requiresTrashed = true;
      const offset = Math.max(0, Number(request.offset ?? 0));
      const limit = Math.min(500, Math.max(1, Number(request.limit ?? 100)));
      const items = mockClips.filter((clip) => {
        if (Boolean(clip.is_trashed) !== plan.requiresTrashed) return false;
        const searchableText = mockFileSearchableText.get(clip.id)?.searchableText;
        const protectedClip = withMockProtection(clip);
        const candidate = searchableText
          ? { ...protectedClip, text_content: `${clip.text_content}\n${searchableText}` }
          : protectedClip;
        return clipMatchesSearch(candidate as unknown as import('../types').ClipItem, plan);
      });
      return {
        items: items.slice(offset, offset + limit).map((clip) => ({
          ...withMockProtection(clip),
          content_types: [...(clip.content_types ?? [])],
          file_formats: [...(clip.file_formats ?? [])],
          bin_ids: [...clip.bin_ids],
        })),
        totalCount: items.length,
        limit,
        offset,
      } as unknown as T;
    }
    case 'get_content_classifiers':
      return mockClassifiers.map((classifier) => ({ ...classifier, patterns: [...classifier.patterns] })) as unknown as T;
    case 'get_content_extractors':
      return mockExtractors.map((extractor) => ({ ...extractor, recipe: structuredClone(extractor.recipe), defaultRecipe: extractor.defaultRecipe ? structuredClone(extractor.defaultRecipe) : null, defaults: extractor.defaults ? { ...extractor.defaults } : null })) as unknown as T;
    case 'choose_extractor_executable':
      return '/mock/bin/custom-extractor' as unknown as T;
    case 'choose_extractor_resource_file':
      return '/mock/input/sample.dat' as unknown as T;
    case 'test_content_extractor_recipe':
      return { outcome: 'produced', text: 'Mock extracted text' } as unknown as T;
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
      const input = args?.input as { id: string; label: string; icon: string; group: string };
      const created = { ...input, isBuiltin: false, isArchived: false, defaults: null };
      mockContentTypes.push(created);
      return created as unknown as T;
    }
    case 'update_content_type': {
      const index = mockContentTypes.findIndex(({ id }) => id === String(args?.id));
      if (index >= 0) mockContentTypes[index] = { ...mockContentTypes[index], ...(args?.input as Record<string, string>) };
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
    case 'get_hotkey_capability_status':
      return {
        platform: 'unsupported',
        backend: 'unsupported',
        state: 'unavailable',
        is_trusted: true,
        is_dev_mode: false,
        configured_count: 0,
        registered_count: 0,
        issues: [],
      } as unknown as T;
    case 'get_bin_transform_ref':
      return (mockBinTransforms.get(Number(args?.binId)) || null) as unknown as T;
    case 'set_bin_transform_ref': {
      const binId = Number(args?.binId);
      if (args?.transformRef) mockBinTransforms.set(binId, String(args.transformRef));
      else mockBinTransforms.delete(binId);
      return null as unknown as T;
    }
    case 'get_intelligence_connections':
      return [...mockIntelligenceConnections]
        .sort((left, right) => left.priority - right.priority)
        .map((connection) => ({ ...connection })) as unknown as T;
    case 'detect_intelligence_connections': {
      const detected = [
        { adapterId: 'codex_cli', name: 'Codex CLI', providerKind: 'cli', executablePath: '/opt/homebrew/bin/codex', defaultEndpoint: null, version: 'codex-cli', capabilities: ['structured_output', 'json_events', 'local_models'], executionSupported: true },
        { adapterId: 'claude_cli', name: 'Claude CLI', providerKind: 'cli', executablePath: '/opt/homebrew/bin/claude', defaultEndpoint: null, version: 'claude', capabilities: ['non_interactive', 'structured_output'], executionSupported: false },
        { adapterId: 'ollama', name: 'Ollama', providerKind: 'ollama', executablePath: '/opt/homebrew/bin/ollama', defaultEndpoint: 'http://127.0.0.1:11434', version: 'ollama', capabilities: ['local', 'openai_compatible'], executionSupported: false },
        { adapterId: 'antigravity_ide', name: 'Antigravity IDE', providerKind: 'cli', executablePath: '/Applications/Antigravity IDE.app/Contents/Resources/app/bin/antigravity-ide', defaultEndpoint: null, version: 'Antigravity IDE', capabilities: ['interactive_chat', 'mcp_client'], executionSupported: false },
      ];
      for (const candidate of detected) {
        const endpoint = candidate.providerKind === 'cli' ? candidate.executablePath : candidate.defaultEndpoint;
        if (mockIntelligenceConnections.some((connection) => connection.providerKind === candidate.providerKind && connection.endpoint === endpoint)) continue;
        const now = new Date().toISOString();
        mockIntelligenceConnections.push({
          id: `mock-detected-${candidate.adapterId}`,
          name: candidate.name,
          providerKind: candidate.providerKind,
          endpoint,
          model: null,
          credentialRef: null,
          enabled: false,
          priority: mockIntelligenceConnections.length,
          createdAt: now,
          updatedAt: now,
        });
      }
      return detected as unknown as T;
    }
    case 'create_intelligence_connection': {
      const now = new Date().toISOString();
      const connection = {
        id: `mock-connection-${Date.now()}`,
        name: String(args?.name || 'AI Connection'),
        providerKind: String(args?.providerKind || 'ollama'),
        endpoint: typeof args?.endpoint === 'string' ? args.endpoint : null,
        model: typeof args?.model === 'string' ? args.model : null,
        credentialRef: typeof args?.credentialRef === 'string' ? args.credentialRef : null,
        enabled: true,
        priority: mockIntelligenceConnections.length,
        createdAt: now,
        updatedAt: now,
      };
      mockIntelligenceConnections.push(connection);
      return connection as unknown as T;
    }
    case 'update_intelligence_connection': {
      const connection = mockIntelligenceConnections.find((item) => item.id === args?.id);
      if (connection) {
        connection.name = String(args?.name || connection.name);
        connection.providerKind = String(args?.providerKind || connection.providerKind);
        connection.endpoint = typeof args?.endpoint === 'string' ? args.endpoint : null;
        connection.model = typeof args?.model === 'string' ? args.model : null;
        connection.credentialRef = typeof args?.credentialRef === 'string' ? args.credentialRef : null;
        connection.enabled = Boolean(args?.enabled);
        connection.updatedAt = new Date().toISOString();
      }
      return null as unknown as T;
    }
    case 'delete_intelligence_connection':
      mockIntelligenceConnections = mockIntelligenceConnections.filter((connection) => connection.id !== args?.id);
      return null as unknown as T;
    case 'reorder_intelligence_connections': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(String) : [];
      ids.forEach((id, priority) => {
        const connection = mockIntelligenceConnections.find((item) => item.id === id);
        if (connection) connection.priority = priority;
      });
      return null as unknown as T;
    }
    case 'execute_transformation': {
      const request = args?.request as { input?: string; target?: { kind?: string; transformRef?: string; operationRef?: string } } | undefined;
      const input = request?.input || '';
      const targetRef = request?.target?.transformRef || request?.target?.operationRef || '';
      const output = request?.target?.kind === 'transform'
        ? `# ${input}`
        : targetRef.includes('uppercase') ? input.toUpperCase() : input;
      return {
        executionId: 'mock-execution',
        output,
        connectionId: request?.target?.kind === 'transform' ? 'mock-detected-codex_cli' : null,
        connectionName: request?.target?.kind === 'transform' ? 'Codex CLI' : null,
        durationMs: request?.target?.kind === 'transform' ? 260 : 1,
      } as unknown as T;
    }
    case 'preview_manual_transform_steps': {
      const input = typeof args?.input === 'string' ? args.input : '';
      const steps = Array.isArray(args?.steps) ? args.steps : [];
      const output = steps.reduce((current: string, step: { operationRef?: string }) => (
        step.operationRef?.includes('uppercase') ? current.toUpperCase() : current
      ), input);
      return output as unknown as T;
    }
    case 'cancel_transformation_execution':
      return true as unknown as T;
    case 'get_intelligence_scheduler_snapshot':
      return {
        revision: 0,
        activeCount: 0,
        queuedCount: 0,
        jobs: [],
        recentEvents: [],
      } as unknown as T;
    case 'get_installation_diagnostics':
      return {
        appVersion: '1.0.0',
        buildKind: 'Development',
        platform: 'macos',
        architecture: 'aarch64',
        bundleIdentifier: 'software.jjj.pasted',
        appPath: '/Applications/Pasted.app',
        dataPath: '/Users/example/Library/Application Support/software.jjj.pasted',
        databaseSizeBytes: 2_457_600,
        signingStatus: 'Ad hoc',
        signingIdentity: null,
        signingTeamId: null,
        notarizationStatus: 'Not expected for development builds',
        cliPath: '/Applications/Pasted.app/Contents/MacOS/pasted',
      } as unknown as T;
    case 'get_third_party_licenses':
      return {
        schemaVersion: 1,
        componentCount: 2,
        components: [
          { ecosystem: 'cargo', name: 'tauri', version: '2.x', license: 'MIT OR Apache-2.0', repository: 'https://github.com/tauri-apps/tauri', noticeIds: ['development'] },
          { ecosystem: 'npm', name: 'react', version: '19.x', license: 'MIT', repository: 'https://github.com/facebook/react', noticeIds: ['development'] },
        ],
        noticeText: [
          'Pasted Third-Party Software Notices',
          '',
          'Development preview',
          '',
          'Production builds embed the complete generated component inventory and license text.',
          'Run `pasted licenses` or open this dialog in the native app to inspect that document.',
        ].join('\n'),
      } as unknown as T;
    case 'get_ocr_backfill_status':
      return {
        totalImages: 0,
        eligibleCount: 0,
        queuedCount: 0,
        runningCount: 0,
        completedCount: 0,
        noTextCount: 0,
        failedCount: 0,
      } as unknown as T;
    case 'start_ocr_backfill':
    case 'cancel_ocr_backfill':
      return undefined as T;
    case 'retry_failed_ocr':
      return 0 as unknown as T;
    case 'plan_transformation_intent': {
      const request = args?.request as { intent?: string; sampleInput?: string; planningMode?: string } | undefined;
      await new Promise((resolve) => window.setTimeout(resolve, 220));
      return {
        plan: {
          schema_version: 1,
          intent: request?.intent || '',
          summary: 'Apply the requested transformation with semantic judgment.',
          planning_mode: request?.planningMode || 'pinned',
          steps: [{
            name: 'Transform text',
            rationale: 'The requested outcome depends on language and context.',
            scope: 'whole_input',
            executor: {
              kind: 'semantic',
              instructions: request?.intent || 'Transform the text as requested.',
              model_policy: 'balanced',
            },
          }],
        },
        connectionId: 'mock-detected-codex_cli',
        connectionName: 'Codex CLI',
        durationMs: 220,
      } as unknown as T;
    }
    case 'test_transformation_plan': {
      const request = args?.request as { input?: string; plan?: { intent?: string } } | undefined;
      await new Promise((resolve) => window.setTimeout(resolve, 260));
      return {
        output: `# ${request?.input || 'Transformed text'}`,
        connectionId: 'mock-detected-codex_cli',
        connectionName: 'Codex CLI',
        durationMs: 260,
      } as unknown as T;
    }
    case 'get_intent_transforms':
      return mockSavedTransforms.map((transform) => ({ ...transform })) as unknown as T;
    case 'save_saved_transform': {
      const now = new Date().toISOString();
      const plan = args?.plan as { summary?: string } | undefined;
      const transform = {
        id: Date.now(),
        stableRef: `transform:mock-${Date.now()}`,
        name: String(args?.name || plan?.summary || 'Untitled Transform'),
        plan,
        connectionId: args?.connectionId || null,
        revision: 1,
        createdAt: now,
        updatedAt: now,
      };
      mockSavedTransforms.unshift(transform);
      return transform as unknown as T;
    }
    case 'update_saved_transform': {
      const index = mockSavedTransforms.findIndex((transform) => transform.stableRef === args?.transformRef);
      if (index < 0) throw new Error('Transform not found');
      const current = mockSavedTransforms[index];
      const updated = {
        ...current,
        name: String(args?.name || current.name),
        plan: args?.plan || current.plan,
        connectionId: args?.connectionId || null,
        revision: Number(current.revision || 1) + 1,
        updatedAt: new Date().toISOString(),
      };
      mockSavedTransforms.splice(index, 1);
      mockSavedTransforms.unshift(updated);
      return updated as unknown as T;
    }
    case 'delete_saved_transform':
      mockSavedTransforms = mockSavedTransforms.filter((transform) => transform.stableRef !== args?.transformRef);
      return null as unknown as T;
    case 'apply_transform_preview_to_clip': {
      const clipId = Number(args?.clipId);
      const clip = mockClips.find((item) => item.id === clipId);
      if (clip) clip.text_content = String(args?.output || clip.text_content);
      if (clip) clip.is_transformed = 1;
      const transform = mockSavedTransforms.find((item) => item.stableRef === args?.transformRef);
      const provenance = {
        transformRef: String(args?.transformRef || ''),
        transformName: String(transform?.name || 'Transform'),
        transformRevision: Number(transform?.revision || 1),
        connectionId: args?.connectionId || null,
        durationMs: Number(args?.durationMs || 0),
        createdAt: new Date().toISOString(),
      };
      mockClipTransformations.set(clipId, provenance);
      return provenance as unknown as T;
    }
    case 'get_clip_transformation_provenance':
      return (mockClipTransformations.get(Number(args?.clipId)) || null) as unknown as T;
    case 'copy_clip_to_system':
    case 'copy_clip_by_id':
    case 'paste_text_to_frontmost':
      return null as unknown as T;
    case 'quit_app':
      return undefined as unknown as T;
    case 'get_source_icons':
      return {} as unknown as T;
    case 'get_external_import_sources':
      return [
        { id: 'alfred', label: 'Alfred', description: 'Clipboard history from Alfred Powerpack', available: true, detected: true, defaultPath: '/mock/Alfred/clipboard.alfdb', supportsCustomFile: true, selectionKind: 'file' },
        { id: 'pastebot', label: 'Pastebot', description: 'Text history from Pastebot', available: false, detected: true, defaultPath: '/mock/Pastebot.sqlite', supportsCustomFile: true, selectionKind: 'folder' },
        { id: 'pasta', label: 'Pasta', description: 'Text history from Pasta', available: false, detected: false, defaultPath: '/mock/Pasta/pasta.sqlite', supportsCustomFile: true, selectionKind: 'file' },
        { id: 'paste', label: 'Paste', description: 'Text history from Paste', available: false, detected: false, defaultPath: '/mock/Paste/Paste.sqlite', supportsCustomFile: true, selectionKind: 'folder' },
        { id: 'copyclip', label: 'CopyClip 2', description: 'Text history from CopyClip 2', available: false, detected: false, defaultPath: '/mock/CopyClip.data', supportsCustomFile: true, selectionKind: 'file' },
        { id: 'maccy', label: 'Maccy', description: 'Text history from Maccy', available: false, detected: false, defaultPath: '/mock/Maccy/Storage.sqlite', supportsCustomFile: true, selectionKind: 'file' },
        { id: 'flycut', label: 'Flycut', description: 'Text history from Flycut', available: false, detected: false, defaultPath: '/mock/Flycut.plist', supportsCustomFile: true, selectionKind: 'file' },
      ] as unknown as T;
    case 'import_external_history':
      return {
        source: String(args?.source ?? 'pastebot'),
        scannedCount: 42,
        importedCount: 38,
        duplicateCount: 3,
        skippedCount: 1,
        historyCapacityAdjustedTo: 1200,
      } as unknown as T;
    case 'get_library_location':
      return mockLibraryLocation as unknown as T;
    case 'get_storage_protection':
      return {
        status: 'protected',
        technology: 'FileVault',
        summary: 'FileVault is on',
        detail: 'The volume containing this database is encrypted.',
      } as unknown as T;
    case 'move_library':
      mockLibraryLocation = {
        path: '/mock/Custom Pasted Library/pasted.db',
        directory: '/mock/Custom Pasted Library',
        isDefault: false,
      };
      return { location: mockLibraryLocation, recoveryPath: '/mock/Pasted/pasted.db' } as unknown as T;
    case 'restore_default_library_location':
      mockLibraryLocation = {
        path: '/mock/Pasted/pasted.db',
        directory: '/mock/Pasted',
        isDefault: true,
      };
      return { location: mockLibraryLocation, recoveryPath: '/mock/Custom Pasted Library/pasted.db' } as unknown as T;
    case 'factory_reset_app': {
      const report = {
        clipsDeleted: mockClips.length,
        binsDeleted: mockBins.length,
        transformsDeleted: mockSavedTransforms.length,
        connectionsDeleted: mockIntelligenceConnections.length,
        activityEntriesDeleted: 0,
      };
      mockClips = [];
      mockBins = [
        { id: 1, name: 'Images', icon: '🖼️', color: '#ec4899', smart_rule: '{"version":1,"conditions":[{"type":"clip_type","operator":"is","value":"image"}],"match":"any"}', bin_type: 'category' },
        { id: 2, name: 'Links and Web', icon: 'Link', color: '#3b82f6', smart_rule: '{"version":1,"conditions":[{"type":"content_type","operator":"is","value":"link"}],"match":"any"}', bin_type: 'category' },
        { id: 3, name: 'Code Snippets', icon: 'Code', color: '#10b981', smart_rule: '{"version":1,"conditions":[{"type":"content_type","operator":"is","value":"code"}],"match":"any"}', bin_type: 'category' },
      ];
      mockSavedTransforms = [];
      mockIntelligenceConnections = [];
      mockClipTransformations = new Map();
      mockBinTransforms = new Map();
      return report as unknown as T;
    }
    case 'get_clip_versions':
      return [] as unknown as T;
    case 'get_clip_version_count':
      return 0 as unknown as T;
    case 'get_clip_image':
      return null as unknown as T;
    case 'analyze_content': {
      const request = (args?.request ?? {}) as Record<string, unknown>;
      const hasText = typeof request.text === 'string';
      const hasClipId = request.clipId !== undefined;
      if (hasText === hasClipId) throw new Error('Provide exactly one of text or clipId');
      const clip = hasClipId
        ? mockClips.find((item) => item.id === Number(request.clipId))
        : undefined;
      if (hasClipId && !clip) throw new Error('Clip not found');
      const text = hasText ? String(request.text) : String(clip?.text_content ?? '');
      const clipKind = clip?.content_type === 'file' || clip?.content_type === 'image'
        ? clip.content_type
        : 'text';
      const paths = clip ? getClipFilePaths(clip) : [];
      const structure = {
        origin: clip ? getClipOriginKind(clip) : 'command_line',
        byteCount: new TextEncoder().encode(clipKind === 'file' ? paths.join('') : text).length,
        ...(clipKind === 'file'
          ? { files: { itemCount: paths.length, extensions: [] } }
          : clipKind === 'image'
            ? { image: { width: 0, height: 0 } }
            : { text: {
            characterCount: Array.from(text).length,
            wordCount: text.split(/\p{White_Space}+/u).filter(Boolean).length,
            lineCount: text.length === 0 ? 0 : text.split(/\r?\n/).length - (/\r?\n$/.test(text) ? 1 : 0),
          } }),
      };
      const includeSuggestions = request.includeSuggestions !== false
        && (request.policy === undefined || request.policy === 'interactive')
        && clipKind !== 'file'
        && clipKind !== 'image';
      const includeClassifiers = request.includeClassifiers !== false
        && clipKind !== 'file'
        && clipKind !== 'image';
      const suggestions = includeSuggestions ? mockSmartActionSuggestions(text) : null;
      const participants = [
        { stableRef: 'inspector:structure-v1', pass: 'inspect', outcome: 'produced' },
        ...(includeClassifiers ? [{ stableRef: 'analysis:content-classifiers', pass: 'classify', outcome: 'produced' }] : []),
        ...(suggestions ? [{ stableRef: 'suggestion:smart-actions-v1', pass: 'suggest', outcome: 'produced' }] : []),
      ];
      return {
        formatVersion: 1,
        policy: request.policy ?? 'interactive',
        through: request.policy && request.policy !== 'interactive' ? 'classify' : 'suggest',
        result: {
          clipKind,
          structure,
          ...(includeClassifiers ? { classificationMatches: [] } : {}),
          searchableTextAvailable: false,
          ...(suggestions ? { suggestions } : {}),
        },
        participants,
        appliedClipId: null,
        ...(clipKind === 'file' ? { liveFileObservations: { availableCount: 0, fileCount: 0, directoryCount: 0, totalSizeBytes: 0 } } : {}),
      } as unknown as T;
    }
    case 'get_file_clip_previews':
      return [] as unknown as T;
    case 'restore_clip_version': {
      const clipId = Number(args?.clipId);
      const clip = mockClips.find((item) => item.id === clipId);
      return (clip ? { ...clip } : null) as unknown as T;
    }
    case 'update_clip_note': {
      const clipId = Number(args?.clipId);
      const clip = mockClips.find((item) => item.id === clipId);
      if (clip && clip.is_trashed === 0) clip.note = typeof args?.note === 'string' ? args.note : null;
      return null as unknown as T;
    }
    case 'delete_clip': {
      const clip = mockClips.find((item) => item.id === Number(args?.id));
      if (clip && !clip.is_protected) {
        clip.is_trashed = 1;
        clip.bin_id = null;
        clip.bin_ids = clip.bin_ids.filter((id) => mockBins.find((bin) => bin.id === id)?.bin_type === 'tag');
      }
      return null as unknown as T;
    }
    case 'batch_trash_clips': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(Number) : [];
      for (const clip of mockClips) {
        if (ids.includes(clip.id) && !clip.is_protected) {
          clip.is_trashed = 1;
          clip.bin_id = null;
          clip.bin_ids = clip.bin_ids.filter((id) => mockBins.find((bin) => bin.id === id)?.bin_type === 'tag');
        }
      }
      return null as unknown as T;
    }
    case 'restore_clip': {
      const clip = mockClips.find((item) => item.id === Number(args?.id));
      if (clip) clip.is_trashed = 0;
      return null as unknown as T;
    }
    case 'restore_all_trashed_clips': {
      const restoredIds = mockClips.filter((clip) => clip.is_trashed !== 0).map((clip) => clip.id);
      for (const clip of mockClips) {
        if (clip.is_trashed !== 0) {
          clip.is_trashed = 0;
          clip.trashed_at = null;
        }
      }
      return {
        action: 'restore_all',
        requestedCount: restoredIds.length,
        changedCount: restoredIds.length,
        skippedCount: 0,
        clipIds: restoredIds,
      } as unknown as T;
    }
    case 'purge_clip_permanently':
      mockClips = mockClips.filter((clip) => clip.id !== Number(args?.id) || clip.is_protected);
      return null as unknown as T;
    case 'empty_trash':
      mockClips = mockClips.filter((clip) => clip.is_trashed === 0 || clip.is_protected);
      return null as unknown as T;
    case 'assign_clip_bin': {
      const clipId = Number(args?.clipId);
      const binId = args?.binId === null ? null : Number(args?.binId);
      if (Number.isInteger(clipId) && (binId === null || Number.isInteger(binId))) {
        assignMockClips([clipId], binId);
      }
      const transformed = binId !== null && mockBinTransforms.has(binId)
        ? mockClips.find((clip) => clip.id === clipId) ?? null
        : null;
      return (transformed ? { ...transformed, is_transformed: true } : null) as unknown as T;
    }
    case 'batch_assign_bin_clips': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(Number).filter(Number.isInteger) : [];
      const binId = args?.binId === null ? null : Number(args?.binId);
      if (binId === null || Number.isInteger(binId)) assignMockClips(ids, binId);
      return true as unknown as T;
    }
    case 'create_bin': {
      const id = Math.max(0, ...mockBins.map((bin) => bin.id)) + 1;
      const created = {
        id,
        name: typeof args?.name === 'string' ? args.name : 'Untitled Bin',
        icon: typeof args?.icon === 'string' ? args.icon : '📂',
        color: typeof args?.color === 'string' ? args.color : 'default',
        smart_rule: typeof args?.smartRule === 'string' ? args.smartRule : null,
        bin_type: 'category',
        protect_clips: false,
      };
      mockBins.push(created);
      return created as unknown as T;
    }
    case 'update_bin': {
      const bin = mockBins.find((item) => item.id === Number(args?.id));
      if (bin) {
        if (typeof args?.name === 'string') bin.name = args.name;
        if (typeof args?.icon === 'string') bin.icon = args.icon;
        if (typeof args?.color === 'string') bin.color = args.color;
        bin.smart_rule = typeof args?.smartRule === 'string' ? args.smartRule : null;
      }
      return null as unknown as T;
    }
    case 'update_bin_protection': {
      const bin = mockBins.find((item) => item.id === Number(args?.id));
      if (bin && !bin.smart_rule) bin.protect_clips = Boolean(args?.protectClips);
      return null as unknown as T;
    }
    case 'get_clip_hotkey_assignments':
      return mockClips
        .filter((clip) => Boolean(clip.hotkey))
        .map((clip) => ({ clipId: clip.id, hotkey: clip.hotkey })) as unknown as T;
    case 'update_clip_hotkey': {
      const clip = mockClips.find((item) => item.id === Number(args?.clipId));
      if (clip) {
        clip.hotkey = typeof args?.hotkey === 'string' && args.hotkey.trim()
          ? args.hotkey.trim()
          : null;
        if (clip.hotkey) {
          clip.is_protected = 1;
          clip.is_explicitly_protected = true;
        }
      }
      return clip as unknown as T;
    }
    case 'delete_bin': {
      const id = Number(args?.id);
      const disposition = typeof args?.disposition === 'string' ? args.disposition : 'keep';
      const destinationBinId = Number(args?.destinationBinId);
      mockBins = mockBins.filter((bin) => bin.id !== id);
      for (const clip of mockClips) {
        const belongsToBin = clip.bin_id === id || clip.bin_ids.includes(id);
        if (!belongsToBin) continue;
        clip.bin_ids = clip.bin_ids.filter((binId) => binId !== id);
        if (disposition === 'move' && Number.isFinite(destinationBinId)) {
          clip.bin_ids = clip.bin_ids.filter((binId) => {
            const candidate = mockBins.find((bin) => bin.id === binId);
            return candidate?.bin_type === 'tag';
          });
          clip.bin_ids.push(destinationBinId);
          clip.bin_id = destinationBinId;
        } else if (disposition === 'trash' && !clip.is_protected) {
          clip.bin_ids = [];
          clip.bin_id = null;
          clip.is_trashed = 1;
          clip.trashed_at = new Date().toISOString();
        } else if (clip.bin_id === id) {
          clip.bin_id = null;
        }
      }
      return null as unknown as T;
    }
    case 'toggle_pin_clip': {
      const clip = mockClips.find((item) => item.id === Number(args?.id));
      if (clip) {
        const nextPinned = clip.is_pinned === 0;
        if (nextPinned) {
          for (const item of mockClips) {
            if (item.is_pinned) item.pin_order += 1;
          }
        }
        clip.is_pinned = nextPinned ? 1 : 0;
        clip.pin_order = 0;
      }
      return Boolean(clip?.is_pinned) as unknown as T;
    }
    case 'batch_pin_clips': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(Number).filter(Number.isInteger) : [];
      const pinState = Boolean(args?.pinState);
      const changedIds = [...new Set(ids)].filter((id) => {
        const clip = mockClips.find((item) => item.id === id);
        return clip && Boolean(clip.is_pinned) !== pinState;
      });
      if (pinState && changedIds.length > 0) {
        for (const clip of mockClips) {
          if (clip.is_pinned) clip.pin_order += changedIds.length;
        }
      }
      changedIds.forEach((id, index) => {
        const clip = mockClips.find((item) => item.id === id);
        if (clip) {
          clip.is_pinned = pinState ? 1 : 0;
          clip.pin_order = pinState ? index : 0;
        }
      });
      return {
        action: pinState ? 'pin' : 'unpin',
        requestedCount: ids.length,
        changedCount: changedIds.length,
        skippedCount: ids.length - changedIds.length,
        clipIds: changedIds,
      } as unknown as T;
    }
    case 'reorder_pinned_clips': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(Number).filter(Number.isInteger) : [];
      ids.forEach((id, index) => {
        const clip = mockClips.find((item) => item.id === id);
        if (clip?.is_pinned) clip.pin_order = index;
      });
      return null as unknown as T;
    }
    case 'reorder_bin_clips': {
      const binId = Number(args?.binId);
      const clipIds = Array.isArray(args?.clipIds) ? args.clipIds.map(Number) : [];
      const bin = mockBins.find((item) => item.id === binId);
      if (bin) bin.clip_order = clipIds;
      return null as unknown as T;
    }
    case 'toggle_clip_protected': {
      const clip = mockClips.find((item) => item.id === Number(args?.clipId));
      if (clip) clip.is_protected = clip.is_protected ? 0 : 1;
      return null as unknown as T;
    }
    case 'batch_protect_clips': {
      const ids = Array.isArray(args?.ids) ? args.ids.map(Number) : [];
      for (const clip of mockClips) {
        if (ids.includes(clip.id)) clip.is_protected = args?.protectedState ? 1 : 0;
      }
      return {
        action: args?.protectedState ? 'protect' : 'unprotect',
        requestedCount: ids.length,
        changedCount: ids.length,
        skippedCount: 0,
        clipIds: ids,
      } as unknown as T;
    }
    case 'enforce_activity_retention':
    case 'enforce_clip_retention':
    case 'enforce_revision_retention':
    case 'enforce_trash_retention':
    case 'perform_titlebar_double_click':
    case 'play_system_sound':
    case 'set_dock_visibility':
    case 'set_linux_native_menu_theme':
    case 'set_overlay_cursor':
    case 'set_titlebar_direction':
    case 'toggle_hud_window':
      return undefined as unknown as T;
    case 'get_transforms':
      return [
        ...mockManualTransforms.map((manualTransform) => ({
          ...manualTransform,
          authoringKind: 'manual',
          executionCharacter: 'replayable',
          connectionId: null,
          plan: null,
        })),
        ...mockSavedTransforms.map((transform) => ({
          ...transform,
          authoringKind: 'intent',
          executionCharacter: 'interpretive',
          steps: [],
        })),
      ] as unknown as T;
    case 'get_capture_feedback_clip': {
      const clip = mockClips.find(({ id }) => id === Number(args?.id));
      if (!clip) throw new Error('Clip was not found');
      return {
        id: clip.id,
        contentType: clip.content_type,
        previewText: clip.text_content.slice(0, 280),
        source: clip.source,
        isPinned: Boolean(clip.is_pinned),
        isProtected: Boolean(clip.is_protected),
        isTrashed: Boolean(clip.is_trashed),
      } as unknown as T;
    }
    case 'get_installed_applications':
      return ['Finder', 'Safari', 'Terminal'] as unknown as T;
    case 'install_cli_to_path':
      return '/mock/bin/pasted' as unknown as T;
    case 'open_backing_page':
    case 'open_emoji_picker':
    case 'request_accessibility_permission':
      return true as unknown as T;
    case 'paste_clip_by_id':
      return undefined as unknown as T;
    case 'resolve_logical_shortcut_key':
      return String(args?.fallback ?? '') as unknown as T;
    case 'remove_clip_bin': {
      const clip = mockClips.find(({ id }) => id === Number(args?.clipId));
      const binId = Number(args?.binId);
      if (!clip) throw new Error('Clip was not found');
      clip.bin_ids = clip.bin_ids.filter((id) => id !== binId);
      if (clip.bin_id === binId) clip.bin_id = null;
      return {
        mutation: { action: 'remove_bin', requestedCount: 1, changedCount: 1, skippedCount: 0, clipIds: [clip.id] },
        updatedClips: [{ ...clip }],
      } as unknown as T;
    }
    case 'trash_unpinned_clips':
      mockClips.forEach((clip) => {
        if (!clip.is_pinned && !clip.is_protected) clip.is_trashed = 1;
      });
      return undefined as unknown as T;
    case 'purge_unpinned_clips':
      mockClips = mockClips.filter((clip) => clip.is_pinned || clip.is_protected);
      return undefined as unknown as T;
    case 'update_bin_hotkey': {
      const bin = mockBins.find(({ id }) => id === Number(args?.id));
      if (bin) bin.hotkey = typeof args?.hotkey === 'string' ? args.hotkey : null;
      return undefined as unknown as T;
    }
    case 'export_clips_json':
      return JSON.stringify(mockClips) as unknown as T;
    case 'export_clips_csv':
      return 'id,content_type,text_content,source,created_at\n' as unknown as T;
    case 'transform_text': {
      const input = String(args?.input ?? '');
      const filterType = String(args?.filterType ?? '');
      if (filterType === 'uppercase') return input.toUpperCase() as unknown as T;
      if (filterType === 'lowercase') return input.toLowerCase() as unknown as T;
      if (filterType === 'trim') return input.trim() as unknown as T;
      if (filterType === 'regex') {
        const config = JSON.parse(String(args?.config ?? '{}')) as { pattern?: string; replacement?: string };
        return input.replace(new RegExp(config.pattern ?? '', 'g'), config.replacement ?? '') as unknown as T;
      }
      return input as unknown as T;
    }
    default:
      throw new Error(`Unsupported browser IPC command: ${cmd}`);
  }
}
