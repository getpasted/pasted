import { useEffect, useLayoutEffect, useMemo, useState } from 'react';
import { Clipboard, Copy, Lightbulb, Plus, Radar, RotateCcw, ScanSearch, ScanText, Search, Shapes, Trash2, type LucideIcon } from 'lucide-react';
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
import { ConnectedMenuAction } from './ConnectedMenuAction';
import { translate } from '../localization/runtime';
import { useLocalization } from '../localization/LocalizationProvider';
import { localizedBuiltinDescription, localizedBuiltinName, localizedContentTypeGroupLabel } from '../localization/presentation';
import { contentTypeLabel } from '../utils/contentTypes';
import { analysisApi } from '../api/analysis';
import { SearchIndexManagerDialog } from './SearchIndexManagerDialog';

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
  missingCount?: number;
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
    content_type: 'prose',
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
    <section className="theme-divider flex min-h-[49px] items-center justify-between gap-3 border-b p-2 last:border-b-0" aria-label={translate('component.settingsAnalysisPanel.stepTitle', { step: step, title: title })}>
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
        <ActionButton aria-label={translate('component.settingsAnalysisPanel.manageTitle', { title: title })} onClick={onManage} className="h-7 min-h-7 shrink-0 px-2.5">
          <Icon className="h-3.5 w-3.5" /> {translate('component.settingsAnalysisPanel.manage')}
        </ActionButton>
      </div>
    </section>
  );
}

export function SettingsAnalysisPanel({
  contentClassificationEnabled,
  fileFormatsEnabled,
  ocrEnabled,
  transcriptionsEnabled,
  transformationsEnabled,
  typesEnabled,
  sourcesEnabled,
  searchEnabled,
  onOpenIntelligence,
}: {
  contentClassificationEnabled: boolean;
  fileFormatsEnabled: boolean;
  ocrEnabled: boolean;
  transcriptionsEnabled: boolean;
  transformationsEnabled: boolean;
  typesEnabled: boolean;
  sourcesEnabled: boolean;
  searchEnabled: boolean;
  onOpenIntelligence?: () => void;
}) {
  const { showToast } = useToast();
  const { locale } = useLocalization();
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
  const [isIndexManagerOpen, setIsIndexManagerOpen] = useState(false);
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
    const loaded = await analysisApi.listClassifiers<ContentClassifier>();
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
      get title() { return translate('common.discardChangesQuestion'); },
      get description() { return translate('component.settingsAnalysisPanel.unsavedChangesToThisClassifierWillBeLost'); },
      confirmLabel: translate('component.appDialog.discard'),
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
      showToast({ tone: 'success', message: translate('component.settingsAnalysisPanel.nameSaved', { name: saved.name }) });
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
      showToast({ tone: 'success', message: translate('component.settingsAnalysisPanel.nameDeletedResetCanRecoverShippedClassifiers', { name: selected.name }) });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
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
      showToast({ tone: 'success', get message() { return translate('component.settingsAnalysisPanel.shippedAnalysisDefaultsRestoredCustomDefinitionsWerePreserved'); } });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    } finally {
      setRestoring(false);
    }
  };

  const restoreAnalysis = () => {
    requestConfirmation({
      get title() { return translate('component.settingsAnalysisPanel.resetShippedAnalysisDefinitions'); },
      get description() { return translate('component.settingsAnalysisPanel.shippedExtractorsClassifiersContentTypesAndContentTypeGroupsReturnToTheir'); },
      details: translate('component.settingsAnalysisPanel.customDefinitionsRemainUnchanged'),
      confirmLabel: translate('common.reset'),
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
      showToast({ tone: 'success', get message() { return translate('component.settingsAnalysisPanel.builtInContentTypesAndClassifiersResetCustomDefinitionsWerePreserved'); } });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    }
  };

  const restoreClassifierDefaults = () => {
    discardDraftThen(() => requestConfirmation({
      get title() { return translate('component.settingsAnalysisPanel.resetShippedClassifierDefinitions'); },
      get description() { return translate('component.settingsAnalysisPanel.shippedContentTypesContentTypeGroupsAndClassifiersReturnToTheirDefaults'); },
      details: translate('component.settingsAnalysisPanel.customDefinitionsRemainUnchanged'),
      confirmLabel: translate('common.reset'),
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
      const reports = await Promise.all([
        contentClassificationEnabled
          ? analysisApi.rescanClassifications<ClassificationRescanReport>()
          : Promise.resolve(null),
        fileFormatsEnabled
          ? analysisApi.rescanFileFormats<ClassificationRescanReport>()
          : Promise.resolve(null),
      ]);
      const scannedCount = reports.reduce((total, report) => total + (report?.scannedCount ?? 0), 0);
      const changedCount = reports.reduce((total, report) => total + (report?.changedCount ?? 0), 0);
      const unchangedCount = reports.reduce((total, report) => total + (report?.unchangedCount ?? 0), 0);
      const missingCount = reports.reduce((total, report) => total + (report?.missingCount ?? 0), 0);
      const failedCount = reports.reduce((total, report) => total + (report?.failedCount ?? 0), 0);
      const details = [
        changedCount > 0 ? translate('component.settingsAnalysisPanel.rescanUpdated', { count: changedCount }) : null,
        unchangedCount > 0 ? translate('component.settingsAnalysisPanel.rescanUnchanged', { count: unchangedCount }) : null,
        missingCount > 0 ? translate('component.settingsAnalysisPanel.rescanMissing', { count: missingCount }) : null,
        failedCount > 0 ? translate('component.settingsAnalysisPanel.rescanFailed', { count: failedCount }) : null,
      ].filter((detail): detail is string => detail !== null);
      showToast({
        tone: failedCount > 0 ? 'info' : 'success',
        message: details.length > 0
          ? translate('component.settingsAnalysisPanel.rescanSummary', {
            count: scannedCount,
            details: new Intl.ListFormat(locale, { style: 'short', type: 'conjunction' }).format(details),
          })
          : translate('component.settingsAnalysisPanel.rescanSummaryEmpty', { count: scannedCount }),
      });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    } finally {
      setRescanning(false);
    }
  };

  const rescanHistory = () => {
    requestConfirmation({
      get title() { return translate('component.settingsAnalysisPanel.rescanExistingClips'); },
      get description() { return translate('component.settingsAnalysisPanel.enabledScannersWillRefreshDerivedClipData'); },
      details: translate('component.settingsAnalysisPanel.rescanCanChangeDerivedOrganization'),
      confirmLabel: translate('component.settingsAnalysisPanel.rescanClips'),
      onConfirm: rescanHistoryConfirmed,
    });
  };

  const test = async () => {
    try {
      const result = await analysisApi.testClassifier<ClassificationResult>(currentInput(), sample);
      if (result.outcome === 'failed') throw new Error(result.failure?.message ?? translate('component.settingsAnalysisPanel.classificationFailed'));
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
        title={translate('component.settingsAnalysisPanel.analysis')}
        description={translate('component.settingsAnalysisPanel.automaticallyScanClipsAndIndexTheirContents')}
        actions={(contentClassificationEnabled || fileFormatsEnabled) ? (
          <ActionButton onClick={rescanHistory} disabled={rescanning}>
            <ScanSearch className="h-3.5 w-3.5" /> {rescanning ? translate('component.settingsAnalysisPanel.rescanning') : translate('component.settingsAnalysisPanel.rescanClips')}
          </ActionButton>
        ) : undefined}
      />
      <section className="theme-surface overflow-hidden rounded-xl border" aria-label={translate('component.settingsAnalysisPanel.analysisSequence')}>
        <div>
          <AnalysisManagerRow
            step={1}
            icon={Clipboard}
            title={translate('component.settingsAnalysisPanel.capture')}
            description={translate('component.settingsAnalysisPanel.assignClipTypeAndCaptureContext')}
            onManage={() => setIsCaptureManagerOpen(true)}
          />
          <AnalysisManagerRow
            step={2}
            icon={ScanSearch}
            title={translate('component.settingsAnalysisPanel.inspect')}
            description={translate('component.settingsAnalysisPanel.measureStructureAndMediaFacts')}
            onManage={() => setIsInspectorManagerOpen(true)}
          />
          <AnalysisManagerRow
            step={3}
            icon={ScanText}
            title={translate('component.settingsAnalysisPanel.extract')}
            description={translate('component.settingsAnalysisPanel.createSearchableRepresentations')}
            onManage={() => setIsExtractorManagerOpen(true)}
          />
          {searchEnabled && <AnalysisManagerRow
            step={4}
            icon={Search}
            title={translate('component.settingsAnalysisPanel.index')}
            description={translate('component.settingsAnalysisPanel.keepCapturedAndExtractedTextReadyForSearch')}
            onManage={() => setIsIndexManagerOpen(true)}
          />}
          {(contentClassificationEnabled || typesEnabled) && <AnalysisManagerRow
            step={searchEnabled ? 5 : 4}
            icon={Radar}
            title={translate('component.settingsAnalysisPanel.classify')}
            description={translate('component.settingsAnalysisPanel.assignRegisteredContentTypesToAnalyzableText')}
            onManage={openClassifierManager}
          />}
          {transformationsEnabled && <AnalysisManagerRow
            step={4 + Number(searchEnabled) + Number(contentClassificationEnabled || typesEnabled)}
            icon={Lightbulb}
            title={translate('component.settingsAnalysisPanel.suggest')}
            description={translate('component.settingsAnalysisPanel.suggestActionsFromAnalysisSignals')}
            onManage={() => setIsSuggestionManagerOpen(true)}
          />}
        </div>
        <div className="theme-divider flex items-center justify-between gap-3 border-t px-3 py-2">
          <ActionButton onClick={restoreAnalysis} disabled={restoring}>
            <RotateCcw className="h-3.5 w-3.5" /> {restoring ? translate('component.settingsAnalysisPanel.resetting') : translate('component.settingsAnalysisPanel.reset')}
          </ActionButton>
          <p className="theme-text-muted text-end text-[10px]">
            {translate('component.settingsAnalysisPanel.notAllStepsRunForAllClipsSomeStepsMayBeLong')}
          </p>
        </div>
      </section>
      <>
        <AppDialog
          isOpen={isManagerOpen}
          onClose={() => setIsManagerOpen(false)}
          labelledBy="classifier-manager-title"
          isDirty={isEditorDirty}
          discardMessage={translate('component.settingsAnalysisPanel.discardClassifierChanges')}
          panelClassName="theme-panel @container flex max-h-[90vh] w-full max-w-4xl flex-col overflow-hidden border shadow-2xl"
        >
          {({ requestClose }) => <>
            <AppDialogHeader onClose={requestClose} className="shrink-0">
              <AppDialogHeading
                id="classifier-manager-title"
                title={translate('component.settingsAnalysisPanel.classifiers')}
                description={translate('component.settingsAnalysisPanel.manageHowCopiedTextIsClassifiedTheLowestPriorityNumberRunsFirst')}
                icon={<Radar />}
              />
            </AppDialogHeader>
            <AppDialogBody className="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-y-auto text-xs @xl:grid-cols-[minmax(0,3fr)_minmax(0,7fr)]">
              <section className="theme-surface flex min-h-[260px] flex-col overflow-hidden rounded-xl border @xl:min-h-0">
                <RegistryPanelHeader
                  title={translate('component.settingsAnalysisPanel.classifiers')}
                  actions={
                    <AppDialogButton onClick={beginNew} className="h-7 min-h-7 px-2.5">
                      <Plus className="h-3 w-3" /> {translate('common.new')}
                    </AppDialogButton>
                  }
                />
                <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
                  {classifiers.map((classifier) => {
                    const displayName = localizedBuiltinName('classifier', classifier.stable_ref, classifier.name, classifier.is_builtin, classifier.defaults?.name);
                    const displayDescription = localizedBuiltinDescription('classifier', classifier.stable_ref, classifier.description, classifier.is_builtin, classifier.defaults?.description);
                    return <RegistryListItem
                      key={classifier.id}
                      selected={selectedId === classifier.id}
                      onSelect={() => selectClassifier(classifier.id)}
                      icon={<ContentTypeIcon type={classifier.content_type as ClipContentType} className="h-4 w-4" />}
                      title={displayName}
                      subtitle={displayDescription}
                      trailing={
                        <SettingsSwitch
                          checked={classifier.enabled}
                          label={displayName}
                          busy={togglingId === classifier.id}
                          onClick={() => {
                            toggleClassifier(classifier);
                          }}
                        />
                      }
                    />;
                  })}
                </div>
                <RegistryPanelFooter align="end">
                  <AppDialogButton onClick={() => void duplicate()} disabled={!selected || isEditorDirty || saving} title={isEditorDirty ? translate('component.settingsAnalysisPanel.saveOrCancelChangesBeforeDuplicating') : undefined}><Copy className="h-3.5 w-3.5" /> {translate('common.duplicate')}</AppDialogButton>
                  <AppDialogButton variant="danger" onClick={remove} disabled={!selected || saving}><Trash2 className="h-3.5 w-3.5" /> {translate('component.settingsAnalysisPanel.delete')}</AppDialogButton>
                </RegistryPanelFooter>
              </section>
              <section className="theme-surface flex min-w-0 flex-col overflow-hidden rounded-xl border">
                <RegistryPanelHeader
                  title={translate('component.settingsAnalysisPanel.classifierSettings')}
                />
                <div className="min-h-0 flex-1 space-y-3 overflow-y-auto p-4">
                <div className="grid grid-cols-1 gap-3 @md:grid-cols-[minmax(0,1fr)_minmax(270px,0.8fr)]">
                  <label className={`space-y-1 ${modified.name ? 'settings-field-modified' : ''}`}>
                    <ModifiedFieldLabel modified={modified.name}>{translate('common.name')}</ModifiedFieldLabel>
                    <input value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} className="theme-input ui-field-radius w-full border px-3 py-2" />
                  </label>
                  <div className={`space-y-1 ${modified.content_type ? 'settings-field-modified' : ''}`}>
                    <ModifiedFieldLabel modified={modified.content_type}>{translate('component.settingsAnalysisPanel.contentType')}</ModifiedFieldLabel>
                    <ConnectedMenuAction
                      className="w-full"
                      groupLabel={translate('component.settingsAnalysisPanel.classifierContentType')}
                      actionLabel={translate('component.settingsAnalysisPanel.manageContentTypes')}
                      action={<><Shapes className="h-3.5 w-3.5" aria-hidden="true" /><span>{translate('component.settingsAnalysisPanel.manage')}</span></>}
                      onAction={() => setIsTypeManagerOpen(true)}
                    >
                      <MenuSelect
                        value={draft.content_type}
                        onChange={(content_type) => setDraft({ ...draft, content_type })}
                        label={translate('component.settingsAnalysisPanel.classifierContentType')}
                        leadingIcon={<ContentTypeIcon type={draft.content_type as ClipContentType} className="h-4 w-4" />}
                        options={contentTypes.map((type) => ({
                          value: type.id,
                          label: contentTypeLabel(type.id),
                          group: (() => {
                            const group = contentTypeGroups.find(({ id }) => id === type.group);
                            return group ? localizedContentTypeGroupLabel(group.id, group.label, group.isBuiltin, group.defaults?.label) : type.group;
                          })(),
                          disabled: type.isArchived,
                          icon: <ContentTypeIcon type={type.id as ClipContentType} className="h-4 w-4" />,
                        }))}
                        className="min-w-0 flex-1"
                      />
                    </ConnectedMenuAction>
                  </div>
                </div>
                <label className={`block space-y-1 ${modified.description ? 'settings-field-modified' : ''}`}>
                  <ModifiedFieldLabel modified={modified.description}>{translate('common.description')}</ModifiedFieldLabel>
                  <input value={draft.description} onChange={(event) => setDraft({ ...draft, description: event.target.value })} className="theme-input ui-field-radius w-full border px-3 py-2" />
                </label>
                <div className="grid grid-cols-1 items-end gap-3 @md:grid-cols-[110px_minmax(180px,1fr)_auto]">
                  <label className={`space-y-1 ${modified.priority ? 'settings-field-modified' : ''}`}>
                    <ModifiedFieldLabel modified={modified.priority}>{translate('common.priority')}</ModifiedFieldLabel>
                    <input type="number" value={draft.priority} onChange={(event) => setDraft({ ...draft, priority: Number(event.target.value) || 0 })} className="theme-input ui-field-radius w-full border px-3 py-2 font-mono" />
                  </label>
                  <label className={`space-y-1 ${modified.validator ? 'settings-field-modified' : ''}`}>
                    <ModifiedFieldLabel modified={modified.validator}>{translate('component.settingsAnalysisPanel.validation')}</ModifiedFieldLabel>
                    <MenuSelect
                      value={draft.validator ?? ''}
                      onChange={(validator) => setDraft({ ...draft, validator: validator || null })}
                      options={[
                        { value: '', get label() { return translate('component.settingsAnalysisPanel.regexOnly'); } },
                        { value: 'luhn', get label() { return translate('component.settingsAnalysisPanel.cardChecksum'); } },
                        { value: 'iban', get label() { return translate('component.settingsAnalysisPanel.ibanChecksum'); } },
                        { value: 'ip', get label() { return translate('component.settingsAnalysisPanel.ipParser'); } },
                        { value: 'phone', get label() { return translate('component.settingsAnalysisPanel.phoneGuardrails'); } },
                        { value: 'env_block', get label() { return translate('component.settingsAnalysisPanel.environmentBlock'); } },
                        { value: 'prose', get label() { return translate('component.settingsAnalysisPanel.proseGuardrails'); } },
                      ]}
                      label={translate('component.settingsAnalysisPanel.semanticValidation')}
                      className="w-full"
                    />
                  </label>
                  <label className={`flex min-h-9 items-center gap-2 ${modified.enabled ? 'settings-field-modified' : ''}`}>
                    <input type="checkbox" checked={draft.enabled} onChange={(event) => setDraft({ ...draft, enabled: event.target.checked })} className="theme-checkbox h-4 w-4 rounded" />
                    <ModifiedFieldLabel modified={modified.enabled}>{translate('common.enabled')}</ModifiedFieldLabel>
                  </label>
                </div>
                <label className={`block space-y-1 ${modified.patterns ? 'settings-field-modified' : ''}`}>
                  <ModifiedFieldLabel modified={modified.patterns}>{translate('component.settingsAnalysisPanel.regularExpressions')} <span className="font-normal">{translate('component.settingsAnalysisPanel.onePerLineAnyMayMatch')}</span></ModifiedFieldLabel>
                  <textarea dir="auto" value={patternsText} onChange={(event) => setPatternsText(event.target.value)} spellCheck={false} className="theme-input ui-field-radius min-h-32 w-full resize-y border px-3 py-2 font-mono text-[11px] leading-relaxed" />
                </label>
                {draft.validator && (
                  <div className="theme-status-info rounded-lg border px-3 py-2 text-[10px]">
                    {translate('component.settingsAnalysisPanel.candidatesAlsoPassTheBuiltIn')} <strong>{draft.validator}</strong>{translate('component.settingsAnalysisPanel.validatorToReduceFalsePositives')}</div>
                )}
                <div className="theme-divider grid grid-cols-[minmax(0,1fr)_auto] gap-2 border-t pt-3">
                  <input value={sample} onChange={(event) => { setSample(event.target.value); setSampleMatched(null); }} placeholder={translate('component.settingsAnalysisPanel.trySampleText')} className="theme-input ui-field-radius border px-3 py-2 font-mono" />
                  <AppDialogButton onClick={test} className="h-auto min-h-9">{translate('component.settingsAnalysisPanel.test')}</AppDialogButton>
                </div>
                {sampleMatched !== null && (
                  <div className={sampleMatched ? 'theme-status-success-text' : 'theme-status-danger-text'}>
                    {sampleMatched ? translate('component.settingsAnalysisPanel.matchesThisClassifier') : translate('component.settingsAnalysisPanel.doesNotMatchThisClassifier')}
                  </div>
                )}
                </div>
                <RegistryPanelFooter>
                  <div>
                    {selected?.is_builtin && <AppDialogButton onClick={resetSelectedDraft} disabled={!hasModifiedFields || saving}><RotateCcw className="h-3.5 w-3.5" /> {translate('common.resetToDefault')}</AppDialogButton>}
                  </div>
                  <div className="flex items-center gap-2">
                    <AppDialogButton onClick={cancelDraft} disabled={selectedId !== 'new' && !isEditorDirty}>{translate('common.cancel')}</AppDialogButton>
                    <AppDialogButton variant="primary" onClick={save} disabled={saving || (selectedId !== 'new' && !isEditorDirty)}><SaveButtonContent isSaving={saving} /></AppDialogButton>
                  </div>
                </RegistryPanelFooter>
              </section>
            </AppDialogBody>
            <AppDialogFooter align="between" className="shrink-0">
              <AppDialogButton onClick={restoreClassifierDefaults} disabled={saving}>
                <RotateCcw className="h-3.5 w-3.5" /> {translate('component.settingsAnalysisPanel.reset')}
              </AppDialogButton>
              <AppDialogButton onClick={requestClose}>{translate('common.close')}</AppDialogButton>
            </AppDialogFooter>
          </>}
        </AppDialog>
        <ContentTypeManagerDialog isOpen={isTypeManagerOpen} onClose={() => setIsTypeManagerOpen(false)} />
      </>
      <BuiltinLifecycleManagerDialog
        isOpen={isCaptureManagerOpen}
        onClose={() => setIsCaptureManagerOpen(false)}
        kind="capture"
        title={translate('component.settingsAnalysisPanel.capture')}
        description={translate('component.settingsAnalysisPanel.reviewClipTypeAndContextRecordedBeforeAnalysisBegins')}
        icon={Clipboard}
        sourcesEnabled={sourcesEnabled}
      />
      <BuiltinLifecycleManagerDialog
        isOpen={isInspectorManagerOpen}
        onClose={() => setIsInspectorManagerOpen(false)}
        kind="inspector"
        title={translate('component.settingsAnalysisPanel.inspectors')}
        description={translate('component.settingsAnalysisPanel.reviewClipInspectionBehaviorAndMediaAvailability')}
        icon={ScanSearch}
        fileFormatsEnabled={fileFormatsEnabled}
      />
      <ContentExtractorManagerDialog
        isOpen={isExtractorManagerOpen}
        onClose={() => setIsExtractorManagerOpen(false)}
        ocrEnabled={ocrEnabled}
        transcriptionsEnabled={transcriptionsEnabled}
        onOpenIntelligence={onOpenIntelligence ? () => {
          setIsExtractorManagerOpen(false);
          onOpenIntelligence();
        } : undefined}
      />
      <SearchIndexManagerDialog
        isOpen={isIndexManagerOpen}
        onClose={() => setIsIndexManagerOpen(false)}
      />
      <BuiltinLifecycleManagerDialog
        isOpen={isSuggestionManagerOpen}
        onClose={() => setIsSuggestionManagerOpen(false)}
        kind="suggestion"
        title={translate('component.settingsAnalysisPanel.suggestions')}
        description={translate('component.settingsAnalysisPanel.reviewSmartActionSuggestions')}
        icon={Lightbulb}
      />
      <ConfirmationDialog request={confirmation} onCancel={() => setConfirmation(null)} />
      {ocrEnabled && <SettingsOcrPanel />}
    </div>
  );
}
