import { useEffect, useLayoutEffect, useMemo, useState } from 'react';
import { Archive, Layers3, Plus, RotateCcw, Shapes } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading, SaveButtonContent } from './AppDialogLayout';
import { ContentTypeGlyph } from './ContentTypeIcon';
import { ContentTypeGroupManagerDialog } from './ContentTypeGroupManagerDialog';
import { MenuSelect } from './MenuSelect';
import { ModifiedFieldLabel } from './ModifiedFieldLabel';
import { useContentTypes, type RegisteredContentType } from './ContentTypeProvider';
import { useToast } from './ToastProvider';
import { RegistryListItem } from './RegistryListItem';
import { RegistryPanelHeader } from './RegistryPanelHeader';
import { useNewItemSelection } from '../hooks/useNewItemSelection';
import { ConnectedMenuAction } from './ConnectedMenuAction';
import { ConfirmationDialog, type ConfirmationDialogRequest } from './ConfirmationDialog';

const ICONS = [
  'AlignLeft', 'AtSign', 'Binary', 'BookOpen', 'Box', 'Braces', 'Calendar',
  'CheckSquare', 'CircleDollarSign', 'Clipboard', 'Clock', 'Database',
  'Type', 'ScrollText', 'Link', 'Mail', 'Phone', 'Image', 'Files', 'MapPin',
  'Palette', 'Code', 'TerminalSquare', 'Variable', 'FileCode2', 'KeyRound',
  'CreditCard', 'Landmark', 'ShieldKeyhole', 'Hash', 'Network', 'Router',
  'Fingerprint', 'FileText', 'FileJson', 'FileSpreadsheet', 'Folder', 'Globe',
  'Heart', 'List', 'Lock', 'MessageSquare', 'Package', 'Receipt', 'Search',
  'Settings', 'Star', 'Tag', 'User', 'Wallet', 'Wrench', 'Zap',
];

type TypeDraft = Pick<RegisteredContentType, 'id' | 'label' | 'icon' | 'group'>;
const newDraft = (): TypeDraft => ({ id: '', label: '', icon: 'FileText', group: 'custom' });

export function ContentTypeManagerDialog({ isOpen, onClose }: { isOpen: boolean; onClose: () => void }) {
  const { definitions, groups, refresh } = useContentTypes();
  const { showToast } = useToast();
  const [selectedId, setSelectedId] = useState<string | 'new'>('new');
  const selected = useMemo(() => definitions.find(({ id }) => id === selectedId), [definitions, selectedId]);
  const [draft, setDraft] = useState<TypeDraft>(newDraft());
  const [saving, setSaving] = useState(false);
  const [isGroupManagerOpen, setIsGroupManagerOpen] = useState(false);
  const [confirmation, setConfirmation] = useState<ConfirmationDialogRequest | null>(null);

  useEffect(() => {
    if (!isOpen) return;
    void refresh().then((loaded) => setSelectedId((current) => current === 'new' ? loaded[0]?.id ?? 'new' : current));
  }, [isOpen, refresh]);
  useLayoutEffect(() => { setDraft(selected ? { id: selected.id, label: selected.label, icon: selected.icon, group: selected.group } : newDraft()); }, [selected]);

  const comparisonDraft = selectedId === 'new'
    ? null
    : selected?.isBuiltin ? selected.defaults : selected ? { id: selected.id, label: selected.label, icon: selected.icon, group: selected.group } : null;
  const modified = {
    label: comparisonDraft !== null && draft.label.trim() !== comparisonDraft.label,
    icon: comparisonDraft !== null && draft.icon !== comparisonDraft.icon,
    group: comparisonDraft !== null && draft.group !== comparisonDraft.group,
  };
  const hasModifiedFields = Object.values(modified).some(Boolean);

  const resetSelectedDraft = () => {
    if (!selected?.isBuiltin || !selected.defaults) return;
    setDraft({ id: selected.id, ...selected.defaults });
  };

  const { beginNew: beginNewType, cancelNew: cancelNewType } = useNewItemSelection({
    selectedId,
    setSelectedId,
    itemIds: definitions.map(({ id }) => id),
    emptySelection: 'new',
  });

  const save = async () => {
    setSaving(true);
    try {
      const input = { ...draft, id: draft.id.trim(), label: draft.label.trim() };
      const saved = selectedId === 'new'
        ? await invoke<RegisteredContentType>('create_content_type', { input })
        : await invoke<RegisteredContentType>('update_content_type', { id: selectedId, input });
      await refresh();
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
    const action = selected.isArchived ? 'restore' : 'archive';
    try {
      await invoke('set_content_type_archived', { id: selected.id, archived: !selected.isArchived });
      await refresh();
      showToast({ tone: 'success', message: `${selected.label} ${action}d.` });
    } catch (error) {
      showToast({ tone: 'error', message: String(error) });
    }
  };

  const requestToggleArchived = () => {
    if (!selected || selected.isBuiltin) return;
    if (selected.isArchived) {
      void toggleArchived();
      return;
    }
    setConfirmation({
      title: 'Archive Content Type?',
      description: `“${selected.label}” will be archived.`,
      details: 'Its Classifiers will be disabled. Existing clips will keep this Content Type.',
      confirmLabel: 'Archive',
      tone: 'warning',
      onConfirm: async () => {
        setConfirmation(null);
        await toggleArchived();
      },
    });
  };

  return <>
    <AppDialog
      isOpen={isOpen}
      onClose={onClose}
      labelledBy="content-type-manager-title"
      panelClassName="theme-panel @container flex max-h-[90vh] w-full max-w-4xl flex-col overflow-hidden border shadow-2xl"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose} className="shrink-0">
          <AppDialogHeading id="content-type-manager-title" title="Content Types" description="Manage shared names, icons, and groups. Content Type IDs remain stable." icon={<Shapes />} />
        </AppDialogHeader>
        <AppDialogBody className="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-y-auto text-xs @xl:grid-cols-[minmax(0,3fr)_minmax(0,7fr)]">
          <section className="theme-surface flex min-h-[260px] flex-col overflow-hidden rounded-xl border @xl:min-h-0">
            <RegistryPanelHeader
              title="Registered Content Types"
              actions={<AppDialogButton onClick={beginNewType} className="h-7 min-h-7 px-2.5"><Plus className="h-3 w-3" /> New</AppDialogButton>}
            />
            <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
              {definitions.map((item) => <RegistryListItem
                key={item.id}
                selected={selectedId === item.id}
                onSelect={() => setSelectedId(item.id)}
                icon={<ContentTypeGlyph icon={item.icon} className="h-4 w-4" />}
                title={item.label}
                muted={item.isArchived}
                trailing={item.isArchived && <Archive className="theme-text-subtle h-3.5 w-3.5" />}
              />)}
            </div>
          </section>
          <section className="theme-surface flex min-w-0 flex-col overflow-hidden rounded-xl border">
            <RegistryPanelHeader title="Content Type Settings" />
            <div className="min-h-0 flex-1 space-y-4 overflow-y-auto p-4">
            <div className="grid grid-cols-1 gap-3 @md:grid-cols-[1fr_150px]">
              <label className={`space-y-1 ${modified.label ? 'settings-field-modified' : ''}`}><ModifiedFieldLabel modified={modified.label}>Name</ModifiedFieldLabel><input value={draft.label} onChange={(event) => setDraft({ ...draft, label: event.target.value })} className="theme-input w-full rounded-lg border px-3 py-2" /></label>
              <label className="space-y-1"><span className="theme-text-muted font-semibold">Stable ID</span><input value={draft.id} disabled={selectedId !== 'new'} onChange={(event) => setDraft({ ...draft, id: event.target.value.toLowerCase().replace(/[^a-z0-9_]/g, '_') })} className="theme-input w-full rounded-lg border px-3 py-2 font-mono disabled:opacity-60" /></label>
            </div>
            <div className="grid grid-cols-1 gap-3 @md:grid-cols-[minmax(180px,0.7fr)_minmax(320px,1.3fr)]">
              <label className={`space-y-1 ${modified.icon ? 'settings-field-modified' : ''}`}><ModifiedFieldLabel modified={modified.icon}>Icon</ModifiedFieldLabel><MenuSelect value={draft.icon} onChange={(icon) => setDraft({ ...draft, icon })} label="Content type icon" leadingIcon={<ContentTypeGlyph icon={draft.icon} className="h-4 w-4" />} options={ICONS.map((icon) => ({ value: icon, label: icon.replace(/([a-z])([A-Z])/g, '$1 $2'), icon: <ContentTypeGlyph icon={icon} className="h-4 w-4" /> }))} className="w-full" searchable searchPlaceholder="Search icons…" /></label>
              <div className={`space-y-1 ${modified.group ? 'settings-field-modified' : ''}`}>
                <ModifiedFieldLabel modified={modified.group}>Group</ModifiedFieldLabel>
                <ConnectedMenuAction
                  className="w-full"
                  groupLabel="Content type group"
                  actionLabel="Manage Content Type Groups"
                  action={<><Layers3 className="h-3.5 w-3.5" aria-hidden="true" /><span>Manage…</span></>}
                  onAction={() => setIsGroupManagerOpen(true)}
                >
                  <MenuSelect value={draft.group} onChange={(group) => setDraft({ ...draft, group })} label="Content type group" options={groups.map((group) => ({ value: group.id, label: group.label, disabled: group.isArchived }))} className="min-w-0 flex-1" />
                </ConnectedMenuAction>
              </div>
            </div>
            <div className="theme-subtle-surface rounded-lg border p-3 text-[11px] leading-relaxed">
              {selected?.isBuiltin ? 'Built-in IDs cannot be changed or archived. Their name, icon, and group can be customized and reset later.' : 'Custom Content Types can be archived without changing historical clips. Archiving also disables Classifiers that produce the Content Type.'}
            </div>
            </div>
          </section>
        </AppDialogBody>
        <AppDialogFooter align="between" className="shrink-0">
          <div>{selected && !selected.isBuiltin && <AppDialogButton onClick={requestToggleArchived} variant={selected.isArchived ? 'secondary' : 'warning'}><Archive className="h-3.5 w-3.5" /> {selected.isArchived ? 'Restore Content Type' : 'Archive Content Type…'}</AppDialogButton>}</div>
          <div className="flex items-center gap-2">
            {selectedId === 'new'
              ? <AppDialogButton onClick={cancelNewType}>Cancel</AppDialogButton>
              : <AppDialogButton onClick={requestClose}>Close</AppDialogButton>}
            {selected?.isBuiltin && <AppDialogButton onClick={resetSelectedDraft} disabled={!hasModifiedFields || saving}><RotateCcw className="h-3.5 w-3.5" /> Reset to Default</AppDialogButton>}
            <AppDialogButton variant="primary" onClick={() => void save()} disabled={saving}><SaveButtonContent isSaving={saving} /></AppDialogButton>
          </div>
        </AppDialogFooter>
      </>}
    </AppDialog>
    <ContentTypeGroupManagerDialog isOpen={isGroupManagerOpen} onClose={() => setIsGroupManagerOpen(false)} />
    <ConfirmationDialog request={confirmation} onCancel={() => setConfirmation(null)} />
  </>;
}
