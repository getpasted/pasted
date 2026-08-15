import { useEffect, useLayoutEffect, useMemo, useState } from 'react';
import { Clipboard, Copy, Lightbulb, Plus, Radar, RotateCcw, ScanSearch, ScanText, Shapes, Trash2, type LucideIcon } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading, SaveButtonContent } from './AppDialogLayout';
import { ContentTypeIcon } from './ContentTypeIcon';
import { ContentTypeManagerDialog } from './ContentTypeManagerDialog';
import { ContentExtractorManagerDialog } from './ContentExtractorManagerDialog';
import { ConfirmationDialog, type ConfirmationDialogRequest } from './ConfirmationDialog';
import { useContentTypes } from './ContentTypeProvider';
import { MenuSelect } from './MenuSelect';
import { ModifiedFieldLabel } from './ModifiedFieldLabel';
import { RegistryListItem } from './RegistryListItem';
import { RegistryPanelFooter } from './RegistryPanelFooter';
import { RegistryPanelHeader } from './RegistryPanelHeader';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsOcrPanel } from './SettingsOcrPanel';
import { SettingsSwitch } from './SettingsSwitch';
import { ActionButton } from './AppDialogLayout';
import { useToast } from './ToastProvider';
import type { ClipContentType } from '../types';
import { useNewItemSelection } from '../hooks/useNewItemSelection';
import { BuiltinLifecycleManagerDialog } from './BuiltinLifecycleManagerDialog';

interface ContentClassifier {
  id: number;
  stable_ref: string;
  name: string;
  content_type: string;
  description: string;
  patterns: string[];
  validator: string | null;
  enabled: boolean;
  priority: number;
  is_builtin: boolean;
  defaults: ClassifierInput | null;
}

interface ClassifierInput {
  name: string;
  content_type: string;
  description: string;
  patterns: string[];
  validator: string | null;
  enabled: boolean;
  priority: number;
}

interface ClassificationRescanReport {
  scannedCount: number;
  changedCount: number;
  unchangedCount: number;
  failedCount: number;
}

interface ClassificationResult {
  formatVersion: number;
  policy: 'capture' | 'background' | 'interactive' | 'rescan';
  through: 'inspect' | 'extract' | 'classify' | 'suggest';
  outcome: 'matched' | 'no_match' | 'failed';
  matched: boolean;
  failure: { code: string; message: string } | null;
}

function toInput(classifier?: ContentClassifier): ClassifierInput {
  return classifier ? {
    name: classifier.name,
    content_type: classifier.content_type,
    description: classifier.description,
    patterns: classifier.patterns,
    validator: classifier.validator,
    enabled: classifier.enabled,
    priority: classifier.priority,
  } : {
    name: 'Custom Classifier',
    content_type: 'text',
    description: '',
    patterns: ['^.+$'],
    validator: null,
    enabled: true,
    priority: 200,
  };
}

function AnalysisManagerRow({
  step,
  icon: Icon,
  title,
  description,
  onManage,
}: {
  step: number;
  icon: LucideIcon;
  title: string;
  description: string;
  onManage: () => void;
}) {
  return (
    <section className="theme-divider flex min-h-[49px] items-center justify-between gap-3 border-b p-2 last:border-b-0" aria-label={`${step}. ${title}`}>
      <div className="flex min-w-0 items-center gap-2 px-1">
        <span className="theme-badge grid h-6 w-6 shrink-0 place-items-center rounded-full border text-[10px] font-bold tabular-nums" aria-hidden="true">
          {step}
        </span>
        <div className="min-w-0">
          <h3 className="theme-text-main text-xs font-semibold">{title}</h3>
          <p className="theme-text-muted mt-0.5 text-[10px]">{description}</p>
        </div>
      </div>
      <div className="flex shrink-0 items-center gap-2">
        <ActionButton aria-label={`Manage ${title}…`} onClick={onManage} className="h-7 min-h-7 shrink-0 px-2.5">
          <Icon className="h-3.5 w-3.5" /> Manage…
        </ActionButton>
      </div>
    </section>
  );
}

export function SettingsAnalysisPanel({
  contentClassificationEnabled,
  ocrEnabled,
  transcriptionsEnabled,
  transformationsEnabled,
  typesEnabled,
  sourcesEnabled,
}: {
  contentClassificationEnabled: boolean;
  ocrEnabled: boolean;
  transcriptionsEnabled: boolean;
  transformationsEnabled: boolean;
  typesEnabled: boolean;
  sourcesEnabled: boolean;
}) {
  const { showToast } = useToast();
  const { definitions: contentTypes, groups: contentTypeGroups, refresh: refreshContentTypes, refreshGroups } = useContentTypes();
  const [classifiers, setClassifiers] = useState<ContentClassifier[]>([]);
  const [selectedId, setSelectedId] = useState<number | 'new' | null>(null);
  const [draft, setDraft] = useState<ClassifierInput>(toInput());
  const [patternsText, setPatternsText] = useState('^.+$');
  const [sample, setSample] = useState('');
  const [sampleMatched, setSampleMatched] = useState<boolean | null>(null);
  const [saving, setSaving] = useState(false);
  const [restoring, setRestoring] = useState(false);
  const [rescanning, setRescanning] = useState(false);
  const [togglingId, setTogglingId] = useState<number | null>(null);
  const [isTypeManagerOpen, setIsTypeManagerOpen] = useState(false);
  const [isCaptureManagerOpen, setIsCaptureManagerOpen] = useState(false);
  const [isInspectorManagerOpen, setIsInspectorManagerOpen] = useState(false);
  const [isExtractorManagerOpen, setIsExtractorManagerOpen] = useState(false);
  const [isManagerOpen, setIsManagerOpen] = useState(false);
  const [isSuggestionManagerOpen, setIsSuggestionManagerOpen] = useState(false);
  const [confirmation, setConfirmation] = useState<ConfirmationDialogRequest | null>(null);

  const selected = useMemo(
    () => typeof selectedId === 'number' ? classifiers.find((classifier) => classifier.id === selectedId) : undefined,
    [classifiers, selectedId],
  );
  const { beginNew: beginNewClassifier, cancelNew: cancelNewClassifier } = useNewItemSelection({
    selectedId,
    setSelectedId,
    itemIds: classifiers.map(({ id }) => id),
    emptySelection: null,
  });

  const load = async () => {
    const loaded = await invoke<ContentClassifier[]>('get_content_classifiers');
    setClassifiers(loaded);
    return loaded;
  };

  useEffect(() => {
    void load();
  }, []);
  useEffect(() => {
    if (!isManagerOpen) return;
    setSelectedId((current) => current ?? classifiers[0]?.id ?? 'new');
  }, [classifiers, isManagerOpen]);
  useLayoutEffect(() => {
    const next = selectedId === 'new' ? toInput() : toInput(selected);
    setDraft(next);
    setPatternsText(next.patterns.join('\n'));
    setSampleMatched(null);
  }, [selected, selectedId]);

  const currentInput = (): ClassifierInput => ({
    ...draft,
    name: draft.name.trim(),
    content_type: draft.content_type.trim(),
    description: draft.description.trim(),
    patterns: patternsText.split('\n').map((pattern) => pattern.trim()).filter(Boolean),
  });

  const comparisonInput = selectedId === 'new'
    ? toInput()
    : selected?.is_builtin ? selected.defaults : selected ? toInput(selected) : null;
  const inputForComparison = currentInput();
  const modified = {
    name: selectedId !== 'new' && comparisonInput !== null && inputForComparison.name !== comparisonInput.name,
    content_type: selectedId !== 'new' && comparisonInput !== null && inputForComparison.content_type !== comparisonInput.content_type,
    description: selectedId !== 'new' && comparisonInput !== null && inputForComparison.description !== comparisonInput.description,
    priority: selectedId !== 'new' && comparisonInput !== null && inputForComparison.priority !== comparisonInput.priority,
    validator: selectedId !== 'new' && comparisonInput !== null && inputForComparison.validator !== comparisonInput.validator,
    enabled: selectedId !== 'new' && comparisonInput !== null && inputForComparison.enabled !== comparisonInput.enabled,
    patterns: selectedId !== 'new' && comparisonInput !== null && JSON.stringify(inputForComparison.patterns) !== JSON.stringify(comparisonInput.patterns),
  };
  const hasModifiedFields = Object.values(modified).some(Boolean);
  const editorBaseline = selectedId === 'new' ? toInput() : selected ? toInput(selected) : null;
  const isEditorDirty = editorBaseline !== null
    && JSON.stringify(inputForComparison) !== JSON.stringify(editorBaseline);

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
      title: 'Discard changes?',
      description: 'Unsaved changes to this Classifier will be lost.',
      confirmLabel: 'Discard Changes',
      tone: 'danger',
      onConfirm: action,
    });
  };

  const selectClassifier = (id: number) => {
    discardDraftThen(() => setSelectedId(id));
  };

  const beginNew = () => {
    discardDraftThen(beginNewClassifier);
  };

  const cancelDraft = () => {
    if (selectedId === 'new') {
      cancelNewClassifier();
      return;
    }
    if (!selected) return;
    const restored = toInput(selected);
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

  const openClassifierManager = () => {
    setIsManagerOpen(true);
  };

  const save = async () => {
    setSaving(true);
    try {
      const input = currentInput();
      const saved = selectedId === 'new'
        ? await invoke<ContentClassifier>('create_content_classifier', { input })
        : await invoke<ContentClassifier>('update_content_classifier', { id: selectedId, input });
      await load();
      setSelectedId(saved.id);
      showToast({ tone: 'success', message: `${saved.name} saved.` });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
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
      showToast({ tone: 'success', message: `${selected.name} deleted. Reset can recover shipped Classifiers.` });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    }
  };

  const remove = () => {
    if (!selected) return;
    requestConfirmation({
      title: 'Delete Classifier?',
      description: selected.name,
      details: selected.is_builtin
        ? 'This removes the Classifier from the library. Reset can recover it.'
        : 'This permanently removes the custom Classifier from the library.',
      confirmLabel: 'Delete Classifier',
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
      showToast({ tone: 'success', message: `${created.name} created.` });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    }
  };

  const restoreAnalysisConfirmed = async () => {
    setRestoring(true);
    try {
      const [restored] = await Promise.all([
        invoke<ContentClassifier[]>('restore_default_content_classifiers'),
        invoke('restore_default_content_extractors'),
        invoke('restore_default_content_types'),
        invoke('restore_default_content_type_groups'),
      ]);
      await Promise.all([refreshContentTypes(), refreshGroups()]);
      setClassifiers(restored);
      setSelectedId(restored[0]?.id ?? 'new');
      showToast({ tone: 'success', message: 'Shipped Analysis defaults restored. Custom definitions were preserved.' });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    } finally {
      setRestoring(false);
    }
  };

  const restoreAnalysis = () => {
    requestConfirmation({
      title: 'Reset shipped Analysis definitions?',
      description: 'Shipped Extractors, Classifiers, Content Types, and Content Type Groups return to their defaults.',
      details: 'Custom definitions remain unchanged.',
      confirmLabel: 'Reset',
      onConfirm: restoreAnalysisConfirmed,
    });
  };

  const restoreClassifierDefaultsConfirmed = async () => {
    try {
      const [restored] = await Promise.all([
        invoke<ContentClassifier[]>('restore_default_content_classifiers'),
        invoke('restore_default_content_types'),
        invoke('restore_default_content_type_groups'),
      ]);
      await Promise.all([refreshContentTypes(), refreshGroups()]);
      setClassifiers(restored);
      setSelectedId(restored[0]?.id ?? 'new');
      showToast({ tone: 'success', message: 'Built-in Content Types and Classifiers reset. Custom definitions were preserved.' });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    }
  };

  const restoreClassifierDefaults = () => {
    discardDraftThen(() => requestConfirmation({
      title: 'Reset shipped Classifier definitions?',
      description: 'Shipped Content Types, Content Type Groups, and Classifiers return to their defaults.',
      details: 'Custom definitions remain unchanged.',
      confirmLabel: 'Reset',
      onConfirm: restoreClassifierDefaultsConfirmed,
    }));
  };

  const toggleClassifierConfirmed = async (classifier: ContentClassifier) => {
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
      showToast({ tone: 'error', message: String(error) });
    } finally {
      setTogglingId(null);
    }
  };

  const toggleClassifier = (classifier: ContentClassifier) => {
    if (selectedId === classifier.id) {
      discardDraftThen(() => toggleClassifierConfirmed(classifier));
      return;
    }
    void toggleClassifierConfirmed(classifier);
  };

  const rescanHistoryConfirmed = async () => {
    setRescanning(true);
    try {
      const report = await invoke<ClassificationRescanReport>('rescan_content_classification_history', { confirmed: true });
      showToast({
        tone: report.failedCount > 0 ? 'info' : 'success',
        message: report.failedCount > 0
          ? `Rescanned ${report.scannedCount} text clips; ${report.changedCount} reclassified and ${report.failedCount} failed.`
          : `Rescanned ${report.scannedCount} text clips; ${report.changedCount} reclassified.`,
      });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    } finally {
      setRescanning(false);
    }
  };

  const rescanHistory = () => {
    requestConfirmation({
      title: 'Rescan existing text clips?',
      description: 'Current enabled Classifiers will reclassify the text clip history.',
      details: 'Content Types, Smart Bin membership, and sensitive-content masking can change. Images and files remain unchanged.',
      confirmLabel: 'Rescan Clips',
      onConfirm: rescanHistoryConfirmed,
    });
  };

  const test = async () => {
    try {
      const result = await invoke<ClassificationResult>('test_content_classifier', { input: currentInput(), sample });
      if (result.outcome === 'failed') throw new Error(result.failure?.message ?? 'Classification failed.');
      setSampleMatched(result.matched);
    } catch (error) {
      setSampleMatched(false);
      showToast({ tone: 'error', message: String(error) });
    }
  };

  return (
    <div className="space-y-5 text-xs">
      <SettingsPanelHeader
        icon={ScanSearch}
        title="Analysis"
        description="Automatically scan clips and index their contents."
        actions={contentClassificationEnabled ? (
          <ActionButton onClick={rescanHistory} disabled={rescanning}>
            <ScanSearch className="h-3.5 w-3.5" /> {rescanning ? 'Rescanning…' : 'Rescan Clips…'}
          </ActionButton>
        ) : undefined}
      />
      <section className="theme-surface overflow-hidden rounded-xl border" aria-label="Analysis sequence">
        <div>
          <AnalysisManagerRow
            step={1}
            icon={Clipboard}
            title="Capture"
            description="Assign Clip Type and capture context."
            onManage={() => setIsCaptureManagerOpen(true)}
          />
          <AnalysisManagerRow
            step={2}
            icon={ScanSearch}
            title="Inspect"
            description="Measure structure and media facts."
            onManage={() => setIsInspectorManagerOpen(true)}
          />
          {(ocrEnabled || transcriptionsEnabled) && <AnalysisManagerRow
            step={3}
            icon={ScanText}
            title="Extract"
            description="Create searchable representations."
            onManage={() => setIsExtractorManagerOpen(true)}
          />}
          {(contentClassificationEnabled || typesEnabled) && <AnalysisManagerRow
            step={4}
            icon={Radar}
            title="Classify"
            description="Assign registered Content Types to analyzable text."
            onManage={openClassifierManager}
          />}
          {transformationsEnabled && <AnalysisManagerRow
            step={5}
            icon={Lightbulb}
            title="Suggest"
            description="Suggest actions from analysis signals."
            onManage={() => setIsSuggestionManagerOpen(true)}
          />}
        </div>
        <div className="theme-divider flex items-center justify-between gap-3 border-t px-3 py-2">
          <ActionButton onClick={restoreAnalysis} disabled={restoring}>
            <RotateCcw className="h-3.5 w-3.5" /> {restoring ? 'Resetting…' : 'Reset…'}
          </ActionButton>
          <p className="theme-text-muted text-right text-[10px]">
            Not all steps run for all clips. Some steps may be long-running.
          </p>
        </div>
      </section>
      <>
        <AppDialog
          isOpen={isManagerOpen}
          onClose={() => setIsManagerOpen(false)}
          labelledBy="classifier-manager-title"
          isDirty={isEditorDirty}
          discardMessage="Discard changes to this classifier?"
          panelClassName="theme-panel @container flex max-h-[90vh] w-full max-w-4xl flex-col overflow-hidden border shadow-2xl"
        >
          {({ requestClose }) => <>
            <AppDialogHeader onClose={requestClose} className="shrink-0">
              <AppDialogHeading
                id="classifier-manager-title"
                title="Classifiers"
                description="Manage how copied text is classified. The lowest priority number runs first."
                icon={<Radar />}
              />
            </AppDialogHeader>
            <AppDialogBody className="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-y-auto text-xs @xl:grid-cols-[minmax(0,3fr)_minmax(0,7fr)]">
              <section className="theme-surface flex min-h-[260px] flex-col overflow-hidden rounded-xl border @xl:min-h-0">
                <RegistryPanelHeader
                  title="Classifiers"
                  actions={
                    <AppDialogButton onClick={beginNew} className="h-7 min-h-7 px-2.5">
                      <Plus className="h-3 w-3" /> New
                    </AppDialogButton>
                  }
                />
                <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
                  {classifiers.map((classifier) => (
                    <RegistryListItem
                      key={classifier.id}
                      selected={selectedId === classifier.id}
                      onSelect={() => selectClassifier(classifier.id)}
                      icon={<ContentTypeIcon type={classifier.content_type as ClipContentType} className="h-4 w-4" />}
                      title={classifier.name}
                      subtitle={classifier.description}
                      trailing={
                        <SettingsSwitch
                          checked={classifier.enabled}
                          label={classifier.name}
                          busy={togglingId === classifier.id}
                          onClick={() => {
                            toggleClassifier(classifier);
                          }}
                        />
                      }
                    />
                  ))}
                </div>
                <RegistryPanelFooter align="end">
                  <AppDialogButton onClick={() => void duplicate()} disabled={!selected || isEditorDirty || saving} title={isEditorDirty ? 'Save or cancel changes before duplicating.' : undefined}><Copy className="h-3.5 w-3.5" /> Duplicate</AppDialogButton>
                  <AppDialogButton variant="danger" onClick={remove} disabled={!selected || saving}><Trash2 className="h-3.5 w-3.5" /> Delete…</AppDialogButton>
                </RegistryPanelFooter>
              </section>
              <section className="theme-surface flex min-w-0 flex-col overflow-hidden rounded-xl border">
                <RegistryPanelHeader
                  title="Classifier Settings"
                  actions={
                    <AppDialogButton onClick={() => setIsTypeManagerOpen(true)} className="h-7 min-h-7 shrink-0 px-2.5">
                      <Shapes className="h-3.5 w-3.5" /> Manage Content Types…
                    </AppDialogButton>
                  }
                />
                <div className="min-h-0 flex-1 space-y-3 overflow-y-auto p-4">
                <div className="grid grid-cols-1 gap-3 @md:grid-cols-[minmax(0,1fr)_minmax(150px,0.45fr)]">
                  <label className={`space-y-1 ${modified.name ? 'settings-field-modified' : ''}`}>
                    <ModifiedFieldLabel modified={modified.name}>Name</ModifiedFieldLabel>
                    <input value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} className="theme-input ui-field-radius w-full border px-3 py-2" />
                  </label>
                  <div className={`space-y-1 ${modified.content_type ? 'settings-field-modified' : ''}`}>
                    <ModifiedFieldLabel modified={modified.content_type}>Content type</ModifiedFieldLabel>
                    <MenuSelect
                      value={draft.content_type}
                      onChange={(content_type) => setDraft({ ...draft, content_type })}
                      label="Classifier content type"
                      leadingIcon={<ContentTypeIcon type={draft.content_type as ClipContentType} className="h-4 w-4" />}
                      options={contentTypes.map((type) => ({
                        value: type.id,
                        label: type.label,
                        group: contentTypeGroups.find(({ id }) => id === type.group)?.label ?? type.group,
                        disabled: type.isArchived,
                        icon: <ContentTypeIcon type={type.id as ClipContentType} className="h-4 w-4" />,
                      }))}
                      className="w-full"
                    />
                  </div>
                </div>
                <label className={`block space-y-1 ${modified.description ? 'settings-field-modified' : ''}`}>
                  <ModifiedFieldLabel modified={modified.description}>Description</ModifiedFieldLabel>
                  <input value={draft.description} onChange={(event) => setDraft({ ...draft, description: event.target.value })} className="theme-input ui-field-radius w-full border px-3 py-2" />
                </label>
                <div className="grid grid-cols-1 items-end gap-3 @md:grid-cols-[110px_minmax(180px,1fr)_auto]">
                  <label className={`space-y-1 ${modified.priority ? 'settings-field-modified' : ''}`}>
                    <ModifiedFieldLabel modified={modified.priority}>Priority</ModifiedFieldLabel>
                    <input type="number" value={draft.priority} onChange={(event) => setDraft({ ...draft, priority: Number(event.target.value) || 0 })} className="theme-input ui-field-radius w-full border px-3 py-2 font-mono" />
                  </label>
                  <label className={`space-y-1 ${modified.validator ? 'settings-field-modified' : ''}`}>
                    <ModifiedFieldLabel modified={modified.validator}>Validation</ModifiedFieldLabel>
                    <MenuSelect
                      value={draft.validator ?? ''}
                      onChange={(validator) => setDraft({ ...draft, validator: validator || null })}
                      options={[
                        { value: '', label: 'Regex only' },
                        { value: 'luhn', label: 'Card checksum' },
                        { value: 'iban', label: 'IBAN checksum' },
                        { value: 'ip', label: 'IP parser' },
                        { value: 'phone', label: 'Phone guardrails' },
                        { value: 'env_block', label: 'Environment block' },
                        { value: 'prose', label: 'Prose guardrails' },
                      ]}
                      label="Semantic validation"
                      className="w-full"
                    />
                  </label>
                  <label className={`flex min-h-9 items-center gap-2 ${modified.enabled ? 'settings-field-modified' : ''}`}>
                    <input type="checkbox" checked={draft.enabled} onChange={(event) => setDraft({ ...draft, enabled: event.target.checked })} className="theme-checkbox h-4 w-4 rounded" />
                    <ModifiedFieldLabel modified={modified.enabled}>Enabled for new clips</ModifiedFieldLabel>
                  </label>
                </div>
                <label className={`block space-y-1 ${modified.patterns ? 'settings-field-modified' : ''}`}>
                  <ModifiedFieldLabel modified={modified.patterns}>Regular expressions <span className="font-normal">(one per line; any may match)</span></ModifiedFieldLabel>
                  <textarea value={patternsText} onChange={(event) => setPatternsText(event.target.value)} spellCheck={false} className="theme-input ui-field-radius min-h-32 w-full resize-y border px-3 py-2 font-mono text-[11px] leading-relaxed" />
                </label>
                {draft.validator && (
                  <div className="theme-status-info rounded-lg border px-3 py-2 text-[10px]">
                    Candidates also pass the built-in <strong>{draft.validator}</strong> validator to reduce false positives.
                  </div>
                )}
                <div className="theme-divider grid grid-cols-[minmax(0,1fr)_auto] gap-2 border-t pt-3">
                  <input value={sample} onChange={(event) => { setSample(event.target.value); setSampleMatched(null); }} placeholder="Try sample text…" className="theme-input ui-field-radius border px-3 py-2 font-mono" />
                  <AppDialogButton onClick={test} className="h-auto min-h-9">Test</AppDialogButton>
                </div>
                {sampleMatched !== null && (
                  <div className={sampleMatched ? 'theme-status-success-text' : 'theme-status-danger-text'}>
                    {sampleMatched ? 'Matches this classifier' : 'Does not match this classifier'}
                  </div>
                )}
                </div>
                <RegistryPanelFooter>
                  <div>
                    {selected?.is_builtin && <AppDialogButton onClick={resetSelectedDraft} disabled={!hasModifiedFields || saving}><RotateCcw className="h-3.5 w-3.5" /> Reset to Default</AppDialogButton>}
                  </div>
                  <div className="flex items-center gap-2">
                    <AppDialogButton onClick={cancelDraft} disabled={selectedId !== 'new' && !isEditorDirty}>Cancel</AppDialogButton>
                    <AppDialogButton variant="primary" onClick={save} disabled={saving || (selectedId !== 'new' && !isEditorDirty)}><SaveButtonContent isSaving={saving} /></AppDialogButton>
                  </div>
                </RegistryPanelFooter>
              </section>
            </AppDialogBody>
            <AppDialogFooter align="between" className="shrink-0">
              <AppDialogButton onClick={restoreClassifierDefaults} disabled={saving}>
                <RotateCcw className="h-3.5 w-3.5" /> Reset…
              </AppDialogButton>
              <AppDialogButton onClick={requestClose}>Close</AppDialogButton>
            </AppDialogFooter>
          </>}
        </AppDialog>
        <ContentTypeManagerDialog isOpen={isTypeManagerOpen} onClose={() => setIsTypeManagerOpen(false)} />
      </>
      <BuiltinLifecycleManagerDialog
        isOpen={isCaptureManagerOpen}
        onClose={() => setIsCaptureManagerOpen(false)}
        kind="capture"
        title="Capture"
        description="Review Clip Type and context recorded before Analysis begins."
        icon={Clipboard}
        sourcesEnabled={sourcesEnabled}
      />
      <BuiltinLifecycleManagerDialog
        isOpen={isInspectorManagerOpen}
        onClose={() => setIsInspectorManagerOpen(false)}
        kind="inspector"
        title="Inspectors"
        description="Review clip inspection behavior and media availability."
        icon={ScanSearch}
      />
      <ContentExtractorManagerDialog
        isOpen={isExtractorManagerOpen}
        onClose={() => setIsExtractorManagerOpen(false)}
        ocrEnabled={ocrEnabled}
        transcriptionsEnabled={transcriptionsEnabled}
      />
      <BuiltinLifecycleManagerDialog
        isOpen={isSuggestionManagerOpen}
        onClose={() => setIsSuggestionManagerOpen(false)}
        kind="suggestion"
        title="Suggestions"
        description="Review Smart Action suggestions."
        icon={Lightbulb}
      />
      <ConfirmationDialog request={confirmation} onCancel={() => setConfirmation(null)} />
      {ocrEnabled && <SettingsOcrPanel />}
    </div>
  );
}
