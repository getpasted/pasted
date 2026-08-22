import { useEffect, useLayoutEffect, useMemo, useState } from 'react';

import { analysisApi } from '../api/analysis';
import type { ConfirmationDialogRequest } from '../components/ConfirmationDialog';
import {
  classifierDraftIsDirty,
  classifierModifiedFields,
  emptyClassifierInput,
  nextClassifierSelection,
  normalizedClassifierInput,
  toClassifierInput,
  type ClassifierInput,
  type ContentClassifier,
} from '../components/classifierModel';
import { useToast } from '../components/ToastProvider';
import { translate } from '../localization/runtime';
import { errorMessage } from '../utils/errors';
import { safeInvoke as invoke } from '../utils/tauri';
import { useNewItemSelection } from './useNewItemSelection';

interface ClassificationResult {
  formatVersion: number;
  policy: 'capture' | 'background' | 'interactive' | 'rescan';
  through: 'inspect' | 'extract' | 'classify' | 'suggest';
  outcome: 'matched' | 'no_match' | 'failed';
  matched: boolean;
  failure: { code: string; message: string } | null;
}

export function useClassifierManager({
  isOpen,
  refreshContentTypes,
  refreshGroups,
}: {
  isOpen: boolean;
  refreshContentTypes: () => unknown | Promise<unknown>;
  refreshGroups: () => unknown | Promise<unknown>;
}) {
  const { showToast } = useToast();
  const [classifiers, setClassifiers] = useState<ContentClassifier[]>([]);
  const [selectedId, setSelectedId] = useState<number | 'new' | null>(null);
  const newClassifierInput = () => emptyClassifierInput(
    translate('component.settingsAnalysisPanel.customClassifier'),
  );
  const [draft, setDraft] = useState<ClassifierInput>(newClassifierInput);
  const [patternsText, setPatternsText] = useState('^.+$');
  const [sample, setSample] = useState('');
  const [sampleMatched, setSampleMatched] = useState<boolean | null>(null);
  const [saving, setSaving] = useState(false);
  const [togglingId, setTogglingId] = useState<number | null>(null);
  const [confirmation, setConfirmation] = useState<ConfirmationDialogRequest | null>(null);

  const selected = useMemo(
    () => typeof selectedId === 'number'
      ? classifiers.find((classifier) => classifier.id === selectedId)
      : undefined,
    [classifiers, selectedId],
  );
  const { beginNew: beginNewClassifier, cancelNew: cancelNewClassifier } = useNewItemSelection({
    selectedId,
    setSelectedId,
    itemIds: classifiers.map(({ id }) => id),
    emptySelection: null,
  });

  const load = async () => {
    const loaded = await analysisApi.listClassifiers<ContentClassifier>();
    setClassifiers(loaded);
    setSelectedId((current) => nextClassifierSelection(loaded, current));
    return loaded;
  };

  useEffect(() => {
    if (isOpen) {
      void load().catch((error) => {
        showToast({ tone: 'error', message: errorMessage(error) });
      });
    }
  }, [isOpen]);
  useLayoutEffect(() => {
    const next = selectedId === 'new' || !selected
      ? newClassifierInput()
      : toClassifierInput(selected);
    setDraft(next);
    setPatternsText(next.patterns.join('\n'));
    setSampleMatched(null);
  }, [selected, selectedId]);

  const currentInput = normalizedClassifierInput(draft, patternsText);
  const comparisonInput = selectedId === 'new'
    ? newClassifierInput()
    : selected?.is_builtin ? selected.defaults : selected ? toClassifierInput(selected) : null;
  const modified = classifierModifiedFields(currentInput, comparisonInput, selectedId === 'new');
  const hasModifiedFields = Object.values(modified).some(Boolean);
  const editorBaseline = selectedId === 'new'
    ? newClassifierInput()
    : selected ? toClassifierInput(selected) : null;
  const isEditorDirty = classifierDraftIsDirty(currentInput, editorBaseline);

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
    if (!isEditorDirty) {
      void action();
      return;
    }
    requestConfirmation({
      get title() { return translate('common.discardChangesQuestion'); },
      get description() { return translate('component.settingsAnalysisPanel.unsavedChangesToThisClassifierWillBeLost'); },
      confirmLabel: translate('component.appDialog.discard'),
      tone: 'danger',
      onConfirm: action,
    });
  };

  const selectClassifier = (id: number) => discardDraftThen(() => setSelectedId(id));
  const beginNew = () => discardDraftThen(beginNewClassifier);

  const cancelDraft = () => {
    if (selectedId === 'new') {
      cancelNewClassifier();
      return;
    }
    if (!selected) return;
    const restored = toClassifierInput(selected);
    setDraft(restored);
    setPatternsText(restored.patterns.join('\n'));
    setSampleMatched(null);
  };

  const resetSelectedDraft = () => {
    if (!selected?.is_builtin || !selected.defaults) return;
    setDraft({ ...selected.defaults, patterns: [...selected.defaults.patterns] });
    setPatternsText(selected.defaults.patterns.join('\n'));
    setSampleMatched(null);
  };

  const save = async () => {
    setSaving(true);
    try {
      const saved = selectedId === 'new'
        ? await invoke<ContentClassifier>('create_content_classifier', { input: currentInput })
        : await invoke<ContentClassifier>('update_content_classifier', { id: selectedId, input: currentInput });
      await load();
      setSelectedId(saved.id);
      showToast({ tone: 'success', message: translate('component.settingsAnalysisPanel.nameSaved', { name: saved.name }) });
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    } finally {
      setSaving(false);
    }
  };

  const removeConfirmed = async () => {
    if (typeof selectedId !== 'number' || !selected) return;
    try {
      await invoke('delete_content_classifier', { id: selectedId });
      const remaining = await load();
      setSelectedId((current) => current === selectedId ? remaining[0]?.id ?? 'new' : current);
      showToast({ tone: 'success', message: translate('component.settingsAnalysisPanel.nameDeletedResetCanRecoverShippedClassifiers', { name: selected.name }) });
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    }
  };

  const remove = () => {
    if (!selected) return;
    requestConfirmation({
      get title() { return translate('component.settingsAnalysisPanel.deleteClassifier'); },
      description: selected.name,
      details: selected.is_builtin
        ? translate('component.settingsAnalysisPanel.removingBuiltinClassifierCanBeRecovered')
        : translate('component.settingsAnalysisPanel.removingCustomClassifierIsPermanent'),
      confirmLabel: translate('component.settingsAnalysisPanel.deleteClassifier'),
      tone: 'danger',
      onConfirm: removeConfirmed,
    });
  };

  const duplicate = async () => {
    if (!selected || isEditorDirty) return;
    try {
      const created = await invoke<ContentClassifier>('duplicate_content_classifier', {
        reference: selected.stable_ref,
        name: `${selected.name} Copy`,
      });
      await load();
      setSelectedId(created.id);
      showToast({ tone: 'success', message: translate('component.settingsAnalysisPanel.nameCreated', { name: created.name }) });
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    }
  };

  const restoreDefaultsConfirmed = async () => {
    try {
      const [restored] = await Promise.all([
        invoke<ContentClassifier[]>('restore_default_content_classifiers'),
        invoke('restore_default_content_types'),
        invoke('restore_default_content_type_groups'),
      ]);
      await Promise.all([refreshContentTypes(), refreshGroups()]);
      setClassifiers(restored);
      setSelectedId(restored[0]?.id ?? 'new');
      showToast({ tone: 'success', get message() { return translate('component.settingsAnalysisPanel.builtInContentTypesAndClassifiersResetCustomDefinitionsWerePreserved'); } });
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    }
  };

  const restoreDefaults = () => discardDraftThen(() => requestConfirmation({
    get title() { return translate('component.settingsAnalysisPanel.resetShippedClassifierDefinitions'); },
    get description() { return translate('component.settingsAnalysisPanel.shippedContentTypesContentTypeGroupsAndClassifiersReturnToTheirDefaults'); },
    details: translate('component.settingsAnalysisPanel.customDefinitionsRemainUnchanged'),
    confirmLabel: translate('common.reset'),
    onConfirm: restoreDefaultsConfirmed,
  }));

  const toggleConfirmed = async (classifier: ContentClassifier) => {
    setTogglingId(classifier.id);
    try {
      const enabled = !classifier.enabled;
      await invoke('set_library_item_enabled', {
        kind: 'classifier',
        stableRef: classifier.stable_ref,
        enabled,
      });
      setClassifiers((current) => current.map((item) => (
        item.id === classifier.id ? { ...item, enabled } : item
      )));
    } catch (error) {
      showToast({ tone: 'error', message: errorMessage(error) });
    } finally {
      setTogglingId(null);
    }
  };

  const toggle = (classifier: ContentClassifier) => {
    if (selectedId === classifier.id) {
      discardDraftThen(() => toggleConfirmed(classifier));
      return;
    }
    void toggleConfirmed(classifier);
  };

  const test = async () => {
    try {
      const result = await analysisApi.testClassifier<ClassificationResult>(currentInput, sample);
      if (result.outcome === 'failed') {
        throw new Error(result.failure?.message ?? translate('component.settingsAnalysisPanel.classificationFailed'));
      }
      setSampleMatched(result.matched);
    } catch (error) {
      setSampleMatched(false);
      showToast({ tone: 'error', message: errorMessage(error) });
    }
  };

  return {
    beginNew,
    cancelDraft,
    classifiers,
    confirmation,
    draft,
    duplicate,
    hasModifiedFields,
    isEditorDirty,
    modified,
    patternsText,
    remove,
    resetSelectedDraft,
    restoreDefaults,
    sample,
    sampleMatched,
    save,
    saving,
    selected,
    selectedId,
    selectClassifier,
    setConfirmation,
    setDraft,
    setPatternsText,
    setSample,
    setSampleMatched,
    test,
    toggle,
    togglingId,
  };
}
