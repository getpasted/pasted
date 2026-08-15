import { useEffect, useMemo, useState, type ComponentType } from 'react';
import { FileAudio, Lightbulb, ScanSearch } from 'lucide-react';
import type { LibraryItemView } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';
import { RegistryDetailHeader } from './RegistryDetailHeader';
import { RegistryListItem } from './RegistryListItem';
import { RegistryPanelHeader } from './RegistryPanelHeader';

type BuiltinAnalysisKind = Extract<LibraryItemView['kind'], 'inspector' | 'enricher'>;

interface InspectorDefinition {
  stableRef: string;
  engine: string | null;
  isAvailable: boolean;
  unavailableReason: string | null;
}

function engineLabel(engine: string) {
  if (engine === 'ffprobe-cli-v1') return 'ffprobe';
  if (engine === 'mediainfo-cli-v1') return 'MediaInfo';
  return engine;
}

export function BuiltinAnalysisManagerDialog({
  isOpen,
  onClose,
  kind,
  title,
  description,
  icon: HeadingIcon,
}: {
  isOpen: boolean;
  onClose: () => void;
  kind: BuiltinAnalysisKind;
  title: string;
  description: string;
  icon: ComponentType<{ className?: string }>;
}) {
  const [items, setItems] = useState<LibraryItemView[]>([]);
  const [inspectors, setInspectors] = useState<InspectorDefinition[]>([]);
  const [selectedRef, setSelectedRef] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen) return;
    const load = async () => {
      try {
        const [loaded, loadedInspectors] = await Promise.all([
          invoke<LibraryItemView[]>('get_library_items', { kind, includeArchived: false }),
          kind === 'inspector'
            ? invoke<InspectorDefinition[]>('get_content_inspectors')
            : Promise.resolve([]),
        ]);
        setItems(loaded);
        setInspectors(loadedInspectors);
        setSelectedRef((current) => loaded.some(({ stableRef }) => stableRef === current) ? current : loaded[0]?.stableRef ?? null);
        setError(null);
      } catch (reason) {
        setError(String(reason));
      }
    };
    void load();
  }, [isOpen, kind]);

  const selected = useMemo(
    () => items.find(({ stableRef }) => stableRef === selectedRef),
    [items, selectedRef],
  );
  const contract = selected?.participantContract;
  const runtime = inspectors.find(({ stableRef }) => stableRef === selectedRef);
  const worksWith = kind === 'enricher'
    ? 'Clips with analyzable text'
    : selected?.typeRelations?.some(({ kind: relationKind, typeId }) => relationKind === 'accepts' && typeId === 'file')
      ? 'File clips'
      : 'All clips';
  const provides = kind === 'enricher'
    ? 'Smart Action recommendations'
    : selected?.stableRef.includes('media') ? 'Media metadata' : 'Structural details';
  const availabilityText = runtime?.engine
    ? runtime.isAvailable
      ? `${engineLabel(runtime.engine)} available`
      : runtime.unavailableReason?.match(/^.*?\.(?:\s|$)/)?.[0].trim() ?? 'Engine unavailable'
    : null;
  const DetailIcon = kind === 'inspector' && selected?.stableRef.includes('media')
    ? FileAudio
    : kind === 'enricher' ? Lightbulb : ScanSearch;

  return (
    <AppDialog
      isOpen={isOpen}
      onClose={onClose}
      labelledBy={`${kind}-manager-title`}
      panelClassName="theme-panel @container flex max-h-[90vh] w-full max-w-4xl flex-col overflow-hidden border shadow-2xl"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose} className="shrink-0">
          <AppDialogHeading id={`${kind}-manager-title`} title={title} description={description} icon={<HeadingIcon />} />
        </AppDialogHeader>
        <AppDialogBody className="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-y-auto text-xs @xl:grid-cols-[minmax(0,3fr)_minmax(0,7fr)]">
          <section className="theme-surface flex min-h-[220px] flex-col overflow-hidden rounded-xl border @xl:min-h-0">
            <RegistryPanelHeader title={title} />
            <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
              {items.map((item) => (
                <RegistryListItem
                  key={item.stableRef}
                  selected={selectedRef === item.stableRef}
                  onSelect={() => setSelectedRef(item.stableRef)}
                  icon={<HeadingIcon className="h-4 w-4" />}
                  title={item.name}
                  subtitle={item.description}
                />
              ))}
            </div>
          </section>
          <section className="theme-surface flex min-w-0 flex-col overflow-hidden rounded-xl border">
            <RegistryPanelHeader title={`${kind === 'inspector' ? 'Inspector' : 'Enricher'} Details`} />
            <div className="min-h-0 flex-1 space-y-4 overflow-y-auto p-4">
              {error && <div role="alert" className="theme-status-danger rounded-lg border px-3 py-2">Definitions could not be loaded.</div>}
              {selected && <>
                <RegistryDetailHeader
                  icon={<DetailIcon className="h-5 w-5" />}
                  title={selected.name}
                  meta={selected.description}
                  trailing={availabilityText ? <span
                    className={`max-w-56 rounded border px-2 py-1 text-right text-[9px] font-semibold ${runtime?.isAvailable ? 'theme-status-success' : 'theme-status-warning'}`}
                    title={runtime?.unavailableReason ?? `The ${engineLabel(runtime?.engine ?? '')} engine is ready.`}
                  >{availabilityText}</span> : undefined}
                />
                <div className="grid grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-stretch gap-2">
                  <div className="theme-subtle-surface rounded-lg border p-3">
                    <span className="theme-text-muted block text-[9px] font-semibold">Works with</span>
                    <strong className="theme-text-main mt-1 block text-[11px]">{worksWith}</strong>
                  </div>
                  <span className="theme-text-muted self-center" aria-hidden="true">→</span>
                  <div className="theme-subtle-surface rounded-lg border p-3">
                    <span className="theme-text-muted block text-[9px] font-semibold">Provides</span>
                    <strong className="theme-text-main mt-1 block text-[11px]">{provides}</strong>
                  </div>
                </div>
                <details className="theme-subtle-surface rounded-lg border px-3 py-2">
                  <summary className="theme-text-main cursor-pointer text-[10px] font-semibold">Technical details</summary>
                  <div className="theme-divider mt-3 space-y-3 border-t pt-3 text-[10px]">
                    <p className="theme-text-subtle leading-relaxed">
                      The stable reference identifies this {kind} in the CLI and API. Use it with <code>pasted {kind} get &lt;ref&gt; --json</code>.
                    </p>
                    <div className="space-y-1">
                      <span className="theme-text-muted block font-semibold">Stable reference</span>
                      <code className="theme-input block break-all rounded-lg border px-3 py-2">{selected.stableRef}</code>
                    </div>
                    {contract && <div className="grid grid-cols-1 gap-2 @md:grid-cols-2">
                      <div><span className="theme-text-muted block font-semibold">Analysis pass</span><code>{contract.pass}</code></div>
                      <div><span className="theme-text-muted block font-semibold">Priority</span><code>{contract.priority}</code></div>
                      <div><span className="theme-text-muted block font-semibold">Requires</span><code>{contract.requires.join(' + ')}</code></div>
                      <div><span className="theme-text-muted block font-semibold">Provides</span><code>{contract.provides.join(' + ')}</code></div>
                    </div>}
                  </div>
                </details>
              </>}
            </div>
          </section>
        </AppDialogBody>
        <AppDialogFooter className="shrink-0">
          <AppDialogButton onClick={requestClose}>Close</AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
}
