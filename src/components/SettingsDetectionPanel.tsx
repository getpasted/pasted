import { useEffect, useLayoutEffect, useMemo, useState } from 'react';
import { Copy, Pencil, Plus, Radar, RotateCcw, ScanSearch, Shapes, Trash2 } from 'lucide-react';
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

interface ContentDetector {
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
  defaults: DetectorInput | null;
}

interface DetectorInput {
  name: string;
  content_type: string;
  description: string;
  patterns: string[];
  validator: string | null;
  enabled: boolean;
  priority: number;
}

interface DetectionRescanReport {
  scannedCount: number;
  changedCount: number;
  unchangedCount: number;
  failedCount: number;
}

interface DetectionResult {
  formatVersion: number;
  policy: 'capture' | 'background' | 'interactive' | 'rescan';
  through: 'inspect' | 'extract' | 'classify' | 'enrich';
  outcome: 'matched' | 'no_match' | 'failed';
  matched: boolean;
  failure: { code: string; message: string } | null;
}

function toInput(detector?: ContentDetector): DetectorInput {
  return detector ? {
    name: detector.name,
    content_type: detector.content_type,
    description: detector.description,
    patterns: detector.patterns,
    validator: detector.validator,
    enabled: detector.enabled,
    priority: detector.priority,
  } : {
    name: 'Custom Detector',
    content_type: 'text',
    description: '',
    patterns: ['^.+$'],
    validator: null,
    enabled: true,
    priority: 200,
  };
}

export function SettingsDetectionPanel({
  contentDetectionEnabled,
  ocrEnabled,
}: {
  contentDetectionEnabled: boolean;
  ocrEnabled: boolean;
}) {
  const { showToast } = useToast();
  const { definitions: contentTypes, groups: contentTypeGroups, refresh: refreshContentTypes, refreshGroups } = useContentTypes();
  const [detectors, setDetectors] = useState<ContentDetector[]>([]);
  const [selectedId, setSelectedId] = useState<number | 'new' | null>(null);
  const [draft, setDraft] = useState<DetectorInput>(toInput());
  const [patternsText, setPatternsText] = useState('^.+$');
  const [sample, setSample] = useState('');
  const [sampleMatched, setSampleMatched] = useState<boolean | null>(null);
  const [saving, setSaving] = useState(false);
  const [rescanning, setRescanning] = useState(false);
  const [togglingId, setTogglingId] = useState<number | null>(null);
  const [isTypeManagerOpen, setIsTypeManagerOpen] = useState(false);
  const [isExtractorManagerOpen, setIsExtractorManagerOpen] = useState(false);
  const [isManagerOpen, setIsManagerOpen] = useState(false);
  const [confirmation, setConfirmation] = useState<ConfirmationDialogRequest | null>(null);

  const selected = useMemo(
    () => typeof selectedId === 'number' ? detectors.find((detector) => detector.id === selectedId) : undefined,
    [detectors, selectedId],
  );
  const { beginNew: beginNewDetector, cancelNew: cancelNewDetector } = useNewItemSelection({
    selectedId,
    setSelectedId,
    itemIds: detectors.map(({ id }) => id),
    emptySelection: null,
  });

  const load = async () => {
    const loaded = await invoke<ContentDetector[]>('get_content_detectors');
    setDetectors(loaded);
  };

  useEffect(() => {
    if (contentDetectionEnabled) void load();
  }, [contentDetectionEnabled]);
  useEffect(() => {
    if (!isManagerOpen) return;
    setSelectedId((current) => current ?? detectors[0]?.id ?? 'new');
  }, [detectors, isManagerOpen]);
  useLayoutEffect(() => {
    const next = selectedId === 'new' ? toInput() : toInput(selected);
    setDraft(next);
    setPatternsText(next.patterns.join('\n'));
    setSampleMatched(null);
  }, [selected, selectedId]);

  const currentInput = (): DetectorInput => ({
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
      description: 'Unsaved changes to this Detector will be lost.',
      confirmLabel: 'Discard Changes',
      tone: 'danger',
      onConfirm: action,
    });
  };

  const selectDetector = (id: number) => {
    discardDraftThen(() => setSelectedId(id));
  };

  const beginNew = () => {
    discardDraftThen(beginNewDetector);
  };

  const cancelDraft = () => {
    if (selectedId === 'new') {
      cancelNewDetector();
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

  const openDetectorManager = () => {
    setIsManagerOpen(true);
  };

  const save = async () => {
    setSaving(true);
    try {
      const input = currentInput();
      const saved = selectedId === 'new'
        ? await invoke<ContentDetector>('create_content_detector', { input })
        : await invoke<ContentDetector>('update_content_detector', { id: selectedId, input });
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
      await invoke('delete_content_detector', { id: selectedId });
      const remaining = detectors.filter((detector) => detector.id !== selectedId);
      setDetectors(remaining);
      setSelectedId(remaining[0]?.id ?? 'new');
      showToast({ tone: 'success', message: `${selected.name} deleted. Restore Shipped Defaults can recover shipped detectors.` });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    }
  };

  const remove = () => {
    if (!selected) return;
    requestConfirmation({
      title: 'Delete Detector?',
      description: selected.name,
      details: selected.is_builtin
        ? 'This removes the Detector from the library. Restore Shipped Defaults can recover it.'
        : 'This permanently removes the custom Detector from the library.',
      confirmLabel: 'Delete Detector',
      tone: 'danger',
      onConfirm: removeConfirmed,
    });
  };

  const duplicate = async () => {
    if (!selected || isEditorDirty) return;
    try {
      const created = await invoke<ContentDetector>('duplicate_content_detector', {
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

  const restoreConfirmed = async () => {
    try {
      const [restored] = await Promise.all([
        invoke<ContentDetector[]>('restore_default_content_detectors'),
        invoke('restore_default_content_types'),
        invoke('restore_default_content_type_groups'),
      ]);
      await Promise.all([refreshContentTypes(), refreshGroups()]);
      setDetectors(restored);
      setSelectedId(restored[0]?.id ?? 'new');
      showToast({ tone: 'success', message: 'Built-in Types and detectors restored. Custom entries were preserved.' });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    }
  };

  const restore = () => {
    discardDraftThen(() => requestConfirmation({
      title: 'Restore shipped Analysis definitions?',
      description: 'Shipped Types, Type Groups, and Detectors return to their defaults.',
      details: 'Custom definitions remain unchanged.',
      confirmLabel: 'Restore Defaults',
      onConfirm: restoreConfirmed,
    }));
  };

  const toggleDetectorConfirmed = async (detector: ContentDetector) => {
    setTogglingId(detector.id);
    try {
      const enabled = !detector.enabled;
      await invoke('set_library_item_enabled', {
        kind: 'detector',
        stableRef: detector.stable_ref,
        enabled,
      });
      setDetectors((current) => current.map((item) => (
        item.id === detector.id ? { ...item, enabled } : item
      )));
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    } finally {
      setTogglingId(null);
    }
  };

  const toggleDetector = (detector: ContentDetector) => {
    if (selectedId === detector.id) {
      discardDraftThen(() => toggleDetectorConfirmed(detector));
      return;
    }
    void toggleDetectorConfirmed(detector);
  };

  const rescanHistoryConfirmed = async () => {
    setRescanning(true);
    try {
      const report = await invoke<DetectionRescanReport>('rescan_content_detection_history', { confirmed: true });
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
      description: 'Current enabled Detectors will reclassify the text clip history.',
      details: 'Types, Smart Bin membership, and sensitive-content masking can change. Images and files remain unchanged.',
      confirmLabel: 'Rescan Clips',
      onConfirm: rescanHistoryConfirmed,
    });
  };

  const test = async () => {
    try {
      const result = await invoke<DetectionResult>('test_content_detector', { input: currentInput(), sample });
      if (result.outcome === 'failed') throw new Error(result.failure?.message ?? 'Detection failed.');
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
        description="Understand clip content and make it more useful."
        actions={contentDetectionEnabled ? (
          <ActionButton onClick={rescanHistory} disabled={rescanning}>
            <ScanSearch className="h-3.5 w-3.5" /> {rescanning ? 'Rescanning…' : 'Rescan Clips'}
          </ActionButton>
        ) : undefined}
      />
      {ocrEnabled && <section className="theme-surface overflow-hidden rounded-xl border" aria-label="Content extractors">
        <RegistryPanelHeader
          title="Extractors"
          description="Create searchable representations from clip content."
          actions={
            <ActionButton onClick={() => setIsExtractorManagerOpen(true)} className="h-7 min-h-7 shrink-0 px-2.5">
              <Pencil className="h-3.5 w-3.5" /> Manage Extractors
            </ActionButton>
          }
        />
      </section>}
      {contentDetectionEnabled && <>
        <section className="theme-surface overflow-hidden rounded-xl border" aria-label="Content detectors">
          <RegistryPanelHeader
            title="Detectors"
            description="Manage clip-types for copied text."
            actions={
              <ActionButton onClick={openDetectorManager} className="h-7 min-h-7 shrink-0 px-2.5">
                <Pencil className="h-3.5 w-3.5" /> Manage Detectors
              </ActionButton>
            }
          />
        </section>
        <AppDialog
          isOpen={isManagerOpen}
          onClose={() => setIsManagerOpen(false)}
          labelledBy="detector-manager-title"
          isDirty={isEditorDirty}
          discardMessage="Discard changes to this detector?"
          panelClassName="theme-panel @container flex max-h-[90vh] w-full max-w-4xl flex-col overflow-hidden border shadow-2xl"
        >
          {({ requestClose }) => <>
            <AppDialogHeader onClose={requestClose} className="shrink-0">
              <AppDialogHeading
                id="detector-manager-title"
                title="Detectors"
                description="Manage clip-types for copied text. The lowest priority number runs first."
                icon={<Radar />}
              />
            </AppDialogHeader>
            <AppDialogBody className="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-y-auto text-xs @xl:grid-cols-[minmax(0,3fr)_minmax(0,7fr)]">
              <section className="theme-surface flex min-h-[260px] flex-col overflow-hidden rounded-xl border @xl:min-h-0">
                <RegistryPanelHeader
                  title="Detectors"
                  actions={
                    <AppDialogButton onClick={beginNew} className="h-7 min-h-7 px-2.5">
                      <Plus className="h-3 w-3" /> New
                    </AppDialogButton>
                  }
                />
                <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
                  {detectors.map((detector) => (
                    <RegistryListItem
                      key={detector.id}
                      selected={selectedId === detector.id}
                      onSelect={() => selectDetector(detector.id)}
                      icon={<ContentTypeIcon type={detector.content_type as ClipContentType} className="h-4 w-4" />}
                      title={detector.name}
                      subtitle={detector.description}
                      trailing={
                        <SettingsSwitch
                          checked={detector.enabled}
                          label={detector.name}
                          busy={togglingId === detector.id}
                          onClick={() => {
                            toggleDetector(detector);
                          }}
                        />
                      }
                    />
                  ))}
                </div>
                <RegistryPanelFooter align="end">
                  <AppDialogButton onClick={() => void duplicate()} disabled={!selected || isEditorDirty || saving} title={isEditorDirty ? 'Save or cancel changes before duplicating.' : undefined}><Copy className="h-3.5 w-3.5" /> Duplicate</AppDialogButton>
                  <AppDialogButton variant="danger" onClick={remove} disabled={!selected || saving}><Trash2 className="h-3.5 w-3.5" /> Delete</AppDialogButton>
                </RegistryPanelFooter>
              </section>
              <section className="theme-surface flex min-w-0 flex-col overflow-hidden rounded-xl border">
                <RegistryPanelHeader
                  title="Detector Settings"
                  actions={
                    <AppDialogButton onClick={() => setIsTypeManagerOpen(true)} className="h-7 min-h-7 shrink-0 px-2.5">
                      <Shapes className="h-3.5 w-3.5" /> Manage Types
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
                      label="Detector content type"
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
                    {sampleMatched ? 'Matches this detector' : 'Does not match this detector'}
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
              <AppDialogButton onClick={restore} disabled={saving}>
                <RotateCcw className="h-3.5 w-3.5" /> Restore Shipped Defaults…
              </AppDialogButton>
              <AppDialogButton onClick={requestClose}>Close</AppDialogButton>
            </AppDialogFooter>
          </>}
        </AppDialog>
        <ContentTypeManagerDialog isOpen={isTypeManagerOpen} onClose={() => setIsTypeManagerOpen(false)} />
      </>}
      <ContentExtractorManagerDialog isOpen={isExtractorManagerOpen} onClose={() => setIsExtractorManagerOpen(false)} />
      <ConfirmationDialog request={confirmation} onCancel={() => setConfirmation(null)} />
      {ocrEnabled && <SettingsOcrPanel />}
    </div>
  );
}
