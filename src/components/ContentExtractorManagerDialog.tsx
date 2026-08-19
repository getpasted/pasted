import { useEffect, useLayoutEffect, useMemo, useState } from 'react';
import { CircleAlert, CircleCheck, Copy, FolderOpen, Plus, RotateCcw, ScanText, Sparkles, Trash2 } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { analysisApi } from '../api/analysis';
import { errorMessage } from '../utils/errors';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading, SaveButtonContent } from './AppDialogLayout';
import { ConfirmationDialog, type ConfirmationDialogRequest } from './ConfirmationDialog';
import { ModifiedFieldLabel } from './ModifiedFieldLabel';
import { RegistryListItem } from './RegistryListItem';
import { RegistryPanelFooter } from './RegistryPanelFooter';
import { RegistryPanelHeader } from './RegistryPanelHeader';
import { SettingsSwitch } from './SettingsSwitch';
import { useToast } from './ToastProvider';
import { useNewItemSelection } from '../hooks/useNewItemSelection';
import { translate } from '../localization/runtime';
import { localizedBuiltinDescription, localizedBuiltinName } from '../localization/presentation';
import { MenuMultiSelect } from './MenuMultiSelect';
import { MenuSelect } from './MenuSelect';
import type { IntelligenceConnection } from '../types';
import { dateTimeAttribute, formatFullDateTime, formatRelativeTime } from '../utils/date';

type ExtractorInputKind = 'image' | 'file_references';
type ExtractorCapture = 'ignore' | 'stdout_text' | 'file_text' | 'pasted_json_v1';

interface ExtractorRecipe {
  definitionVersion: 1;
  accepts: ExtractorInputKind[];
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

interface ExtractorAuthoringManifest {
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
  runtime: {
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
  };
  recipe: ExtractorRecipe;
  recipeHash: string;
  defaultRecipe: ExtractorRecipe | null;
  defaults: ExtractorInput | null;
}

interface ExtractorInput {
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

const EXTRACTOR_INPUT_OPTIONS = [
  { value: 'original_text', get label() { return translate('component.contentExtractorManagerDialog.text'); }, disabled: true },
  { value: 'image', get label() { return translate('component.contentExtractorManagerDialog.image'); }, disabled: false },
  { value: 'file_references', get label() { return translate('component.contentExtractorManagerDialog.files'); }, disabled: false },
] as const;

const EXTRACTOR_OUTPUT_OPTIONS = [
  { value: 'searchable_text', get label() { return translate('component.contentExtractorManagerDialog.searchableText'); } },
] as const;

const emptyRecipe = (): ExtractorRecipe => ({
  definitionVersion: 1,
  accepts: ['image'],
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

function toInput(extractor?: ContentExtractor): ExtractorInput {
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

interface ExtractorRecipeProposal {
  name: string;
  description: string;
  recipe: ExtractorRecipe;
  setupGuidance: string[];
  authoring: ExtractorAuthoringManifest;
  connectionName: string;
}

type ExtractorTestOutcome =
  | { outcome: 'produced'; text: string }
  | { outcome: 'no_output' }
  | { outcome: 'failed'; failure: { code: string; message: string } };

interface ExtractorAuthoringSession {
  id: number;
  extractorId: number;
  source: 'ai' | 'manual' | 'shipped' | 'migrated';
  provider: string | null;
  model: string | null;
  originalPrompt: string | null;
  createdAt: string;
  messages: ExtractorAuthoringManifest['messages'];
}

function authoringRoleLabel(role: ExtractorAuthoringManifest['messages'][number]['role']) {
  switch (role) {
    case 'user': return translate('component.contentExtractorManagerDialog.roleUser');
    case 'assistant': return translate('component.contentExtractorManagerDialog.roleAssistant');
    case 'tool': return translate('component.contentExtractorManagerDialog.roleTool');
    case 'system': return translate('component.contentExtractorManagerDialog.roleSystem');
  }
}

export function ContentExtractorManagerDialog({
  isOpen,
  onClose,
  onChanged,
  onOpenIntelligence,
  ocrEnabled,
  transcriptionsEnabled,
}: {
  isOpen: boolean;
  onClose: () => void;
  onChanged?: () => void;
  onOpenIntelligence?: () => void;
  ocrEnabled: boolean;
  transcriptionsEnabled: boolean;
}) {
  const { showToast } = useToast();
  const [extractors, setExtractors] = useState<ContentExtractor[]>([]);
  const [selectedId, setSelectedId] = useState<number | 'new' | null>(null);
  const [draft, setDraft] = useState<ExtractorInput>(toInput());
  const [recipeDraft, setRecipeDraft] = useState<ExtractorRecipe>(emptyRecipe());
  const [authoring, setAuthoring] = useState<ExtractorAuthoringManifest | null>(null);
  const [authoringPrompt, setAuthoringPrompt] = useState('');
  const [setupGuidance, setSetupGuidance] = useState<string[]>([]);
  const [connections, setConnections] = useState<IntelligenceConnection[]>([]);
  const [generating, setGenerating] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testOutcome, setTestOutcome] = useState<ExtractorTestOutcome | null>(null);
  const [authoringHistory, setAuthoringHistory] = useState<ExtractorAuthoringSession[] | null>(null);
  const [saving, setSaving] = useState(false);
  const [confirmation, setConfirmation] = useState<ConfirmationDialogRequest | null>(null);
  const visibleExtractors = useMemo(
    () => extractors.filter((extractor) => (
      extractor.stableRef === 'extractor:apple-vision-ocr'
        || extractor.stableRef === 'extractor:tesseract-ocr'
        ? ocrEnabled
        : extractor.stableRef === 'extractor:whisper-transcription'
          ? transcriptionsEnabled
          : true
    )),
    [extractors, ocrEnabled, transcriptionsEnabled],
  );
  const selected = useMemo(
    () => typeof selectedId === 'number' ? visibleExtractors.find((extractor) => extractor.id === selectedId) : undefined,
    [selectedId, visibleExtractors],
  );
  const { beginNew: beginNewExtractor, cancelNew: cancelNewExtractor } = useNewItemSelection({
    selectedId,
    setSelectedId,
    itemIds: visibleExtractors.map(({ id }) => id),
    emptySelection: null,
  });

  const load = async () => {
    const loaded = await analysisApi.listExtractors<ContentExtractor>();
    setExtractors(loaded);
    const visible = loaded.filter((extractor) => (
      extractor.stableRef === 'extractor:apple-vision-ocr'
        || extractor.stableRef === 'extractor:tesseract-ocr'
        ? ocrEnabled
        : extractor.stableRef === 'extractor:whisper-transcription'
          ? transcriptionsEnabled
          : true
    ));
    setSelectedId((current) => visible.some(({ id }) => id === current) ? current : visible[0]?.id ?? null);
  };

  useEffect(() => {
    if (isOpen) {
      void load();
      void invoke<IntelligenceConnection[]>('get_intelligence_connections')
        .then(setConnections)
        .catch(() => setConnections([]));
    }
  }, [isOpen, ocrEnabled, transcriptionsEnabled]);
  useEffect(() => {
    setSelectedId((current) => visibleExtractors.some(({ id }) => id === current)
      ? current
      : visibleExtractors[0]?.id ?? null);
  }, [visibleExtractors]);
  useLayoutEffect(() => {
    setDraft(selectedId === 'new' ? toInput() : toInput(selected));
    setRecipeDraft(selectedId === 'new' ? emptyRecipe() : selected?.recipe ?? emptyRecipe());
    setAuthoring(null);
    setAuthoringPrompt('');
    setSetupGuidance([]);
    setTestOutcome(null);
  }, [selected, selectedId]);

  const baseline = selectedId === 'new' ? toInput() : selected ? toInput(selected) : null;
  const baselineRecipe = selectedId === 'new' ? emptyRecipe() : selected?.recipe ?? null;
  const isDirty = baseline !== null && (
    JSON.stringify(draft) !== JSON.stringify(baseline)
    || JSON.stringify(recipeDraft) !== JSON.stringify(baselineRecipe)
    || authoring !== null
  );
  const defaults = selected?.defaults;
  const defaultDraft = selected && defaults ? { ...toInput(selected), ...defaults } : null;
  const differsFromDefaults = defaultDraft !== null && (
    JSON.stringify(draft) !== JSON.stringify(defaultDraft)
    || JSON.stringify(recipeDraft) !== JSON.stringify(selected?.defaultRecipe)
  );
  const runtimeConfigurationChanged = selected !== undefined
    && JSON.stringify(recipeDraft) !== JSON.stringify(selected.recipe);
  const unavailableReason = selected?.unavailableReason ?? 'The configured engine is unavailable.';
  const shortUnavailableReason = unavailableReason.match(/^.*?\.(?:\s|$)/)?.[0].trim()
    ?? unavailableReason;
  const availabilityLabel = selectedId === 'new'
    ? translate('component.contentExtractorManagerDialog.saveToCheckAvailability')
    : runtimeConfigurationChanged
      ? translate('component.contentExtractorManagerDialog.saveToCheckAvailability')
      : selected?.isAvailable
        ? translate('component.contentExtractorManagerDialog.available')
        : shortUnavailableReason;
  const availabilityTitle = selectedId === 'new' || runtimeConfigurationChanged
    ? translate('component.contentExtractorManagerDialog.saveTheExtractorToCheckEngineAvailability')
    : selected?.isAvailable
      ? translate('component.contentExtractorManagerDialog.theConfiguredEngineIsReadyOnThisSystem')
      : unavailableReason;
  const hasIntelligence = connections.some((connection) => connection.enabled);
  const resourceRequiredLabel = translate('component.contentExtractorManagerDialog.resourceRequired');
  const recipeCanSave = recipeDraft.accepts.length > 0
    && recipeDraft.steps.length > 0
    && recipeDraft.steps.every((step) => (
      Boolean(step.executable.path || step.executable.discover.length > 0)
      && step.id.trim().length > 0
    ));


  const requestConfirmation = (request: ConfirmationDialogRequest) => {
    setConfirmation({
      ...request,
      onConfirm: async () => {
        setConfirmation(null);
        await request.onConfirm();
      },
    });
  };

  const discardDraftThen = (action: () => void | Promise<void>) => {
    if (!isDirty) {
      void action();
      return;
    }
    requestConfirmation({
      get title() { return translate('common.discardChangesQuestion'); },
      get description() { return translate('component.contentExtractorManagerDialog.unsavedChangesToThisExtractorWillBeLost'); },
      confirmLabel: translate('component.appDialog.discard'),
      tone: 'danger',
      onConfirm: action,
    });
  };

  const selectExtractor = (id: number) => {
    discardDraftThen(() => setSelectedId(id));
  };

  const beginNew = () => {
    discardDraftThen(beginNewExtractor);
  };

  const cancelDraft = () => {
    if (selectedId === 'new') {
      cancelNewExtractor();
      return;
    }
    if (selected) {
      setDraft(toInput(selected));
      setRecipeDraft(selected.recipe);
      setAuthoring(null);
      setSetupGuidance([]);
    }
  };

  const save = async () => {
    setSaving(true);
    try {
      const manualAuthoring: ExtractorAuthoringManifest = {
        manifestVersion: 1,
        source: 'manual',
        originalPrompt: authoringPrompt.trim() || null,
        provider: null,
        model: null,
        messages: authoringPrompt.trim() ? [{
          role: 'user',
          content: authoringPrompt.trim(),
          createdAt: new Date().toISOString(),
          structuredContent: null,
        }] : [],
      };
      const input = {
        name: draft.name,
        description: draft.description,
        enabled: draft.enabled,
        priority: draft.priority,
        recipe: recipeDraft,
        authoring: authoring ?? manualAuthoring,
      };
      const saved = selectedId === 'new'
        ? await invoke<ContentExtractor>('create_content_extractor_recipe', { input })
        : await invoke<ContentExtractor>('update_content_extractor_recipe', { id: selectedId, input });
      await load();
      setSelectedId(saved.id);
      onChanged?.();
      showToast({ tone: 'success', message: translate('component.contentExtractorManagerDialog.nameSaved', { name: saved.name }) });
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    } finally {
      setSaving(false);
    }
  };

  const generateRecipe = async () => {
    if (!authoringPrompt.trim()) return;
    setGenerating(true);
    try {
      const proposal = await invoke<ExtractorRecipeProposal>('propose_extractor_recipe', {
        request: { prompt: authoringPrompt },
      });
      setDraft((current) => ({
        ...current,
        name: proposal.name,
        description: proposal.description,
        engine: 'recipe-v1',
        inputContract: proposal.recipe.accepts[0],
        outputContract: proposal.recipe.output,
        executablePath: proposal.recipe.steps[0]?.executable.path ?? null,
        modelPath: proposal.recipe.resources.find((resource) => resource.id === 'model')?.path ?? null,
      }));
      setRecipeDraft(proposal.recipe);
      setAuthoring(proposal.authoring);
      setSetupGuidance(proposal.setupGuidance);
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    } finally {
      setGenerating(false);
    }
  };

  const openAuthoringHistory = async () => {
    if (!selected) return;
    try {
      setAuthoringHistory(await invoke<ExtractorAuthoringSession[]>('get_extractor_authoring_sessions', {
        reference: selected.stableRef,
      }));
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    }
  };

  const chooseStepExecutable = async (index: number) => {
    try {
      const path = await invoke<string | null>('choose_extractor_executable');
      if (!path) return;
      setRecipeDraft((current) => ({
        ...current,
        steps: current.steps.map((step, stepIndex) => stepIndex === index
          ? { ...step, executable: { ...step.executable, path } }
          : step),
      }));
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    }
  };

  const chooseResource = async (index: number) => {
    try {
      const path = await invoke<string | null>('choose_extractor_resource_file', { kind: recipeDraft.resources[index]?.kind ?? 'file' });
      if (!path) return;
      setRecipeDraft((current) => ({
        ...current,
        resources: current.resources.map((resource, resourceIndex) => resourceIndex === index
          ? { ...resource, path }
          : resource),
      }));
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    }
  };

  const testRecipe = async () => {
    setTesting(true);
    try {
      const path = await invoke<string | null>('choose_extractor_resource_file', { kind: 'file' });
      if (!path) return;
      setTestOutcome(await invoke<ExtractorTestOutcome>('test_content_extractor_recipe', {
        recipe: recipeDraft,
        path,
      }));
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    } finally {
      setTesting(false);
    }
  };

  const updateStep = (index: number, update: Partial<ExtractorRecipe['steps'][number]>) => {
    setRecipeDraft((current) => ({
      ...current,
      steps: current.steps.map((step, stepIndex) => stepIndex === index
        ? { ...step, ...update }
        : step),
    }));
  };

  const updateStepExecutable = (index: number, update: Partial<ExtractorRecipe['steps'][number]['executable']>) => {
    const step = recipeDraft.steps[index];
    if (!step) return;
    updateStep(index, { executable: { ...step.executable, ...update } });
  };

  const updateResource = (index: number, update: Partial<ExtractorRecipe['resources'][number]>) => {
    setRecipeDraft((current) => ({
      ...current,
      resources: current.resources.map((resource, resourceIndex) => resourceIndex === index
        ? { ...resource, ...update }
        : resource),
    }));
  };

  const restoreAllConfirmed = async () => {
    try {
      const restored = await invoke<ContentExtractor[]>('restore_default_content_extractors');
      setExtractors(restored);
      setSelectedId(restored[0]?.id ?? null);
      onChanged?.();
      showToast({ tone: 'success', get message() { return translate('component.contentExtractorManagerDialog.builtInExtractorsRestored'); } });
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    }
  };

  const restoreAll = () => {
    discardDraftThen(() => requestConfirmation({
      get title() { return translate('component.contentExtractorManagerDialog.resetShippedExtractors'); },
      get description() { return translate('component.contentExtractorManagerDialog.shippedExtractorsReturnToTheirDefaults'); },
      details: translate('component.contentExtractorManagerDialog.customExtractorsRemainUnchanged'),
      confirmLabel: translate('common.reset'),
      onConfirm: restoreAllConfirmed,
    }));
  };

  const resetDraft = () => {
    if (defaultDraft) setDraft(defaultDraft);
    if (selected?.defaultRecipe) setRecipeDraft(selected.defaultRecipe);
    setAuthoring(null);
    setSetupGuidance([]);
  };

  const toggleConfirmed = async (extractor: ContentExtractor) => {
    try {
      const enabled = !extractor.enabled;
      await invoke('set_library_item_enabled', {
        kind: 'extractor',
        stableRef: extractor.stableRef,
        enabled,
      });
      setExtractors((current) => current.map((item) => (
        item.id === extractor.id ? { ...item, enabled } : item
      )));
      onChanged?.();
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    }
  };

  const toggle = (extractor: ContentExtractor) => {
    if (selectedId === extractor.id) {
      discardDraftThen(() => toggleConfirmed(extractor));
      return;
    }
    void toggleConfirmed(extractor);
  };

  const duplicate = async () => {
    if (!selected || isDirty) return;
    try {
      const created = await invoke<ContentExtractor>('duplicate_content_extractor', {
        reference: selected.stableRef,
        name: `${selected.name} Copy`,
      });
      await load();
      setSelectedId(created.id);
      onChanged?.();
      showToast({ tone: 'success', message: translate('component.contentExtractorManagerDialog.nameCreated', { name: created.name }) });
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    }
  };

  const removeConfirmed = async () => {
    if (!selected) return;
    try {
      await invoke('delete_content_extractor', { id: selected.id });
      await load();
      onChanged?.();
      showToast({ tone: 'success', message: translate('component.contentExtractorManagerDialog.nameDeleted', { name: selected.name }) });
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    }
  };

  const remove = () => {
    if (!selected) return;
    requestConfirmation({
      get title() { return translate('component.contentExtractorManagerDialog.deleteExtractor'); },
      description: selected.name,
      details: selected.isBuiltin
        ? translate('component.contentExtractorManagerDialog.removingBuiltinExtractorCanBeRecovered')
        : translate('component.contentExtractorManagerDialog.removingCustomExtractorIsPermanent'),
      confirmLabel: translate('component.contentExtractorManagerDialog.deleteExtractor'),
      tone: 'danger',
      onConfirm: removeConfirmed,
    });
  };

  return <><AppDialog
    isOpen={isOpen}
    onClose={onClose}
    labelledBy="extractor-manager-title"
    isDirty={isDirty}
    discardMessage={translate('component.contentExtractorManagerDialog.discardExtractorChanges')}
    panelClassName="theme-panel @container flex max-h-[90vh] w-full max-w-4xl flex-col overflow-hidden border shadow-2xl"
  >
    {({ requestClose }) => <>
      <AppDialogHeader onClose={requestClose} className="shrink-0">
        <AppDialogHeading
          id="extractor-manager-title"
          title={translate('component.contentExtractorManagerDialog.extractors')}
          description={translate('component.contentExtractorManagerDialog.createSearchableRepresentationsFromClipContentTheLowestPriorityNumberRunsFirst')}
          icon={<ScanText />}
        />
      </AppDialogHeader>
      <AppDialogBody className="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-y-auto text-xs @xl:grid-cols-[minmax(0,3fr)_minmax(0,7fr)]">
        <section className="theme-surface flex min-h-[260px] flex-col overflow-hidden rounded-xl border @xl:min-h-0">
          <RegistryPanelHeader title={translate('component.contentExtractorManagerDialog.extractors')} actions={<AppDialogButton onClick={beginNew} className="h-7 min-h-7 px-2.5"><Plus className="h-3.5 w-3.5" /> {translate('common.new')}</AppDialogButton>} />
          <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
            {visibleExtractors.length === 0 && (
              <p className="theme-text-muted px-3 py-4 text-center text-[10px]">{translate('component.contentExtractorManagerDialog.noExtractorsAreAvailableForEnabledFunctionality')}</p>
            )}
            {visibleExtractors.map((extractor) => {
              const displayName = localizedBuiltinName('extractor', extractor.stableRef, extractor.name, extractor.isBuiltin, extractor.defaults?.name);
              const displayDescription = localizedBuiltinDescription('extractor', extractor.stableRef, extractor.description, extractor.isBuiltin, extractor.defaults?.description);
              return <RegistryListItem
              key={extractor.id}
              selected={selectedId === extractor.id}
              onSelect={() => selectExtractor(extractor.id)}
              icon={<ScanText className="h-4 w-4" />}
              title={displayName}
              subtitle={extractor.isAvailable ? displayDescription : extractor.unavailableReason}
              muted={!extractor.isAvailable}
              trailing={<SettingsSwitch
                checked={extractor.enabled}
                label={displayName}
                onClick={() => toggle(extractor)}
              />}
            />;
            })}
          </div>
          <RegistryPanelFooter align="end">
            <AppDialogButton onClick={() => void duplicate()} disabled={!selected || isDirty || saving} title={isDirty ? translate('component.contentExtractorManagerDialog.saveOrCancelChangesBeforeDuplicating') : undefined}><Copy className="h-3.5 w-3.5" /> {translate('common.duplicate')}</AppDialogButton>
            <AppDialogButton variant="danger" onClick={remove} disabled={!selected || saving}><Trash2 className="h-3.5 w-3.5" /> {translate('component.contentExtractorManagerDialog.delete')}</AppDialogButton>
          </RegistryPanelFooter>
        </section>
        <section className="theme-surface flex min-w-0 flex-col overflow-hidden rounded-xl border">
          <RegistryPanelHeader
            title={translate('component.contentExtractorManagerDialog.extractorSettings')}
            actions={<span
              title={availabilityTitle}
              className={`${selectedId !== 'new' && !runtimeConfigurationChanged && selected?.isAvailable
                ? 'theme-status-success-text'
                : selectedId !== 'new' && !runtimeConfigurationChanged
                  ? 'theme-status-warning-text'
                  : 'theme-text-muted'} flex min-w-0 max-w-[70%] shrink items-center gap-1.5 text-[10px] font-semibold`}
            >
              {selectedId !== 'new' && !runtimeConfigurationChanged && selected?.isAvailable
                ? <CircleCheck className="h-3.5 w-3.5 shrink-0" />
                : <CircleAlert className="h-3.5 w-3.5 shrink-0" />}
              <span className="truncate">{availabilityLabel}</span>
            </span>}
          />
          <div className="min-h-0 flex-1 space-y-4 overflow-y-auto p-4">
            <div className="grid grid-cols-1 gap-3 @md:grid-cols-[minmax(0,1fr)_110px_auto] @md:items-end">
              <label className="space-y-1">
                <ModifiedFieldLabel modified={selectedId !== 'new' && draft.name !== defaults?.name}>{translate('common.name')}</ModifiedFieldLabel>
                <input value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} className="theme-input ui-field-radius w-full border px-3 py-2" />
              </label>
              <label className="flex h-9 items-center gap-2 px-1">
                <input type="checkbox" checked={draft.enabled} onChange={(event) => setDraft({ ...draft, enabled: event.target.checked })} className="theme-checkbox h-4 w-4 rounded" />
                <ModifiedFieldLabel modified={selectedId !== 'new' && draft.enabled !== defaults?.enabled}>{translate('common.enabled')}</ModifiedFieldLabel>
              </label>
              <label className="space-y-1">
                <ModifiedFieldLabel modified={selectedId !== 'new' && draft.priority !== defaults?.priority}>{translate('common.priority')}</ModifiedFieldLabel>
                <input type="number" value={draft.priority} onChange={(event) => setDraft({ ...draft, priority: Number(event.target.value) || 0 })} className="theme-input ui-field-radius w-full border px-3 py-2 font-mono" />
              </label>
            </div>
            <label className="block space-y-1">
              <ModifiedFieldLabel modified={selectedId !== 'new' && draft.description !== defaults?.description}>{translate('common.description')}</ModifiedFieldLabel>
              <input value={draft.description} onChange={(event) => setDraft({ ...draft, description: event.target.value })} className="theme-input ui-field-radius w-full border px-3 py-2" />
            </label>
            <div className="theme-subtle-surface space-y-3 rounded-xl border p-3">
              <label className="block space-y-1">
                <span className="theme-text-muted block text-[10px] font-semibold">{translate('component.contentExtractorManagerDialog.describeExtractor')}</span>
                <textarea dir="auto"
                  value={authoringPrompt}
                  onChange={(event) => setAuthoringPrompt(event.target.value)}
                  placeholder={translate('component.contentExtractorManagerDialog.describeExtractorPlaceholder')}
                  className="theme-input ui-field-radius min-h-20 w-full resize-y border px-3 py-2"
                />
              </label>
              <div className="flex flex-wrap items-center justify-between gap-2">
                <span className="theme-text-muted text-[10px]">{translate('component.contentExtractorManagerDialog.aiCreatesLocalReviewableRecipe')}</span>
                {hasIntelligence
                  ? <AppDialogButton variant="primary" onClick={() => void generateRecipe()} disabled={!authoringPrompt.trim() || generating}>
                    <Sparkles className="h-3.5 w-3.5" /> {generating ? translate('component.contentExtractorManagerDialog.creating') : translate('component.contentExtractorManagerDialog.createWithAi')}
                  </AppDialogButton>
                  : <AppDialogButton onClick={onOpenIntelligence} disabled={!onOpenIntelligence}>
                    <Sparkles className="h-3.5 w-3.5" /> {translate('component.contentExtractorManagerDialog.connectAi')}
                  </AppDialogButton>}
              </div>
              {setupGuidance.length > 0 && <div className="theme-subtle-surface rounded-lg border p-2.5">
                <span className="theme-text-muted block text-[10px] font-semibold">{translate('component.contentExtractorManagerDialog.setup')}</span>
                <ul className="mt-1 list-disc space-y-1 ps-4 text-[10px]">
                  {setupGuidance.map((item) => <li key={item}>{item}</li>)}
                </ul>
              </div>}
            </div>
            <details className="theme-subtle-surface rounded-xl border p-3 text-[10px]">
              <summary className="theme-text-muted cursor-pointer font-semibold">{translate('component.contentExtractorManagerDialog.advanced')}</summary>
              <div className="mt-3 space-y-4">
                <div className="grid grid-cols-1 gap-3 @md:grid-cols-2">
                  <label className="space-y-1">
                    <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.acceptedInputs')}</span>
                    <MenuMultiSelect
                      values={recipeDraft.accepts}
                      onChange={(values) => setRecipeDraft({ ...recipeDraft, accepts: values as ExtractorInputKind[] })}
                      label={translate('component.contentExtractorManagerDialog.acceptedInputs')}
                      placeholder={translate('component.contentExtractorManagerDialog.chooseInputs')}
                      className="w-full"
                      options={EXTRACTOR_INPUT_OPTIONS.filter((option) => !option.disabled).map((option) => ({ value: option.value, label: option.label }))}
                    />
                  </label>
                  <label className="space-y-1">
                    <span className="theme-text-muted block font-semibold">{translate('common.output')}</span>
                    <MenuSelect
                      value={recipeDraft.output}
                      onChange={() => undefined}
                      label={translate('common.output')}
                      className="w-full"
                      options={EXTRACTOR_OUTPUT_OPTIONS.map((option) => ({ value: option.value, label: option.label }))}
                    />
                  </label>
                </div>
                <div className="space-y-2">
                  <div className="flex items-center justify-between gap-2">
                    <span className="theme-text-muted font-semibold">{translate('component.contentExtractorManagerDialog.commands')}</span>
                    <AppDialogButton type="button" onClick={() => setRecipeDraft((current) => ({ ...current, steps: [...current.steps, { ...emptyRecipe().steps[0], id: `step-${current.steps.length + 1}` }] }))}>
                      <Plus className="h-3.5 w-3.5" /> {translate('common.new')}
                    </AppDialogButton>
                  </div>
                  {recipeDraft.steps.map((step, index) => <div key={`${step.id}-${index}`} className="theme-surface space-y-3 rounded-lg border p-3">
                    <div className="grid grid-cols-1 gap-2 @md:grid-cols-[minmax(0,1fr)_120px_auto]">
                      <label className="space-y-1">
                        <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.stepId')}</span>
                        <input value={step.id} onChange={(event) => updateStep(index, { id: event.target.value })} className="theme-input ui-field-radius w-full border px-2.5 py-2 font-mono" />
                      </label>
                      <label className="space-y-1">
                        <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.timeLimit')}</span>
                        <input type="number" min={1} max={600} value={step.timeoutSeconds} onChange={(event) => updateStep(index, { timeoutSeconds: Number(event.target.value) || 1 })} className="theme-input ui-field-radius w-full border px-2.5 py-2 font-mono" />
                      </label>
                      <AppDialogButton variant="danger" className="self-end" onClick={() => setRecipeDraft((current) => ({ ...current, steps: current.steps.filter((_, stepIndex) => stepIndex !== index) }))} disabled={recipeDraft.steps.length === 1} title={translate('component.contentExtractorManagerDialog.removeCommand')}>
                        <Trash2 className="h-3.5 w-3.5" />
                      </AppDialogButton>
                    </div>
                    <label className="space-y-1">
                      <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.executable')}</span>
                      <span className="flex gap-2">
                        <input value={step.executable.path ?? ''} onChange={(event) => updateStepExecutable(index, { path: event.target.value || null })} placeholder={translate('component.contentExtractorManagerDialog.pathToExecutable')} className="theme-input ui-field-radius min-w-0 flex-1 border px-2.5 py-2 font-mono" />
                        <AppDialogButton type="button" onClick={() => void chooseStepExecutable(index)} title={translate('component.contentExtractorManagerDialog.chooseALocalExecutable')}>
                          <FolderOpen className="h-3.5 w-3.5" /> {translate('component.contentExtractorManagerDialog.choose')}
                        </AppDialogButton>
                      </span>
                    </label>
                    <label className="space-y-1">
                      <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.discoveryCommands')}</span>
                      <input value={step.executable.discover.join(', ')} onChange={(event) => updateStepExecutable(index, { discover: event.target.value.split(',').map((value) => value.trim()).filter(Boolean) })} placeholder={['pdftotext', 'zbarimg'].join(', ')} className="theme-input ui-field-radius w-full border px-2.5 py-2 font-mono" />
                    </label>
                    <label className="space-y-1">
                      <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.versionArguments')}</span>
                      <input value={step.executable.versionArguments.join(' ')} onChange={(event) => updateStepExecutable(index, { versionArguments: event.target.value.split(/\s+/).map((value) => value.trim()).filter(Boolean) })} className="theme-input ui-field-radius w-full border px-2.5 py-2 font-mono" />
                    </label>
                    <label className="space-y-1">
                      <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.arguments')}</span>
                      <textarea dir="auto" value={step.arguments.join('\n')} onChange={(event) => updateStep(index, { arguments: event.target.value.split('\n') })} className="theme-input ui-field-radius min-h-20 w-full resize-y border px-2.5 py-2 font-mono" />
                    </label>
                    <div className="grid grid-cols-1 gap-2 @md:grid-cols-3">
                      <label className="space-y-1">
                        <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.run')}</span>
                        <MenuSelect value={step.mode} onChange={(value) => updateStep(index, { mode: value as 'once' | 'each_input' })} label={translate('component.contentExtractorManagerDialog.run')} className="w-full" options={[
                          { value: 'once', label: translate('component.contentExtractorManagerDialog.once') },
                          { value: 'each_input', label: translate('component.contentExtractorManagerDialog.forEachInput') },
                        ]} />
                      </label>
                      <label className="space-y-1">
                        <span className="theme-text-muted block font-semibold">{translate('component.contentExtractorManagerDialog.capture')}</span>
                        <MenuSelect value={step.capture} onChange={(value) => updateStep(index, { capture: value as ExtractorCapture })} label={translate('component.contentExtractorManagerDialog.capture')} className="w-full" options={[
                          { value: 'stdout_text', label: translate('component.contentExtractorManagerDialog.standardOutput') },
                          { value: 'file_text', label: translate('component.contentExtractorManagerDialog.outputFileText') },
                          { value: 'pasted_json_v1', label: translate('component.contentExtractorManagerDialog.pastedJson') },
                          { value: 'ignore', label: translate('component.contentExtractorManagerDialog.ignoreOutput') },
                        ]} />
                      </label>
                      <label className="space-y-1">
                        <span className="theme-text-muted block font-semibold">{translate('component.binModal.fileExtension')}</span>
                        <input value={step.outputExtension ?? ''} onChange={(event) => updateStep(index, { outputExtension: event.target.value.replace(/^\./, '') || null })} className="theme-input ui-field-radius w-full border px-2.5 py-2 font-mono" />
                      </label>
                    </div>
                  </div>)}
                </div>
                <div className="space-y-2">
                  <div className="flex items-center justify-between gap-2">
                    <span className="theme-text-muted font-semibold">{translate('component.contentExtractorManagerDialog.resources')}</span>
                    <AppDialogButton type="button" onClick={() => setRecipeDraft((current) => ({ ...current, resources: [...current.resources, { id: `resource-${current.resources.length + 1}`, label: translate('component.contentExtractorManagerDialog.resource'), kind: 'file', required: true, path: null }] }))}>
                      <Plus className="h-3.5 w-3.5" /> {translate('common.new')}
                    </AppDialogButton>
                  </div>
                  {recipeDraft.resources.length === 0 && <p className="theme-text-muted">{translate('component.contentExtractorManagerDialog.noAdditionalResourcesAreRequired')}</p>}
                  {recipeDraft.resources.map((resource, index) => <div key={`${resource.id}-${index}`} className="theme-surface space-y-2 rounded-lg border p-3">
                    <div className="grid grid-cols-1 gap-2 @md:grid-cols-[minmax(0,1fr)_160px]">
                      <input value={resource.label} onChange={(event) => updateResource(index, { label: event.target.value })} aria-label={translate('component.contentExtractorManagerDialog.resourceName')} className="theme-input ui-field-radius border px-2.5 py-2" />
                      <input value={resource.id} onChange={(event) => updateResource(index, { id: event.target.value })} aria-label={translate('component.contentExtractorManagerDialog.resourceId')} className="theme-input ui-field-radius border px-2.5 py-2 font-mono" />
                    </div>
                    <div className="grid grid-cols-1 gap-2 @md:grid-cols-[minmax(0,1fr)_160px]">
                      <MenuSelect value={resource.kind} onChange={(value) => updateResource(index, { kind: value as 'file' | 'directory' })} label={translate('component.contentExtractorManagerDialog.resourceKind')} className="w-full" options={[
                        { value: 'file', label: translate('component.contentExtractorManagerDialog.resourceFile') },
                        { value: 'directory', label: translate('component.contentExtractorManagerDialog.resourceDirectory') },
                      ]} />
                      <span className="theme-text-muted flex items-center justify-between gap-2 font-semibold">
                        {resourceRequiredLabel}
                        <SettingsSwitch checked={resource.required} onClick={() => updateResource(index, { required: !resource.required })} label={resourceRequiredLabel} />
                      </span>
                    </div>
                    <span className="flex gap-2">
                      <input value={resource.path ?? ''} onChange={(event) => updateResource(index, { path: event.target.value || null })} aria-label={translate('component.contentExtractorManagerDialog.resourcePath')} className="theme-input ui-field-radius min-w-0 flex-1 border px-2.5 py-2 font-mono" />
                      <AppDialogButton type="button" onClick={() => void chooseResource(index)} title={translate('component.contentExtractorManagerDialog.chooseResource')}><FolderOpen className="h-3.5 w-3.5" /> {translate('component.contentExtractorManagerDialog.choose')}</AppDialogButton>
                      <AppDialogButton variant="danger" onClick={() => setRecipeDraft((current) => ({ ...current, resources: current.resources.filter((_, resourceIndex) => resourceIndex !== index) }))} title={translate('component.contentExtractorManagerDialog.removeResource')}><Trash2 className="h-3.5 w-3.5" /></AppDialogButton>
                    </span>
                  </div>)}
                </div>
                <div className="theme-divider flex flex-wrap items-start justify-between gap-3 border-t pt-3">
                  <div className="min-w-0 flex-1">
                    {testOutcome?.outcome === 'produced' && <textarea dir="auto" readOnly value={testOutcome.text} aria-label={translate('component.contentExtractorManagerDialog.testOutput')} className="theme-input ui-field-radius min-h-20 w-full resize-y border px-2.5 py-2" />}
                    {testOutcome?.outcome === 'no_output' && <p className="theme-text-muted">{translate('component.contentExtractorManagerDialog.testProducedNoText')}</p>}
                    {testOutcome?.outcome === 'failed' && <p className="theme-danger-text">{testOutcome.failure.message}</p>}
                  </div>
                  <AppDialogButton type="button" onClick={() => void testRecipe()} disabled={testing || !recipeCanSave}>
                    <ScanText className="h-3.5 w-3.5" /> {testing ? translate('component.contentExtractorManagerDialog.testing') : translate('component.contentExtractorManagerDialog.test')}
                  </AppDialogButton>
                </div>
              </div>
            </details>
            <details className="theme-subtle-surface rounded-xl border p-3 text-[10px]">
              <summary className="theme-text-muted cursor-pointer font-semibold">{translate('common.technicalDetails')}</summary>
              <dl className="mt-3 grid grid-cols-[110px_minmax(0,1fr)] gap-x-3 gap-y-2">
                <dt className="theme-text-muted">{translate('common.stableReference')}</dt><dd className="truncate font-mono">{selected?.stableRef ?? translate('component.contentExtractorManagerDialog.assignedWhenSaved')}</dd>
                <dt className="theme-text-muted">{translate('component.contentExtractorManagerDialog.recipeVersion')}</dt><dd className="font-mono">{recipeDraft.definitionVersion}</dd>
                <dt className="theme-text-muted">{translate('component.contentExtractorManagerDialog.revision')}</dt><dd>{selected?.revision ?? 1}</dd>
                <dt className="theme-text-muted">{translate('component.contentExtractorManagerDialog.runtimeVersion')}</dt><dd>{selected?.runtime.version ?? translate('component.contentExtractorManagerDialog.unavailable')}</dd>
              </dl>
              {selected && <AppDialogButton type="button" className="mt-3" onClick={() => void openAuthoringHistory()}>{translate('component.contentExtractorManagerDialog.viewAuthoringHistory')}</AppDialogButton>}
            </details>
          </div>
          <RegistryPanelFooter>
            <div>
              {selected?.isBuiltin && <AppDialogButton onClick={resetDraft} disabled={!differsFromDefaults || saving}><RotateCcw className="h-3.5 w-3.5" /> {translate('common.resetToDefault')}</AppDialogButton>}
            </div>
            <div className="flex items-center gap-2">
              <AppDialogButton onClick={cancelDraft} disabled={selectedId !== 'new' && !isDirty}>{translate('common.cancel')}</AppDialogButton>
              <AppDialogButton variant="primary" onClick={() => void save()} disabled={selectedId === null || saving || !recipeCanSave || (selectedId !== 'new' && !isDirty)}><SaveButtonContent isSaving={saving} /></AppDialogButton>
            </div>
          </RegistryPanelFooter>
        </section>
      </AppDialogBody>
      <AppDialogFooter align="between" className="shrink-0">
        <AppDialogButton onClick={restoreAll} disabled={saving}>
          <RotateCcw className="h-3.5 w-3.5" /> {translate('component.contentExtractorManagerDialog.reset')}
        </AppDialogButton>
        <AppDialogButton onClick={requestClose}>{translate('common.close')}</AppDialogButton>
      </AppDialogFooter>
    </>}
  </AppDialog>
  <ConfirmationDialog request={confirmation} onCancel={() => setConfirmation(null)} />
  <AppDialog
    isOpen={authoringHistory !== null}
    onClose={() => setAuthoringHistory(null)}
    labelledBy="extractor-authoring-history-title"
    panelClassName="theme-panel flex max-h-[80vh] w-full max-w-2xl flex-col overflow-hidden border shadow-2xl"
  >
    {({ requestClose }) => <>
      <AppDialogHeader onClose={requestClose}>
        <AppDialogHeading id="extractor-authoring-history-title" title={translate('component.contentExtractorManagerDialog.authoringHistory')} icon={<Sparkles />} />
      </AppDialogHeader>
      <AppDialogBody className="min-h-0 space-y-3 overflow-y-auto">
        {authoringHistory?.length === 0 && <p className="theme-text-muted text-xs">{translate('component.contentExtractorManagerDialog.noAuthoringHistory')}</p>}
        {authoringHistory?.map((session) => <section key={session.id} className="theme-subtle-surface rounded-xl border p-3 text-xs">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <strong>{session.provider
              ? translate('component.contentExtractorManagerDialog.createdWithProvider', { provider: session.provider })
              : translate('component.contentExtractorManagerDialog.createdManually')}</strong>
            <time className="theme-text-muted text-[10px]" dateTime={dateTimeAttribute(session.createdAt)} title={formatFullDateTime(session.createdAt)}>{formatRelativeTime(session.createdAt)}</time>
          </div>
          {session.originalPrompt && <div className="mt-3">
            <span className="theme-text-muted block text-[10px] font-semibold">{translate('component.contentExtractorManagerDialog.originalRequest')}</span>
            <p dir="auto" className="mt-1 whitespace-pre-wrap">{session.originalPrompt}</p>
          </div>}
          {session.messages.length > 0 && <div className="mt-3 space-y-2">
            <span className="theme-text-muted block text-[10px] font-semibold">{translate('component.contentExtractorManagerDialog.conversation')}</span>
            {session.messages.map((message, index) => <div key={`${message.createdAt}-${index}`} className="theme-surface rounded-lg border p-2.5">
              <div className="theme-text-muted mb-1 flex items-center justify-between gap-2 text-[10px]">
                <span>{authoringRoleLabel(message.role)}</span>
                <time dateTime={dateTimeAttribute(message.createdAt)} title={formatFullDateTime(message.createdAt)}>{formatRelativeTime(message.createdAt)}</time>
              </div>
              <p dir="auto" className="whitespace-pre-wrap">{message.content}</p>
            </div>)}
          </div>}
        </section>)}
      </AppDialogBody>
      <AppDialogFooter align="end"><AppDialogButton onClick={requestClose}>{translate('common.close')}</AppDialogButton></AppDialogFooter>
    </>}
  </AppDialog>
  </>;
}
