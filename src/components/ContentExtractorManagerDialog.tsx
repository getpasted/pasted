import { useEffect, useLayoutEffect, useMemo, useState } from 'react';
import { CircleAlert, CircleCheck, Copy, FolderOpen, Plus, RotateCcw, ScanText, Trash2 } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
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
import { MenuSelect } from './MenuSelect';
import { translate } from '../localization/runtime';
import { localizedBuiltinDescription, localizedBuiltinName } from '../localization/presentation';

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

const IMAGE_ENGINES = ['macos-vision-v1', 'tesseract-cli-v1'];
const FILE_ENGINES = ['whisper-cpp-cli-v1'];
const CUSTOM_COMMAND_ENGINE = 'custom-command-v1';

const EXTRACTOR_METHOD_OPTIONS = [
  { value: 'macos-vision-v1', get label() { return translate('component.contentExtractorManagerDialog.appleVision'); } },
  { value: 'tesseract-cli-v1', get label() { return translate('component.contentExtractorManagerDialog.tesseract'); } },
  { value: 'whisper-cpp-cli-v1', label: 'Whisper.cpp' },
  { value: CUSTOM_COMMAND_ENGINE, get label() { return translate('component.contentExtractorManagerDialog.customCommand'); } },
] as const;

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
    engine: CUSTOM_COMMAND_ENGINE,
    executablePath: null,
    modelPath: null,
    inputContract: 'image',
    outputContract: 'searchable_text',
    enabled: false,
    priority: 100,
  };
}

export function ContentExtractorManagerDialog({
  isOpen,
  onClose,
  onChanged,
  ocrEnabled,
  transcriptionsEnabled,
}: {
  isOpen: boolean;
  onClose: () => void;
  onChanged?: () => void;
  ocrEnabled: boolean;
  transcriptionsEnabled: boolean;
}) {
  const { showToast } = useToast();
  const [extractors, setExtractors] = useState<ContentExtractor[]>([]);
  const [selectedId, setSelectedId] = useState<number | 'new' | null>(null);
  const [draft, setDraft] = useState<ExtractorInput>(toInput());
  const [saving, setSaving] = useState(false);
  const [confirmation, setConfirmation] = useState<ConfirmationDialogRequest | null>(null);
  const visibleExtractors = useMemo(
    () => extractors.filter(({ inputContract }) => (
      (inputContract !== 'image' || ocrEnabled)
      && (inputContract !== 'file_references' || transcriptionsEnabled)
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
    const loaded = await invoke<ContentExtractor[]>('get_content_extractors');
    setExtractors(loaded);
    const visible = loaded.filter(({ inputContract }) => (
      (inputContract !== 'image' || ocrEnabled)
      && (inputContract !== 'file_references' || transcriptionsEnabled)
    ));
    setSelectedId((current) => visible.some(({ id }) => id === current) ? current : visible[0]?.id ?? null);
  };

  useEffect(() => {
    if (isOpen) void load();
  }, [isOpen, ocrEnabled, transcriptionsEnabled]);
  useEffect(() => {
    setSelectedId((current) => visibleExtractors.some(({ id }) => id === current)
      ? current
      : visibleExtractors[0]?.id ?? null);
  }, [visibleExtractors]);
  useLayoutEffect(() => setDraft(selectedId === 'new' ? toInput() : toInput(selected)), [selected, selectedId]);

  const baseline = selectedId === 'new' ? toInput() : selected ? toInput(selected) : null;
  const isDirty = baseline !== null && JSON.stringify(draft) !== JSON.stringify(baseline);
  const defaults = selected?.defaults;
  const defaultDraft = selected && defaults ? { ...toInput(selected), ...defaults } : null;
  const differsFromDefaults = defaultDraft !== null && JSON.stringify(draft) !== JSON.stringify(defaultDraft);
  const runtimeConfigurationChanged = selected !== undefined
    && (draft.engine !== selected.engine
      || draft.executablePath !== selected.executablePath
      || draft.modelPath !== selected.modelPath);
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
    if (selected) setDraft(toInput(selected));
  };

  const changeInputContract = (inputContract: string) => {
    let engine = draft.engine;
    let modelPath = draft.modelPath;
    if (inputContract === 'image' && FILE_ENGINES.includes(engine)) {
      engine = 'tesseract-cli-v1';
      modelPath = null;
    }
    if (inputContract === 'file_references' && IMAGE_ENGINES.includes(engine)) engine = 'whisper-cpp-cli-v1';
    setDraft({ ...draft, inputContract, engine, modelPath });
  };

  const changeMethod = (engine: string) => {
    const inputContract = IMAGE_ENGINES.includes(engine)
      ? 'image'
      : FILE_ENGINES.includes(engine)
        ? 'file_references'
        : draft.inputContract;
    setDraft({
      ...draft,
      engine,
      inputContract,
      executablePath: engine === draft.engine ? draft.executablePath : null,
      modelPath: engine === 'whisper-cpp-cli-v1' ? draft.modelPath : null,
    });
  };

  const save = async () => {
    setSaving(true);
    try {
      const saved = selectedId === 'new'
        ? await invoke<ContentExtractor>('create_content_extractor', { input: draft })
        : await invoke<ContentExtractor>('update_content_extractor_definition', { id: selectedId, input: draft });
      await load();
      setSelectedId(saved.id);
      onChanged?.();
      showToast({ tone: 'success', message: translate('component.contentExtractorManagerDialog.nameSaved', { name: saved.name }) });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    } finally {
      setSaving(false);
    }
  };

  const chooseModel = async () => {
    try {
      const modelPath = await invoke<string | null>('choose_extractor_model_file');
      if (modelPath) setDraft((current) => ({ ...current, modelPath }));
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    }
  };

  const chooseExecutable = async () => {
    try {
      const executablePath = await invoke<string | null>('choose_extractor_executable');
      if (executablePath) setDraft((current) => ({ ...current, executablePath }));
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    }
  };

  const restoreAllConfirmed = async () => {
    try {
      const restored = await invoke<ContentExtractor[]>('restore_default_content_extractors');
      setExtractors(restored);
      setSelectedId(restored[0]?.id ?? null);
      onChanged?.();
      showToast({ tone: 'success', get message() { return translate('component.contentExtractorManagerDialog.builtInExtractorsRestored'); } });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
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
      showToast({ tone: 'error', message: String(error) });
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
      showToast({ tone: 'error', message: String(error) });
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
      showToast({ tone: 'error', message: String(error) });
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
          <RegistryPanelHeader title={translate('component.contentExtractorManagerDialog.extractors')} actions={ocrEnabled || transcriptionsEnabled ? <AppDialogButton onClick={beginNew} className="h-7 min-h-7 px-2.5"><Plus className="h-3.5 w-3.5" /> {translate('common.new')}</AppDialogButton> : undefined} />
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
            <div className="grid grid-cols-1 gap-3 @md:grid-cols-[minmax(0,1fr)_110px]">
              <label className="space-y-1">
                <ModifiedFieldLabel modified={selectedId !== 'new' && draft.name !== defaults?.name}>{translate('common.name')}</ModifiedFieldLabel>
                <input value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} className="theme-input ui-field-radius w-full border px-3 py-2" />
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
            <div className="grid grid-cols-1 gap-3 @md:grid-cols-2">
              <label className="space-y-1">
                <span className="theme-text-muted block text-[10px] font-semibold">{translate('component.contentExtractorManagerDialog.inputClipType')}</span>
                <select
                  value={draft.inputContract}
                  disabled={selectedId === null || selected?.isBuiltin}
                  onChange={(event) => changeInputContract(event.target.value)}
                  title={translate('component.contentExtractorManagerDialog.theClipTypeThisExtractorAcceptsTextClipsAreAlreadySearchable')}
                  className="theme-input ui-field-radius w-full border px-3 py-2 disabled:opacity-60"
                >
                  {EXTRACTOR_INPUT_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value} disabled={option.disabled}>
                      {option.label}{option.disabled ? translate('component.contentExtractorManagerDialog.alreadySearchable') : ''}
                    </option>
                  ))}
                </select>
              </label>
              <label className="space-y-1">
                <span className="theme-text-muted block text-[10px] font-semibold">{translate('common.output')}</span>
                <select
                  value={draft.outputContract}
                  disabled={selectedId === null || selected?.isBuiltin}
                  onChange={(event) => setDraft({ ...draft, outputContract: event.target.value })}
                  title={translate('component.contentExtractorManagerDialog.theRepresentationThisExtractorAddsToTheClip')}
                  className="theme-input ui-field-radius w-full border px-3 py-2 disabled:opacity-60"
                >
                  {EXTRACTOR_OUTPUT_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>{option.label}</option>
                  ))}
                </select>
              </label>
            </div>
            <div className="grid grid-cols-1 gap-3 @md:grid-cols-[180px_minmax(0,1fr)]">
              <div className="space-y-1">
                <span className="theme-text-muted block text-[10px] font-semibold">{translate('component.contentExtractorManagerDialog.method')}</span>
                <MenuSelect
                  value={draft.engine}
                  options={EXTRACTOR_METHOD_OPTIONS.map((option) => ({ ...option }))}
                  onChange={changeMethod}
                  label={translate('component.contentExtractorManagerDialog.extractorMethod')}
                  disabled={selected?.isBuiltin}
                  className="w-full"
                />
              </div>
              <label className="space-y-1">
                <span className="theme-text-muted block text-[10px] font-semibold">{translate('component.contentExtractorManagerDialog.runtimeLocation')}</span>
                <span className="flex gap-2">
                  <input
                    value={draft.engine === 'macos-vision-v1'
                    ? translate('component.contentExtractorManagerDialog.macosVisionFramework')
                      : draft.executablePath ?? (selected?.engine === draft.engine ? selected.runtime.location ?? '' : '')}
                    disabled={draft.engine === 'macos-vision-v1'}
                    onChange={(event) => setDraft({ ...draft, executablePath: event.target.value || null })}
                    placeholder={draft.engine === CUSTOM_COMMAND_ENGINE ? translate('component.contentExtractorManagerDialog.pathToExecutable') : translate('component.contentExtractorManagerDialog.automaticDiscovery')}
                    className="theme-input ui-field-radius min-w-0 flex-1 border px-3 py-2 font-mono disabled:opacity-60"
                  />
                  {draft.engine !== 'macos-vision-v1' && <AppDialogButton type="button" onClick={() => void chooseExecutable()} title={translate('component.contentExtractorManagerDialog.chooseALocalExecutable')}>
                    <FolderOpen className="h-3.5 w-3.5" /> {translate('component.contentExtractorManagerDialog.choose')}
                  </AppDialogButton>}
                </span>
                <span className="theme-text-muted block text-[10px]">
                  {draft.engine === 'macos-vision-v1'
                    ? translate('component.contentExtractorManagerDialog.providedByMacos')
                    : draft.executablePath
                      ? translate('component.contentExtractorManagerDialog.usingTheSelectedExecutable')
                      : draft.engine === CUSTOM_COMMAND_ENGINE
                        ? translate('component.contentExtractorManagerDialog.chooseAnExecutableThatSupportsTheCustomExtractorProtocol')
                        : selected?.runtime.location
                          ? translate('component.contentExtractorManagerDialog.discoveredAutomaticallyAtLocation', { location: selected.runtime.location })
                          : translate('component.contentExtractorManagerDialog.discoveredAutomatically')}
                </span>
                {draft.executablePath && draft.engine !== CUSTOM_COMMAND_ENGINE && <AppDialogButton type="button" onClick={() => setDraft({ ...draft, executablePath: null })}>{translate('component.contentExtractorManagerDialog.useAutomaticDiscovery')}</AppDialogButton>}
              </label>
            </div>
            <div className="theme-subtle-surface space-y-3 rounded-xl border p-3">
              <div>
                <span className="theme-text-muted block text-[10px] font-semibold">{translate('component.contentExtractorManagerDialog.resources')}</span>
                {draft.engine !== 'whisper-cpp-cli-v1' && <span className="theme-text-muted text-[10px]">{translate('component.contentExtractorManagerDialog.noAdditionalResourcesAreRequired')}</span>}
              </div>
              {draft.engine === 'whisper-cpp-cli-v1' && <label className="block space-y-1">
                <span className="theme-text-muted font-semibold">{translate('component.contentExtractorManagerDialog.model')}</span>
                <span className="flex gap-2">
                  <input value={draft.modelPath ?? ''} onChange={(event) => setDraft({ ...draft, modelPath: event.target.value || null })} placeholder="/path/to/ggml-model.bin" className="theme-input ui-field-radius min-w-0 flex-1 border px-3 py-2 font-mono" />
                  <AppDialogButton type="button" onClick={() => void chooseModel()} title={translate('component.contentExtractorManagerDialog.chooseALocalWhisperModelFile')}>
                    <FolderOpen className="h-3.5 w-3.5" /> {translate('component.contentExtractorManagerDialog.choose')}
                  </AppDialogButton>
                </span>
                <span className="theme-text-muted block text-[10px]">{translate('component.contentExtractorManagerDialog.chooseALocalWhisperCppGgmlModelFileModelDownloadsAreNot')}</span>
              </label>}
              {selected?.engine === draft.engine && selected.runtime.dependencies.map((dependency) => <div key={dependency.name} className="flex min-w-0 items-start justify-between gap-3 text-[10px]">
                <span className="theme-text-muted font-semibold">{dependency.name}</span>
                <span className={`${dependency.isAvailable ? 'theme-status-success-text' : 'theme-status-warning-text'} min-w-0 truncate text-right`} title={dependency.unavailableReason ?? dependency.location ?? undefined}>
                  {dependency.isAvailable ? dependency.version ?? dependency.location ?? translate('component.contentExtractorManagerDialog.available') : dependency.unavailableReason}
                </span>
              </div>)}
            </div>
            {draft.engine === CUSTOM_COMMAND_ENGINE && <div className="theme-subtle-surface rounded-xl border p-3 text-[10px] leading-relaxed">
              {translate('component.contentExtractorManagerDialog.customCommandProtocolDescription', { versionFlag: '--version', requestCommand: '--pasted-extract-v1 <request.json>', outputField: 'text' })}</div>}
            <details className="theme-subtle-surface rounded-xl border p-3 text-[10px]">
              <summary className="theme-text-muted cursor-pointer font-semibold">{translate('common.technicalDetails')}</summary>
              <dl className="mt-3 grid grid-cols-[110px_minmax(0,1fr)] gap-x-3 gap-y-2">
                <dt className="theme-text-muted">{translate('common.stableReference')}</dt><dd className="truncate font-mono">{selected?.stableRef ?? translate('component.contentExtractorManagerDialog.assignedWhenSaved')}</dd>
                <dt className="theme-text-muted">{translate('component.contentExtractorManagerDialog.engineContract')}</dt><dd className="font-mono">{draft.engine}</dd>
                <dt className="theme-text-muted">{translate('component.contentExtractorManagerDialog.revision')}</dt><dd>{selected?.revision ?? 1}</dd>
                <dt className="theme-text-muted">{translate('component.contentExtractorManagerDialog.runtimeVersion')}</dt><dd>{selected?.runtime.version ?? translate('component.contentExtractorManagerDialog.unavailable')}</dd>
              </dl>
              <p className="theme-text-muted mt-3 leading-relaxed">{translate('component.contentExtractorManagerDialog.engineContractsIdentifyTheRuntimeAdapterAndProtocolVersionAndAreManaged')}</p>
            </details>
            <label className="flex items-center gap-2">
              <input type="checkbox" checked={draft.enabled} onChange={(event) => setDraft({ ...draft, enabled: event.target.checked })} className="theme-checkbox h-4 w-4 rounded" />
              <ModifiedFieldLabel modified={selectedId !== 'new' && draft.enabled !== defaults?.enabled}>{translate('common.enabled')}</ModifiedFieldLabel>
            </label>
            {draft.engine === CUSTOM_COMMAND_ENGINE && draft.enabled && <p className="theme-status-warning rounded-lg border px-3 py-2 text-[10px] leading-relaxed">{translate('component.contentExtractorManagerDialog.enabledCustomCommandsMayRunAutomaticallyForMatchingClipsAndReceiveTheir')}</p>}
          </div>
          <RegistryPanelFooter>
            <div>
              {selected?.isBuiltin && <AppDialogButton onClick={resetDraft} disabled={!differsFromDefaults || saving}><RotateCcw className="h-3.5 w-3.5" /> {translate('common.resetToDefault')}</AppDialogButton>}
            </div>
            <div className="flex items-center gap-2">
              <AppDialogButton onClick={cancelDraft} disabled={selectedId !== 'new' && !isDirty}>{translate('common.cancel')}</AppDialogButton>
              <AppDialogButton variant="primary" onClick={() => void save()} disabled={selectedId === null || saving || (draft.engine === CUSTOM_COMMAND_ENGINE && !draft.executablePath) || (selectedId !== 'new' && !isDirty)}><SaveButtonContent isSaving={saving} /></AppDialogButton>
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
  </>;
}
