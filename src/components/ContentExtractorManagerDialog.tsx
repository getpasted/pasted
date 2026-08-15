import { useEffect, useLayoutEffect, useMemo, useState } from 'react';
import { CircleAlert, CircleCheck, Copy, Plus, RotateCcw, ScanText, Trash2 } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading, SaveButtonContent } from './AppDialogLayout';
import { ModifiedFieldLabel } from './ModifiedFieldLabel';
import { RegistryListItem } from './RegistryListItem';
import { RegistryPanelHeader } from './RegistryPanelHeader';
import { SettingsSwitch } from './SettingsSwitch';
import { useToast } from './ToastProvider';

export interface ContentExtractor {
  id: number;
  stableRef: string;
  name: string;
  description: string;
  engine: string;
  inputContract: string;
  outputContract: string;
  enabled: boolean;
  priority: number;
  isBuiltin: boolean;
  isAvailable: boolean;
  unavailableReason: string | null;
  defaults: ExtractorInput | null;
}

interface ExtractorInput {
  name: string;
  description: string;
  engine: string;
  inputContract: string;
  outputContract: string;
  enabled: boolean;
  priority: number;
}

function toInput(extractor?: ContentExtractor): ExtractorInput {
  return extractor ? {
    name: extractor.name,
    description: extractor.description,
    engine: extractor.engine,
    inputContract: extractor.inputContract,
    outputContract: extractor.outputContract,
    enabled: extractor.enabled,
    priority: extractor.priority,
  } : {
    name: 'Custom Extractor',
    description: 'Extracts searchable text from images.',
    engine: 'macos-vision-v1',
    inputContract: 'image',
    outputContract: 'searchable_text',
    enabled: true,
    priority: 100,
  };
}

export function ContentExtractorManagerDialog({
  isOpen,
  onClose,
  onChanged,
}: {
  isOpen: boolean;
  onClose: () => void;
  onChanged?: () => void;
}) {
  const { showToast } = useToast();
  const [extractors, setExtractors] = useState<ContentExtractor[]>([]);
  const [selectedId, setSelectedId] = useState<number | 'new' | null>(null);
  const [draft, setDraft] = useState<ExtractorInput>(toInput());
  const [saving, setSaving] = useState(false);
  const selected = useMemo(
    () => typeof selectedId === 'number' ? extractors.find((extractor) => extractor.id === selectedId) : undefined,
    [extractors, selectedId],
  );

  const load = async () => {
    const loaded = await invoke<ContentExtractor[]>('get_content_extractors');
    setExtractors(loaded);
    setSelectedId((current) => loaded.some(({ id }) => id === current) ? current : loaded[0]?.id ?? null);
  };

  useEffect(() => {
    if (isOpen) void load();
  }, [isOpen]);
  useLayoutEffect(() => setDraft(selectedId === 'new' ? toInput() : toInput(selected)), [selected, selectedId]);

  const baseline = selectedId === 'new' ? toInput() : selected ? toInput(selected) : null;
  const isDirty = baseline !== null && JSON.stringify(draft) !== JSON.stringify(baseline);
  const defaults = selected?.defaults;
  const defaultDraft = selected && defaults ? { ...toInput(selected), ...defaults } : null;
  const differsFromDefaults = defaultDraft !== null && JSON.stringify(draft) !== JSON.stringify(defaultDraft);

  const save = async () => {
    setSaving(true);
    try {
      const saved = selectedId === 'new'
        ? await invoke<ContentExtractor>('create_content_extractor', { input: draft })
        : await invoke<ContentExtractor>('update_content_extractor_definition', { id: selectedId, input: draft });
      await load();
      setSelectedId(saved.id);
      onChanged?.();
      showToast({ tone: 'success', message: `${saved.name} saved.` });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    } finally {
      setSaving(false);
    }
  };

  const restoreAll = async () => {
    try {
      const restored = await invoke<ContentExtractor[]>('restore_default_content_extractors');
      setExtractors(restored);
      setSelectedId(restored[0]?.id ?? null);
      onChanged?.();
      showToast({ tone: 'success', message: 'Built-in Extractors restored.' });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    }
  };

  const resetDraft = () => {
    if (defaultDraft) setDraft(defaultDraft);
  };

  const toggle = async (extractor: ContentExtractor) => {
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

  const duplicate = async () => {
    if (!selected) return;
    try {
      const created = await invoke<ContentExtractor>('duplicate_content_extractor', {
        reference: selected.stableRef,
        name: `${draft.name || selected.name} Copy`,
      });
      await load();
      setSelectedId(created.id);
      onChanged?.();
      showToast({ tone: 'success', message: `${created.name} created.` });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    }
  };

  const remove = async () => {
    if (!selected) return;
    if (!window.confirm(`Delete the Extractor “${selected.name}”? Shipped Extractors can be recovered with Restore Defaults.`)) return;
    try {
      await invoke('delete_content_extractor', { id: selected.id });
      const remaining = extractors.filter((extractor) => extractor.id !== selected.id);
      setExtractors(remaining);
      setSelectedId(remaining[0]?.id ?? 'new');
      onChanged?.();
      showToast({ tone: 'success', message: `${selected.name} deleted.` });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    }
  };

  return <AppDialog
    isOpen={isOpen}
    onClose={onClose}
    labelledBy="extractor-manager-title"
    isDirty={isDirty}
    discardMessage="Discard changes to this Extractor?"
    panelClassName="theme-panel @container flex max-h-[90vh] w-full max-w-4xl flex-col overflow-hidden border shadow-2xl"
  >
    {({ requestClose }) => <>
      <AppDialogHeader onClose={requestClose} className="shrink-0">
        <AppDialogHeading
          id="extractor-manager-title"
          title="Extractors"
          description="Create searchable representations from clip content. The lowest priority number runs first."
          icon={<ScanText />}
        />
      </AppDialogHeader>
      <AppDialogBody className="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-y-auto text-xs @xl:grid-cols-[minmax(0,3fr)_minmax(0,7fr)]">
        <section className="theme-surface flex min-h-[260px] flex-col overflow-hidden rounded-xl border @xl:min-h-0">
          <RegistryPanelHeader title="Extractors" actions={<AppDialogButton onClick={() => setSelectedId('new')} className="h-7 min-h-7 px-2.5"><Plus className="h-3.5 w-3.5" /> New</AppDialogButton>} />
          <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
            {extractors.map((extractor) => <RegistryListItem
              key={extractor.id}
              selected={selectedId === extractor.id}
              onSelect={() => setSelectedId(extractor.id)}
              icon={<ScanText className="h-4 w-4" />}
              title={extractor.name}
              subtitle={extractor.isAvailable ? extractor.description : extractor.unavailableReason}
              muted={!extractor.isAvailable}
              trailing={<SettingsSwitch
                checked={extractor.enabled}
                label={extractor.name}
                onClick={() => void toggle(extractor)}
              />}
            />)}
          </div>
          <div className="theme-divider flex justify-end border-t p-2">
            <AppDialogButton onClick={() => void restoreAll()} className="h-7 min-h-7 px-2.5">
              <RotateCcw className="h-3.5 w-3.5" /> Restore Defaults
            </AppDialogButton>
          </div>
        </section>
        <section className="theme-surface flex min-w-0 flex-col overflow-hidden rounded-xl border">
          <RegistryPanelHeader title="Extractor Settings" />
          <div className="min-h-0 flex-1 space-y-4 overflow-y-auto p-4">
            <div className="grid grid-cols-1 gap-3 @md:grid-cols-[minmax(0,1fr)_110px]">
              <label className="space-y-1">
                <ModifiedFieldLabel modified={selectedId !== 'new' && draft.name !== defaults?.name}>Name</ModifiedFieldLabel>
                <input value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} className="theme-input ui-field-radius w-full border px-3 py-2" />
              </label>
              <label className="space-y-1">
                <ModifiedFieldLabel modified={selectedId !== 'new' && draft.priority !== defaults?.priority}>Priority</ModifiedFieldLabel>
                <input type="number" value={draft.priority} onChange={(event) => setDraft({ ...draft, priority: Number(event.target.value) || 0 })} className="theme-input ui-field-radius w-full border px-3 py-2 font-mono" />
              </label>
            </div>
            <label className="block space-y-1">
              <ModifiedFieldLabel modified={selectedId !== 'new' && draft.description !== defaults?.description}>Description</ModifiedFieldLabel>
              <input value={draft.description} onChange={(event) => setDraft({ ...draft, description: event.target.value })} className="theme-input ui-field-radius w-full border px-3 py-2" />
            </label>
            <div className="grid grid-cols-1 gap-3 @md:grid-cols-3">
              <div className="theme-subtle-surface rounded-lg border p-3">
                <span className="theme-text-muted block text-[10px] font-semibold">Pass</span>
                <strong className="theme-text-main mt-1 block font-mono">extract</strong>
              </div>
              <label className="space-y-1"><span className="theme-text-muted block text-[10px] font-semibold">Input</span><input value={draft.inputContract} disabled className="theme-input ui-field-radius w-full border px-3 py-2 font-mono disabled:opacity-60" /></label>
              <label className="space-y-1"><span className="theme-text-muted block text-[10px] font-semibold">Output</span><input value={draft.outputContract} disabled className="theme-input ui-field-radius w-full border px-3 py-2 font-mono disabled:opacity-60" /></label>
            </div>
            <label className="block space-y-1">
              <span className="theme-text-muted font-semibold">Engine</span>
              <input value={draft.engine} disabled={selected?.isBuiltin} onChange={(event) => setDraft({ ...draft, engine: event.target.value })} className="theme-input ui-field-radius w-full border px-3 py-2 font-mono disabled:opacity-60" />
            </label>
            <label className="flex items-center gap-2">
              <input type="checkbox" checked={draft.enabled} onChange={(event) => setDraft({ ...draft, enabled: event.target.checked })} className="theme-checkbox h-4 w-4 rounded" />
              <ModifiedFieldLabel modified={selectedId !== 'new' && draft.enabled !== defaults?.enabled}>Enabled</ModifiedFieldLabel>
            </label>
            {selected && (
              <div className="theme-subtle-surface flex items-center justify-between gap-3 rounded-lg border p-3">
                <span className="theme-text-muted text-[10px] font-semibold">Availability</span>
                <span className={`${selected.isAvailable ? 'theme-status-success-text' : 'theme-status-warning-text'} flex items-center gap-1.5 text-right text-[11px] font-semibold`}>
                  {selected.isAvailable
                    ? <><CircleCheck className="h-3.5 w-3.5 shrink-0" /> Available on this system</>
                    : <><CircleAlert className="h-3.5 w-3.5 shrink-0" /> {selected.unavailableReason}</>}
                </span>
              </div>
            )}
          </div>
        </section>
      </AppDialogBody>
      <AppDialogFooter align="between" className="shrink-0">
        <div className="flex items-center gap-2">
          {selected && <AppDialogButton onClick={() => void duplicate()}><Copy className="h-3.5 w-3.5" /> Duplicate</AppDialogButton>}
          {selected && <AppDialogButton variant="danger" onClick={() => void remove()}><Trash2 className="h-3.5 w-3.5" /> Delete</AppDialogButton>}
        </div>
        <div className="flex items-center gap-2">
          {selectedId === 'new' ? <AppDialogButton onClick={() => setSelectedId(extractors[0]?.id ?? null)}>Cancel</AppDialogButton> : <AppDialogButton onClick={requestClose}>Close</AppDialogButton>}
          {selected?.isBuiltin && <AppDialogButton onClick={resetDraft} disabled={!differsFromDefaults || saving}><RotateCcw className="h-3.5 w-3.5" /> Reset to Default</AppDialogButton>}
          <AppDialogButton variant="primary" onClick={() => void save()} disabled={selectedId === null || saving}><SaveButtonContent isSaving={saving} /></AppDialogButton>
        </div>
      </AppDialogFooter>
    </>}
  </AppDialog>;
}
