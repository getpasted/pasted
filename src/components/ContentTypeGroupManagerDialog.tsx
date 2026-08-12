import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { Archive, Layers3, Plus, RotateCcw, Save, Trash2 } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';
import { useContentTypes, type RegisteredContentTypeGroup } from './ContentTypeProvider';
import { ModifiedFieldLabel } from './ModifiedFieldLabel';
import { useToast } from './ToastProvider';

type GroupDraft = Pick<RegisteredContentTypeGroup, 'id' | 'label' | 'sortOrder'>;
const newDraft = (): GroupDraft => ({ id: '', label: '', sortOrder: 100 });

export function ContentTypeGroupManagerDialog({ isOpen, onClose }: { isOpen: boolean; onClose: () => void }) {
  const { definitions, groups, refresh, refreshGroups } = useContentTypes();
  const { showToast } = useToast();
  const [selectedId, setSelectedId] = useState<string | 'new'>('new');
  const selected = useMemo(() => groups.find(({ id }) => id === selectedId), [groups, selectedId]);
  const [draft, setDraft] = useState<GroupDraft>(newDraft());
  const [saving, setSaving] = useState(false);
  const previousSelectedIdRef = useRef<string | null>(null);

  useEffect(() => {
    if (!isOpen) return;
    void refreshGroups().then((loaded) => setSelectedId((current) => current === 'new' ? loaded[0]?.id ?? 'new' : current));
  }, [isOpen, refreshGroups]);
  useLayoutEffect(() => {
    setDraft(selected ? { id: selected.id, label: selected.label, sortOrder: selected.sortOrder } : newDraft());
  }, [selected]);

  const comparisonDraft = selectedId === 'new'
    ? null
    : selected?.isBuiltin ? selected.defaults : selected ? { label: selected.label, sortOrder: selected.sortOrder } : null;
  const modified = {
    label: comparisonDraft !== null && draft.label.trim() !== comparisonDraft.label,
    sortOrder: comparisonDraft !== null && draft.sortOrder !== comparisonDraft.sortOrder,
  };
  const hasModifiedFields = Object.values(modified).some(Boolean);
  const resetSelectedDraft = () => {
    if (!selected?.isBuiltin || !selected.defaults) return;
    setDraft({ id: selected.id, ...selected.defaults });
  };

  const beginNew = () => {
    if (selectedId !== 'new') previousSelectedIdRef.current = selectedId;
    setSelectedId('new');
  };
  const cancelNew = () => {
    const previousId = previousSelectedIdRef.current;
    setSelectedId(previousId && groups.some(({ id }) => id === previousId) ? previousId : groups[0]?.id ?? 'new');
  };
  const save = async () => {
    setSaving(true);
    try {
      const input = { ...draft, id: draft.id.trim(), label: draft.label.trim() };
      const saved = selectedId === 'new'
        ? await invoke<RegisteredContentTypeGroup>('create_content_type_group', { input })
        : await invoke<RegisteredContentTypeGroup>('update_content_type_group', { id: selectedId, input });
      await Promise.all([refresh(), refreshGroups()]);
      setSelectedId(saved.id);
      showToast({ tone: 'success', message: `${saved.label} saved.` });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    } finally {
      setSaving(false);
    }
  };
  const toggleArchived = async () => {
    if (!selected || selected.isBuiltin) return;
    try {
      await invoke('set_content_type_group_archived', { id: selected.id, archived: !selected.isArchived });
      await Promise.all([refresh(), refreshGroups()]);
      showToast({ tone: 'success', message: `${selected.label} ${selected.isArchived ? 'restored' : 'archived'}.` });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    }
  };
  const remove = async () => {
    if (!selected || selected.isBuiltin) return;
    if (!window.confirm(`Permanently delete the empty Group “${selected.label}”? This cannot be undone.`)) return;
    try {
      await invoke('delete_content_type_group', { id: selected.id });
      const loaded = await refreshGroups();
      setSelectedId(loaded[0]?.id ?? 'new');
      showToast({ tone: 'success', message: `${selected.label} deleted.` });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    }
  };
  const usageCount = selected ? definitions.filter(({ group }) => group === selected.id).length : 0;

  return <AppDialog isOpen={isOpen} onClose={onClose} labelledBy="content-type-group-manager-title" panelClassName="flex max-h-[86vh] w-full max-w-2xl flex-col overflow-hidden border shadow-2xl">
    {({ requestClose }) => <>
      <AppDialogHeader onClose={requestClose} className="shrink-0">
        <AppDialogHeading id="content-type-group-manager-title" title="Content Type Groups" description="Organize Types with stable, reusable groups." icon={<Layers3 />} />
      </AppDialogHeader>
      <AppDialogBody className="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-y-auto text-xs sm:grid-cols-[minmax(170px,0.7fr)_minmax(280px,1.3fr)]">
        <section className="theme-surface flex min-h-[230px] flex-col overflow-hidden rounded-xl border sm:min-h-0">
          <div className="theme-divider flex items-center justify-between border-b p-2">
            <span className="theme-text-muted px-1 text-[10px] font-bold uppercase tracking-wider">Groups</span>
            <button type="button" onClick={beginNew} className="app-dialog-button is-secondary h-7 min-h-7 px-2.5"><Plus className="h-3 w-3" /> New</button>
          </div>
          <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
            {groups.map((group) => <button key={group.id} type="button" onClick={() => setSelectedId(group.id)} className={`theme-menu-item flex w-full items-center gap-2 rounded-lg border border-transparent px-2 py-2 text-left ${selectedId === group.id ? 'is-selected' : ''} ${group.isArchived ? 'opacity-55' : ''}`}>
              <Layers3 className="h-4 w-4 shrink-0" />
              <span className="theme-text-main min-w-0 flex-1 truncate font-semibold">{group.label}</span>
              <span className="theme-text-subtle tabular-nums">{definitions.filter(({ group: id }) => id === group.id).length}</span>
            </button>)}
          </div>
        </section>
        <section className="theme-surface min-w-0 space-y-4 rounded-xl border p-4">
          <label className={`block space-y-1 ${modified.label ? 'settings-field-modified' : ''}`}><ModifiedFieldLabel modified={modified.label}>Name</ModifiedFieldLabel><input value={draft.label} onChange={(event) => setDraft({ ...draft, label: event.target.value })} className="theme-input w-full rounded-lg border px-3 py-2" /></label>
          <div className="grid grid-cols-[minmax(0,1fr)_110px] gap-3">
            <label className="space-y-1"><span className="theme-text-muted font-semibold">Stable ID</span><input value={draft.id} disabled={selectedId !== 'new'} onChange={(event) => setDraft({ ...draft, id: event.target.value.toLowerCase().replace(/[^a-z0-9_]/g, '_') })} className="theme-input w-full rounded-lg border px-3 py-2 font-mono disabled:opacity-60" /></label>
            <label className={`space-y-1 ${modified.sortOrder ? 'settings-field-modified' : ''}`}><ModifiedFieldLabel modified={modified.sortOrder}>Sort order</ModifiedFieldLabel><input type="number" value={draft.sortOrder} onChange={(event) => setDraft({ ...draft, sortOrder: Number(event.target.value) || 0 })} className="theme-input w-full rounded-lg border px-3 py-2 font-mono" /></label>
          </div>
          <div className="theme-subtle-surface rounded-lg border p-3 text-[11px] leading-relaxed">
            {selected?.isBuiltin ? 'Built-in Group IDs cannot be changed or archived. Names and sort order can be restored later.' : `Custom Groups can be archived when empty. ${usageCount} Type${usageCount === 1 ? '' : 's'} currently use this Group.`}
          </div>
        </section>
      </AppDialogBody>
      <AppDialogFooter align="between" className="shrink-0">
        <div className="flex items-center gap-2">{selected && !selected.isBuiltin && <><AppDialogButton onClick={() => void toggleArchived()} variant={selected.isArchived ? 'secondary' : 'warning'} disabled={!selected.isArchived && usageCount > 0}><Archive className="h-3.5 w-3.5" /> {selected.isArchived ? 'Restore Group' : 'Archive Group'}</AppDialogButton><AppDialogButton onClick={() => void remove()} variant="danger" disabled={usageCount > 0}><Trash2 className="h-3.5 w-3.5" /> Delete</AppDialogButton></>}</div>
        <div className="flex items-center gap-2">{selectedId === 'new' ? <AppDialogButton onClick={cancelNew}>Cancel</AppDialogButton> : <AppDialogButton onClick={requestClose}>Close</AppDialogButton>}{selected?.isBuiltin && <AppDialogButton onClick={resetSelectedDraft} disabled={!hasModifiedFields || saving}><RotateCcw className="h-3.5 w-3.5" /> Reset to Default</AppDialogButton>}<AppDialogButton variant="primary" onClick={() => void save()} disabled={saving}><Save className="h-3.5 w-3.5" /> Save</AppDialogButton></div>
      </AppDialogFooter>
    </>}
  </AppDialog>;
}
