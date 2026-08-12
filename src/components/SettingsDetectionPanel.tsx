import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { Copy, Plus, Radar, RotateCcw, Save, ScanSearch, Shapes, Trash2, X } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { ContentTypeIcon } from './ContentTypeIcon';
import { ContentTypeManagerDialog } from './ContentTypeManagerDialog';
import { useContentTypes } from './ContentTypeProvider';
import { MenuSelect } from './MenuSelect';
import { ModifiedFieldLabel } from './ModifiedFieldLabel';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { useToast } from './ToastProvider';
import type { ClipContentType } from '../types';

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

export function SettingsDetectionPanel() {
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
  const previousSelectedIdRef = useRef<number | null>(null);

  const selected = useMemo(
    () => typeof selectedId === 'number' ? detectors.find((detector) => detector.id === selectedId) : undefined,
    [detectors, selectedId],
  );

  const load = async () => {
    const loaded = await invoke<ContentDetector[]>('get_content_detectors');
    setDetectors(loaded);
    setSelectedId((current) => current ?? loaded[0]?.id ?? 'new');
  };

  useEffect(() => { void load(); }, []);
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
    ? null
    : selected?.is_builtin ? selected.defaults : selected ? toInput(selected) : null;
  const inputForComparison = currentInput();
  const modified = {
    name: comparisonInput !== null && inputForComparison.name !== comparisonInput.name,
    content_type: comparisonInput !== null && inputForComparison.content_type !== comparisonInput.content_type,
    description: comparisonInput !== null && inputForComparison.description !== comparisonInput.description,
    priority: comparisonInput !== null && inputForComparison.priority !== comparisonInput.priority,
    validator: comparisonInput !== null && inputForComparison.validator !== comparisonInput.validator,
    enabled: comparisonInput !== null && inputForComparison.enabled !== comparisonInput.enabled,
    patterns: comparisonInput !== null && JSON.stringify(inputForComparison.patterns) !== JSON.stringify(comparisonInput.patterns),
  };
  const hasModifiedFields = Object.values(modified).some(Boolean);

  const resetSelectedDraft = () => {
    if (!selected?.is_builtin || !selected.defaults) return;
    setDraft({ ...selected.defaults, patterns: [...selected.defaults.patterns] });
    setPatternsText(selected.defaults.patterns.join('\n'));
    setSampleMatched(null);
  };

  const beginNewDetector = () => {
    if (typeof selectedId === 'number') previousSelectedIdRef.current = selectedId;
    setSelectedId('new');
  };

  const cancelNewDetector = () => {
    const previousId = previousSelectedIdRef.current;
    const fallbackId = detectors[0]?.id ?? null;
    setSelectedId(previousId !== null && detectors.some(({ id }) => id === previousId) ? previousId : fallbackId);
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

  const remove = async () => {
    if (typeof selectedId !== 'number' || !selected) return;
    if (!window.confirm(`Delete the detector “${selected.name}”? Shipped detectors can be recovered with Restore Defaults.`)) return;
    try {
      await invoke('delete_content_detector', { id: selectedId });
      const remaining = detectors.filter((detector) => detector.id !== selectedId);
      setDetectors(remaining);
      setSelectedId(remaining[0]?.id ?? 'new');
      showToast({ tone: 'success', message: `${selected.name} deleted. Restore Defaults can recover shipped detectors.` });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    }
  };

  const duplicate = async () => {
    const next = currentInput();
    next.name = `${next.name || 'Detector'} Copy`;
    next.priority += 1;
    try {
      const created = await invoke<ContentDetector>('create_content_detector', { input: next });
      await load();
      setSelectedId(created.id);
      showToast({ tone: 'success', message: `${created.name} created.` });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    }
  };

  const restore = async () => {
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

  const toggleDetector = async (detector: ContentDetector) => {
    setTogglingId(detector.id);
    try {
      const input = selectedId === detector.id
        ? { ...currentInput(), enabled: !draft.enabled }
        : { ...toInput(detector), enabled: !detector.enabled };
      const saved = await invoke<ContentDetector>('update_content_detector', {
        id: detector.id,
        input,
      });
      setDetectors((current) => current.map((item) => item.id === saved.id ? saved : item));
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    } finally {
      setTogglingId(null);
    }
  };

  const rescanHistory = async () => {
    if (!window.confirm('Rescan all existing text clips with the current enabled detectors? This can change Types, Smart Bin membership, and sensitive-content masking. Images and files will not be changed.')) return;
    setRescanning(true);
    try {
      const report = await invoke<DetectionRescanReport>('rescan_content_detection_history', { confirmed: true });
      showToast({
        tone: 'success',
        message: `Rescanned ${report.scannedCount} text clips; ${report.changedCount} reclassified.`,
      });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    } finally {
      setRescanning(false);
    }
  };

  const test = async () => {
    try {
      setSampleMatched(await invoke<boolean>('test_content_detector', { input: currentInput(), sample }));
    } catch (error) {
      setSampleMatched(false);
      showToast({ tone: 'error', message: String(error) });
    }
  };

  return (
    <div className="space-y-5 text-xs">
      <SettingsPanelHeader
        icon={Radar}
        title="Detection"
        description="Classify new clips with ordered, editable detectors."
        actions={(
          <div className="flex flex-wrap items-center justify-end gap-2">
            <button type="button" onClick={() => setIsTypeManagerOpen(true)} className="app-dialog-button is-secondary">
              <Shapes className="h-3.5 w-3.5" /> Manage Types
            </button>
            <button type="button" onClick={() => void rescanHistory()} disabled={rescanning} className="app-dialog-button is-secondary">
              <ScanSearch className="h-3.5 w-3.5" /> {rescanning ? 'Rescanning…' : 'Rescan History'}
            </button>
            <button type="button" onClick={() => void restore()} className="app-dialog-button is-secondary">
              <RotateCcw className="h-3.5 w-3.5" /> Restore Defaults
            </button>
          </div>
        )}
      />
      <div className="@container">
      <div className="grid min-h-[520px] grid-cols-1 gap-4 @4xl:grid-cols-[minmax(220px,0.72fr)_minmax(0,1.4fr)]">
        <section className="theme-surface overflow-hidden rounded-xl border" aria-label="Content detectors">
          <div className="theme-divider flex items-center justify-between gap-3 border-b p-2">
            <span className="min-w-0 px-1">
              <span className="theme-text-muted block text-[10px] font-bold uppercase tracking-wider">Detectors</span>
              <span className="theme-text-subtle mt-0.5 block text-[9px]">Lowest priority number runs first</span>
            </span>
            <button type="button" onClick={beginNewDetector} className="app-dialog-button is-secondary h-7 min-h-7 shrink-0 px-2.5">
              <Plus className="h-3 w-3" /> New
            </button>
          </div>
          <div className="max-h-72 overflow-y-auto p-1.5 @4xl:max-h-[448px]">
            {detectors.map((detector) => (
              <div
                key={detector.id}
                onClick={() => setSelectedId(detector.id)}
                className={`theme-menu-item flex w-full cursor-pointer items-center gap-2 rounded-lg border border-transparent px-2 py-2 text-left ${selectedId === detector.id ? 'is-selected' : ''}`}
              >
                <button type="button" className="flex min-w-0 flex-1 items-center gap-2 text-left">
                  <ContentTypeIcon type={detector.content_type as ClipContentType} className="h-4 w-4 shrink-0" />
                  <span className="theme-text-main min-w-0 flex-1 truncate font-semibold">{detector.name}</span>
                </button>
                <button
                  type="button"
                  role="switch"
                  aria-checked={selectedId === detector.id ? draft.enabled : detector.enabled}
                  aria-label={`${(selectedId === detector.id ? draft.enabled : detector.enabled) ? 'Disable' : 'Enable'} ${detector.name}`}
                  disabled={togglingId === detector.id}
                  onClick={(event) => {
                    event.stopPropagation();
                    void toggleDetector(detector);
                  }}
                  className={`settings-switch relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent disabled:cursor-wait disabled:opacity-50 ${(selectedId === detector.id ? draft.enabled : detector.enabled) ? 'is-on' : ''}`}
                >
                  <span className={`settings-switch-thumb pointer-events-none inline-block h-4 w-4 rounded-full shadow transition-transform ${(selectedId === detector.id ? draft.enabled : detector.enabled) ? 'translate-x-4' : 'translate-x-0'}`} />
                </button>
              </div>
            ))}
          </div>
        </section>

        <section className="theme-surface min-w-0 space-y-3 rounded-xl border p-3 @md:p-4" aria-label="Detector editor">
          <div className="theme-divider border-b pb-3">
            <span className="theme-text-muted font-semibold">{selectedId === 'new' ? 'New detector' : 'Detector settings'}</span>
          </div>
          <div className="grid grid-cols-1 gap-3 @2xl:grid-cols-[minmax(0,1fr)_minmax(150px,0.45fr)]">
            <label className={`space-y-1 ${modified.name ? 'settings-field-modified' : ''}`}>
              <ModifiedFieldLabel modified={modified.name}>Name</ModifiedFieldLabel>
              <input value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} className="theme-input w-full rounded-lg border px-3 py-2" />
            </label>
            <label className={`space-y-1 ${modified.content_type ? 'settings-field-modified' : ''}`}>
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
            </label>
          </div>
          <label className={`block space-y-1 ${modified.description ? 'settings-field-modified' : ''}`}>
            <ModifiedFieldLabel modified={modified.description}>Description</ModifiedFieldLabel>
            <input value={draft.description} onChange={(event) => setDraft({ ...draft, description: event.target.value })} className="theme-input w-full rounded-lg border px-3 py-2" />
          </label>
          <div className="grid grid-cols-1 items-end gap-3 @xl:grid-cols-2 @3xl:grid-cols-[110px_minmax(180px,1fr)_auto]">
            <label className={`space-y-1 ${modified.priority ? 'settings-field-modified' : ''}`}>
              <ModifiedFieldLabel modified={modified.priority}>Priority</ModifiedFieldLabel>
              <input type="number" value={draft.priority} onChange={(event) => setDraft({ ...draft, priority: Number(event.target.value) || 0 })} className="theme-input w-full rounded-lg border px-3 py-2 font-mono" />
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
            <label className={`flex min-h-9 items-center gap-2 @xl:col-span-2 @3xl:col-span-1 ${modified.enabled ? 'settings-field-modified' : ''}`}>
              <input type="checkbox" checked={draft.enabled} onChange={(event) => setDraft({ ...draft, enabled: event.target.checked })} className="theme-checkbox h-4 w-4 rounded" />
              <ModifiedFieldLabel modified={modified.enabled}>Enabled for new clips</ModifiedFieldLabel>
            </label>
          </div>
          <label className={`block space-y-1 ${modified.patterns ? 'settings-field-modified' : ''}`}>
            <ModifiedFieldLabel modified={modified.patterns}>Regular expressions <span className="font-normal">(one per line; any may match)</span></ModifiedFieldLabel>
            <textarea value={patternsText} onChange={(event) => setPatternsText(event.target.value)} spellCheck={false} className="theme-input min-h-36 w-full resize-y rounded-lg border px-3 py-2 font-mono text-[11px] leading-relaxed" />
          </label>
          {draft.validator && (
            <div className="theme-status-info rounded-lg border px-3 py-2 text-[10px]">
              Candidates also pass the built-in <strong>{draft.validator}</strong> validator to reduce false positives.
            </div>
          )}
          <div className="theme-divider grid grid-cols-1 gap-2 border-t pt-3 @md:grid-cols-[minmax(0,1fr)_auto]">
            <input value={sample} onChange={(event) => { setSample(event.target.value); setSampleMatched(null); }} placeholder="Try sample text…" className="theme-input rounded-lg border px-3 py-2 font-mono" />
            <button type="button" onClick={test} className="app-dialog-button is-secondary h-auto min-h-9">Test</button>
          </div>
          {sampleMatched !== null && (
            <div className={sampleMatched ? 'theme-status-success-text' : 'theme-status-danger-text'}>
              {sampleMatched ? 'Matches this detector' : 'Does not match this detector'}
            </div>
          )}
          <div className="theme-divider flex flex-wrap items-center gap-2 border-t pt-3">
            {selectedId !== 'new' && <button type="button" onClick={() => void duplicate()} className="app-dialog-button is-secondary"><Copy className="h-3.5 w-3.5" /> Duplicate</button>}
            {typeof selectedId === 'number' && <button type="button" onClick={remove} className="app-dialog-button is-danger"><Trash2 className="h-3.5 w-3.5" /> Delete</button>}
            {selectedId === 'new' && <button type="button" onClick={cancelNewDetector} className="app-dialog-button is-secondary ml-auto"><X className="h-3.5 w-3.5" /> Cancel</button>}
            {selected?.is_builtin && <button type="button" onClick={resetSelectedDraft} disabled={!hasModifiedFields || saving} className="app-dialog-button is-secondary ml-auto"><RotateCcw className="h-3.5 w-3.5" /> Reset to Default</button>}
            <button type="button" onClick={save} disabled={saving} className={`app-dialog-button is-primary ${selectedId === 'new' || selected?.is_builtin ? '' : 'ml-auto'}`}><Save className="h-3.5 w-3.5" /> Save</button>
          </div>
        </section>
      </div>
      </div>
      <ContentTypeManagerDialog isOpen={isTypeManagerOpen} onClose={() => setIsTypeManagerOpen(false)} />
    </div>
  );
}
