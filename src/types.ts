export interface ClipNote {
  id: string;
  text: string;
  created_at: string;
}

export function parseClipNotes(noteField?: string | null): ClipNote[] {
  if (!noteField || !noteField.trim()) return [];
  try {
    const parsed = JSON.parse(noteField);
    if (Array.isArray(parsed)) {
      return parsed.map((n, idx) => {
        if (typeof n === 'string') {
          return { id: `note-${idx}`, text: n, created_at: new Date().toISOString() };
        }
        return {
          id: n.id || `note-${idx}`,
          text: n.text || '',
          created_at: n.created_at || new Date().toISOString(),
        };
      });
    }
  } catch {
    // Legacy single string note
    return [{ id: 'note-legacy', text: noteField, created_at: new Date().toISOString() }];
  }
  return [{ id: 'note-legacy', text: noteField, created_at: new Date().toISOString() }];
}

export function serializeClipNotes(notes: ClipNote[]): string | null {
  const validNotes = notes.filter((n) => n.text.trim().length > 0);
  if (validNotes.length === 0) return null;
  return JSON.stringify(validNotes);
}

export function getClipNoteSummary(noteField?: string | null): string {
  const notes = parseClipNotes(noteField);
  if (notes.length === 0) return '';
  return notes.map((n) => n.text.trim()).filter((t) => t.length > 0).join(' • ');
}

export function isSensitiveText(text: string | null): boolean {
  if (!text) return false;
  const trimmed = text.trim();
  if (
    /(?:sk_live_|sk_test_|ghp_|gho_|xoxb-|xoxp-|AKIA[0-9A-Z]{16}|sk-proj-|sk-ant-)\w+/i.test(trimmed) ||
    /bearer\s+[a-zA-Z0-9_\-\.=]+/i.test(trimmed) ||
    /-----BEGIN (?:RSA )?PRIVATE KEY-----/.test(trimmed)
  ) {
    return true;
  }
  const ccDigits = trimmed.replace(/[\s-]/g, '');
  if (/^\d{13,19}$/.test(ccDigits) && !isNaN(Number(ccDigits))) {
    return true;
  }
  if (/^(?:password|passwd|secret_key|api_secret)\s*[:=]/i.test(trimmed)) {
    return true;
  }
  return false;
}

export function maskSensitiveText(text: string | null): string {
  if (!text) return '';
  const trimmed = text.trim();
  if (trimmed.length <= 8) {
    return '•••• ••••';
  }
  const lastFour = trimmed.slice(-4);
  return `•••• •••• •••• ${lastFour}`;
}

export interface ClipItem {
  id: number;
  content_type: ClipContentType;
  text_content: string | null;
  html_content: string | null;
  image_base64: string | null;
  image_path?: string | null;
  content_hash: string;
  source: string;
  is_pinned: boolean;
  is_protected?: boolean;
  is_transformed?: boolean;
  pin_order?: number;
  bin_id: number | null;
  bin_ids?: number[];
  note?: string | null;
  is_trashed?: boolean | number;
  trashed_at?: string | null;
  created_at: string;
  ocr_extractor_ref?: string | null;
  ocr_extractor_name?: string | null;
  ocr_engine_version?: string | null;
}

export type ClipContentType =
  | 'text' | 'prose' | 'image' | 'file' | 'file_path'
  | 'color' | 'link' | 'code' | 'shell_command'
  | 'email' | 'phone' | 'ip_address' | 'mac_address'
  | 'credential' | 'payment_card' | 'env_variable' | 'env_block'
  | 'hash' | 'iban' | 'jwt' | 'uuid'
  | (string & {});

export interface ClipCollectionSummary {
  activeCount: number;
  trashCount: number;
  pinnedCount: number;
  protectedCount: number;
  notedCount: number;
  typeCounts: Array<{ content_type: ClipContentType; count: number }>;
  sourceCounts: Array<{ name: string; count: number }>;
}

export type ClipOriginKind = 'clipboard_content' | 'file_reference' | 'screenshot' | 'command_line';

export function getClipOriginKind(
  clip: Pick<ClipItem, 'content_type' | 'source'>
): ClipOriginKind {
  const source = clip.source.trim().toLowerCase();
  if (
    (clip.content_type === 'image' || clip.content_type === 'file')
    && (source.includes('screenshot')
      || source.includes('screencapture')
      || source.includes('cleanshot'))
  ) {
    return 'screenshot';
  }
  if (clip.content_type === 'file') return 'file_reference';
  if (source === 'cli terminal' || source === 'pasted cli') return 'command_line';
  return 'clipboard_content';
}

export function getClipFilePaths(clip: Pick<ClipItem, 'content_type' | 'text_content'>): string[] {
  if (clip.content_type !== 'file' || !clip.text_content) return [];
  try {
    const paths = JSON.parse(clip.text_content);
    if (Array.isArray(paths) && paths.every((path) => typeof path === 'string')) return paths;
    if (typeof paths === 'string' && paths.trim()) return [paths.trim()];
  } catch {
    // Early file clips stored either one path or a newline-delimited selection.
  }
  return clip.text_content.split(/\r?\n/).map((path) => path.trim()).filter(Boolean);
}

export function getClipFileSummary(clip: Pick<ClipItem, 'content_type' | 'text_content'>): string {
  const paths = getClipFilePaths(clip);
  if (paths.length === 0) return 'File';
  const name = paths[0].split(/[\\/]/).filter(Boolean).pop() || paths[0];
  return paths.length === 1 ? name : `${name} +${paths.length - 1} more`;
}

export interface Bin {
  id: number;
  name: string;
  icon: string;
  color: string;
  smart_rule?: string | null;
  bin_type?: 'category' | 'tag';
  shortcut?: string | null;
  clip_count?: number | null;
  clip_order?: number[];
  created_at: string;
}

export interface ClipVersion {
  id: number;
  clip_id: number;
  text_content: string;
  action_kind?: string | null;
  action_label?: string | null;
  restores_organization?: boolean;
  created_at: string;
}

export interface ClipMutationSummary {
  action: string;
  requestedCount: number;
  changedCount: number;
  skippedCount: number;
  clipIds: number[];
}

export interface Pipeline {
  id: number;
  stableRef: string;
  name: string;
  steps: PipelineStep[];
  shortcut?: string | null;
  revision: number;
  createdAt: string;
  updatedAt: string;
}

export interface PipelineStep {
  position: number;
  operationRef: string;
  configJson: string | null;
  failurePolicy: 'stop' | 'skip';
}

export interface Operation {
  id: number;
  stable_id: string;
  name: string;
  op_type: string;
  config: string | null;
  category: string;
  created_at: string;
}

export interface LibraryItemView {
  stableRef: string;
  kind: 'inspector' | 'extractor' | 'detector' | 'enricher' | 'operation' | 'transform';
  name: string;
  description: string;
  groupLabel: string | null;
  icon: string;
  enabled: boolean | null;
  isBuiltin: boolean;
  isArchived: boolean;
  sortOrder: number | null;
  revision: number;
  inputContract: string;
  outputContract: string;
  analysisPass: 'inspect' | 'extract' | 'classify' | 'enrich' | null;
  participantContract?: {
    stableRef: string;
    name: string;
    pass: 'inspect' | 'extract' | 'classify' | 'enrich';
    priority: number;
    requires: AnalysisRepresentation[];
    provides: AnalysisRepresentation[];
  };
  typeRelations?: Array<{
    kind: 'accepts' | 'classifies_as';
    typeId: string;
  }>;
  createdAt: string;
  updatedAt: string;
  capabilities: {
    canEdit: boolean;
    canDuplicate: boolean;
    canDelete: boolean;
    canDisable: boolean;
    canRestore: boolean;
  };
}

export type AnalysisRepresentation =
  | 'clip_kind'
  | 'capture_source'
  | 'original_text'
  | 'file_references'
  | 'image'
  | 'searchable_text'
  | 'analyzable_text'
  | 'classification'
  | 'structural_metadata'
  | 'media_metadata'
  | 'recommendations';

export type IntelligenceProviderKind =
  | 'openai_compatible'
  | 'anthropic'
  | 'gemini'
  | 'ollama'
  | 'lm_studio'
  | 'cli';

export interface IntelligenceConnection {
  id: string;
  name: string;
  providerKind: IntelligenceProviderKind;
  endpoint: string | null;
  model: string | null;
  credentialRef: string | null;
  enabled: boolean;
  priority: number;
  createdAt: string;
  updatedAt: string;
}

export interface DetectedIntelligenceConnection {
  adapterId: string;
  name: string;
  providerKind: IntelligenceProviderKind;
  executablePath: string | null;
  defaultEndpoint: string | null;
  version: string | null;
  capabilities: string[];
  executionSupported: boolean;
}

export interface InstallationDiagnostics {
  appVersion: string;
  buildKind: string;
  platform: string;
  architecture: string;
  bundleIdentifier: string;
  appPath: string;
  dataPath: string;
  databaseSizeBytes: number;
  signingStatus: string;
  signingIdentity: string | null;
  signingTeamId: string | null;
  notarizationStatus: string;
  cliPath: string | null;
}

export interface ThirdPartyComponent {
  ecosystem: 'cargo' | 'npm';
  name: string;
  version: string;
  license: string;
  repository: string;
  noticeIds: string[];
}

export interface ThirdPartyLicenseDocument {
  schemaVersion: number;
  componentCount: number;
  components: ThirdPartyComponent[];
  noticeText: string;
}

export type IntentPlanningMode = 'pinned' | 'adaptive';

export interface PlannedTransformationStep {
  name: string;
  rationale: string;
  scope: 'whole_input' | 'each_line';
  failure_policy?: 'stop' | 'skip';
  executor:
    | { kind: 'deterministic'; operation_ref: string; config_json?: string | null }
    | { kind: 'semantic'; instructions: string; output_schema?: Record<string, unknown> | null; model_policy: 'fast' | 'balanced' | 'deep' };
}

export interface TransformationPlan {
  schema_version: number;
  intent: string;
  summary: string;
  planning_mode: IntentPlanningMode;
  steps: PlannedTransformationStep[];
}

export interface PlanIntentOutcome {
  plan: TransformationPlan;
  connectionId: string;
  connectionName: string;
  durationMs: number;
}

export interface SavedTransform {
  id: number;
  stableRef: string;
  name: string;
  plan: TransformationPlan;
  connectionId: string | null;
  shortcut?: string | null;
  revision: number;
  createdAt: string;
  updatedAt: string;
}

export interface TransformDefinition {
  id: number;
  stableRef: string;
  name: string;
  authoringKind: 'intent' | 'manual';
  executionCharacter: 'replayable' | 'interpretive' | 'mixed';
  connectionId: string | null;
  shortcut: string | null;
  revision: number;
  createdAt: string;
  updatedAt: string;
  plan: TransformationPlan | null;
  steps: PipelineStep[];
}

export interface ExecutePlanOutcome {
  output: string;
  connectionId: string | null;
  connectionName: string | null;
  durationMs: number;
}

export interface TransformationExecutionOutcome extends ExecutePlanOutcome {
  executionId: string;
}

export interface ClipTransformationProvenance {
  transformRef: string;
  transformName: string;
  transformRevision: number;
  connectionId: string | null;
  durationMs: number;
  createdAt: string;
}

export type TransformExecutionStatus = 'queued' | 'running' | 'succeeded' | 'failed' | 'cancelled';
export type TransformExecutionTrigger = 'manual' | 'shortcut' | 'bin' | 'automation' | 'cli';
export type TransformExecutionDestination = 'preview' | 'replace' | 'copy' | 'paste' | 'route';

export interface TransformationExecution {
  id: string;
  targetKind: 'operation' | 'pipeline' | 'transform';
  targetRef: string;
  targetRevision: number | null;
  sourceClipId: number | null;
  triggerKind: TransformExecutionTrigger;
  destinationKind: TransformExecutionDestination;
  startedAt: string;
  completedAt: string | null;
  durationMs: number | null;
  status: TransformExecutionStatus;
  errorSummary: string | null;
}

export interface IntelligenceSchedulerJob {
  id: string;
  clientRequestId: string | null;
  connectionId: string;
  connectionName: string;
  label: string;
  status: 'queued' | 'running';
  queuedAtMs: number;
  startedAtMs: number | null;
  waitMs: number;
  runMs: number;
}

export interface IntelligenceSchedulerEvent {
  sequence: number;
  jobId: string;
  clientRequestId: string | null;
  connectionName: string;
  label: string;
  status: 'queued' | 'running' | 'succeeded' | 'failed' | 'cancelled';
  timestampMs: number;
  detail: string | null;
}

export interface IntelligenceSchedulerSnapshot {
  revision: number;
  activeCount: number;
  queuedCount: number;
  jobs: IntelligenceSchedulerJob[];
  recentEvents: IntelligenceSchedulerEvent[];
}

export interface SequentialStatus {
  is_active: boolean;
  queue: string[];
  item_ids: number[];
  current_index: number;
  total_count: number;
}

export interface QueuePasteTarget {
  name: string;
  automaticPasteAvailable: boolean;
  unavailableReason: string | null;
}

export interface AppSettings {
  onboardingVersion: number;
  textSize: number;
  enableSounds: boolean;
  captureFeedback: boolean;
  captureFeedbackIgnored: boolean;
  captureFeedbackPreview: boolean;
  captureFeedbackPosition: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';
  captureFeedbackDismissSeconds: number;
  openAtLogin: boolean;
  dockMenubarIcon: 'auto_hide' | 'both' | 'menubar_only';
  menubarIconStyle: 'clipboard' | 'copycat';
  maxClipSizeMb: number;
  filePreviewMode: 'off' | 'safe' | 'all';
  filePreviewMaxMb: number;
  keepClipCount: number;
  keepClipAgeDays: number;
  revisionHistoryLimit: number;
  alwaysPastePlainText: boolean;
  rowHeight: 'small' | 'medium' | 'large';
  startupView: 'last_active' | 'clip_history';
  themeMode: 'system' | 'dark' | 'cool' | 'warm' | '2894' | 'sauced' | 'vampire' | 'flux' | '808';
  enableActivityLog: boolean;
  activityLogCapacity: number;
  activityLogAgeDays: number;
  enableTrash: boolean;
  trashCapacityCount: number;
  trashAgeDays: number;
  enableAnalytics: boolean;
  enableBins: boolean;
  enableContentDetection: boolean;
  enableNotes: boolean;
  enableNotifications: boolean;
  enableOcr: boolean;
  enablePinning: boolean;
  enableProtection: boolean;
  enableQueue: boolean;
  enableRevisions: boolean;
  enableHud: boolean;
  enableTransformations: boolean;
  enableTypes: boolean;
  enableSources: boolean;
  enableCli: boolean;
  enableHelp: boolean;
  hudHotkey?: string;
  seqToggleHotkey?: string;
  seqPopHotkey?: string;
  copyLastPipelineHotkey?: string;
  pasteLastPipelineHotkey?: string;
  openTransformationsHotkey?: string;
  openMainWindowHotkey?: string;
  pasteClip1Hotkey?: string;
  pasteClip2Hotkey?: string;
  pasteClip3Hotkey?: string;
  pasteClip4Hotkey?: string;
  pasteClip5Hotkey?: string;
  pasteClip6Hotkey?: string;
  pasteClip7Hotkey?: string;
  pasteClip8Hotkey?: string;
  pasteClip9Hotkey?: string;
}

export interface OcrBackfillStatus {
  totalImages: number;
  eligibleCount: number;
  queuedCount: number;
  runningCount: number;
  completedCount: number;
  noTextCount: number;
  failedCount: number;
}

export interface BlacklistApp {
  id: string;
  name: string;
  icon: string;
  ignoreText: boolean;
  ignoreImages: boolean;
  ignoreShortcuts: boolean;
}
