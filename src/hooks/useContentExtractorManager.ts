import { useEffect, useLayoutEffect, useMemo, useState } from 'react';

import { analysisApi } from '../api/analysis';
import { translate } from '../localization/runtime';
import type { IntelligenceConnection } from '../types';
import { errorMessage } from '../utils/errors';
import { safeInvoke as invoke } from '../utils/tauri';
import type { ConfirmationDialogRequest } from '../components/ConfirmationDialog';
import { useToast } from '../components/ToastProvider';
import { useNewItemSelection } from './useNewItemSelection';
import {
  emptyRecipe,
  toInput,
  type ContentExtractor,
  type ExtractorAuthoringManifest,
  type ExtractorAuthoringSession,
  type ExtractorInput,
  type ExtractorRecipe,
  type ExtractorRuntimeStatus,
  type ExtractorTestOutcome,
} from '../components/contentExtractorModel';
import {
  canSaveExtractorRecipe,
  extractorDraftIsDirty,
  resetExtractorRecipePreservingLocalPaths,
  visibleContentExtractors,
} from '../components/contentExtractorPolicy';
import { useExtractorAiAuthoring } from './useExtractorAiAuthoring';

export function useContentExtractorManager({
  isOpen,
  onChanged,
  ocrEnabled,
  transcriptionsEnabled,
}: {
  isOpen: boolean;
  onChanged?: () => void;
  ocrEnabled: boolean;
  transcriptionsEnabled: boolean;
}) {
  const { showToast } = useToast();
  const [extractors, setExtractors] = useState<ContentExtractor[]>([]);
  const [loading, setLoading] = useState(true);
  const [runtimeLoadingId, setRuntimeLoadingId] = useState<number | null>(null);
  const [selectedId, setSelectedId] = useState<number | 'new' | null>(null);
  const [draft, setDraft] = useState<ExtractorInput>(toInput());
  const [recipeDraft, setRecipeDraft] = useState<ExtractorRecipe>(emptyRecipe());
  const [authoring, setAuthoring] = useState<ExtractorAuthoringManifest | null>(null);
  const [authoringPrompt, setAuthoringPrompt] = useState('');
  const [setupGuidance, setSetupGuidance] = useState<string[]>([]);
  const [connections, setConnections] = useState<IntelligenceConnection[]>([]);
  const [testing, setTesting] = useState(false);
  const [testOutcome, setTestOutcome] = useState<ExtractorTestOutcome | null>(null);
  const [authoringHistory, setAuthoringHistory] = useState<ExtractorAuthoringSession[] | null>(null);
  const [saving, setSaving] = useState(false);
  const [confirmation, setConfirmation] = useState<ConfirmationDialogRequest | null>(null);
  const aiAuthoring = useExtractorAiAuthoring({
    draft,
    recipe: recipeDraft,
    prompt: authoringPrompt,
    setDraft,
    setRecipe: setRecipeDraft,
    setAuthoring,
    setSetupGuidance,
  });
  const visibleExtractors = useMemo(
    () => visibleContentExtractors(extractors, { ocrEnabled, transcriptionsEnabled }),
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
    setLoading(true);
    try {
      const loaded = await analysisApi.listExtractors<ContentExtractor>();
      setExtractors(loaded);
      const visible = visibleContentExtractors(loaded, { ocrEnabled, transcriptionsEnabled });
      setSelectedId((current) => visible.some(({ id }) => id === current) ? current : visible[0]?.id ?? null);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (isOpen) {
      void load().catch((error) => showToast({ tone: 'error', message: errorMessage(error) }));
      void invoke<IntelligenceConnection[]>('get_intelligence_connections')
        .then(setConnections)
        .catch(() => setConnections([]));
    }
  }, [isOpen, ocrEnabled, transcriptionsEnabled]);
  useEffect(() => {
    if (!isOpen || !selected) return undefined;
    let cancelled = false;
    setRuntimeLoadingId(selected.id);
    void analysisApi.extractorRuntime<ExtractorRuntimeStatus>(selected.stableRef)
      .then((runtime) => {
        if (cancelled) return;
        setExtractors((current) => current.map((extractor) => (
          extractor.id === selected.id ? { ...extractor, runtime } : extractor
        )));
      })
      .catch(() => { /* Availability remains authoritative when version inspection fails. */ })
      .finally(() => {
        if (!cancelled) setRuntimeLoadingId(null);
      });
    return () => { cancelled = true; };
  }, [isOpen, selected?.id, selected?.stableRef, selected?.revision]);
  useEffect(() => {
    setSelectedId((current) => visibleExtractors.some(({ id }) => id === current)
      ? current
      : visibleExtractors[0]?.id ?? null);
  }, [visibleExtractors]);
  useLayoutEffect(() => {
    setDraft(selectedId === 'new' ? toInput() : toInput(selected));
    setRecipeDraft(selectedId === 'new' ? emptyRecipe() : selected?.recipe ?? emptyRecipe());
    setAuthoringPrompt('');
    aiAuthoring.clear();
    setTestOutcome(null);
  }, [selected, selectedId]);

  const baseline = selectedId === 'new' ? toInput() : selected ? toInput(selected) : null;
  const baselineRecipe = selectedId === 'new' ? emptyRecipe() : selected?.recipe ?? null;
  const isDirty = extractorDraftIsDirty({
    draft,
    recipe: recipeDraft,
    baselineDraft: baseline,
    baselineRecipe,
    hasAuthoredChanges: authoring !== null,
  });
  const defaults = selected?.defaults;
  const defaultDraft = selected && defaults ? {
    ...toInput(selected),
    ...defaults,
    executablePath: selected.executablePath,
    modelPath: selected.modelPath,
  } : null;
  const defaultRecipeDraft = selected?.defaultRecipe
    ? resetExtractorRecipePreservingLocalPaths(selected.recipe, selected.defaultRecipe)
    : null;
  const differsFromDefaults = defaultDraft !== null && (
    JSON.stringify(draft) !== JSON.stringify(defaultDraft)
    || JSON.stringify(recipeDraft) !== JSON.stringify(defaultRecipeDraft)
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
  const recipeCanSave = canSaveExtractorRecipe(recipeDraft);


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
      aiAuthoring.clear();
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
    if (defaultRecipeDraft) setRecipeDraft(defaultRecipeDraft);
    aiAuthoring.clear();
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

  return {
    authoringHistory,
    authoringPrompt,
    availabilityLabel,
    availabilityTitle,
    beginNew,
    cancelDraft,
    chooseResource,
    chooseStepExecutable,
    confirmation,
    defaults,
    differsFromDefaults,
    diagnostic: aiAuthoring.diagnostic,
    draft,
    duplicate,
    generateRecipe: aiAuthoring.generate,
    generating: aiAuthoring.generating,
    hasIntelligence,
    isDirty,
    loading,
    openAuthoringHistory,
    recipeCanSave,
    recipeDraft,
    repairRecipe: aiAuthoring.repair,
    repairing: aiAuthoring.repairing,
    remove,
    resetDraft,
    restoreAll,
    runtimeConfigurationChanged,
    runtimeLoadingId,
    save,
    saving,
    selected,
    selectedId,
    selectExtractor,
    setAuthoringHistory,
    setAuthoringPrompt,
    setConfirmation,
    setDraft,
    setRecipeDraft,
    setupGuidance,
    testOutcome,
    testRecipe,
    testing,
    toggle,
    visibleExtractors,
  };
}
