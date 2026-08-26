import { useEffect, useMemo, useState, type ComponentType } from 'react';
import { AppWindow, FileAudio, FileType2, Lightbulb, ScanSearch, Shapes } from 'lucide-react';
import type { LibraryItemView } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { analysisApi } from '../api/analysis';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';
import { RegistryDetailHeader } from './RegistryDetailHeader';
import { RegistryListItem } from './RegistryListItem';
import { RegistryPanelHeader } from './RegistryPanelHeader';
import { translate } from '../localization/runtime';
import { localizedBuiltinDescription, localizedBuiltinName } from '../localization/presentation';

type BuiltinLifecycleKind = Extract<LibraryItemView['kind'], 'capture' | 'inspector' | 'suggestion'>;

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

export function BuiltinLifecycleManagerDialog({
  isOpen,
  onClose,
  kind,
  title,
  description,
  icon: HeadingIcon,
  sourcesEnabled = true,
  fileFormatsEnabled = true,
}: {
  isOpen: boolean;
  onClose: () => void;
  kind: BuiltinLifecycleKind;
  title: string;
  description: string;
  icon: ComponentType<{ className?: string }>;
  sourcesEnabled?: boolean;
  fileFormatsEnabled?: boolean;
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
            ? analysisApi.listInspectors<InspectorDefinition>()
            : Promise.resolve([]),
        ]);
        let visibleItems = kind === 'capture' && !sourcesEnabled
          ? loaded.filter(({ stableRef }) => stableRef !== 'capture:source-attribution-v1')
          : loaded;
        if (kind === 'inspector' && !fileFormatsEnabled) {
          visibleItems = visibleItems.filter(
            ({ stableRef }) => stableRef !== 'inspector:file-format-v1',
          );
        }
        setItems(visibleItems);
        setInspectors(loadedInspectors);
        setSelectedRef((current) => visibleItems.some(({ stableRef }) => stableRef === current) ? current : visibleItems[0]?.stableRef ?? null);
        setError(null);
      } catch (reason) {
        setError(String(reason));
      }
    };
    void load();
  }, [fileFormatsEnabled, isOpen, kind, sourcesEnabled]);

  const selected = useMemo(
    () => items.find(({ stableRef }) => stableRef === selectedRef),
    [items, selectedRef],
  );
  const contract = selected?.participantContract;
  const runtime = inspectors.find(({ stableRef }) => stableRef === selectedRef);
  const isClipType = kind === 'capture' && selected?.stableRef === 'capture:clip-type-v1';
  const worksWith = isClipType
    ? translate('component.builtinLifecycleManagerDialog.clipboardRepresentations')
    : kind === 'capture'
    ? translate('component.builtinLifecycleManagerDialog.clipboardCaptures')
    : kind === 'suggestion'
    ? translate('component.builtinLifecycleManagerDialog.clipsWithAnalyzableText')
    : selected?.typeRelations?.some(({ kind: relationKind, typeId }) => relationKind === 'accepts' && typeId === 'file')
      ? translate('component.builtinLifecycleManagerDialog.fileClips')
      : translate('component.builtinLifecycleManagerDialog.allClips');
  const provides = isClipType
    ? translate('component.builtinLifecycleManagerDialog.textImageOrFiles')
    : kind === 'capture'
    ? translate('component.builtinLifecycleManagerDialog.appNameAndIcon')
    : kind === 'suggestion'
    ? translate('component.builtinLifecycleManagerDialog.smartActionSuggestions')
    : selected?.stableRef.includes('media')
      ? translate('component.builtinLifecycleManagerDialog.mediaMetadata')
      : selected?.stableRef === 'inspector:file-format-v1'
        ? translate('component.builtinLifecycleManagerDialog.fileFormats')
      : translate('component.builtinLifecycleManagerDialog.structuralDetails');
  const availabilityText = runtime?.engine
    ? runtime.isAvailable
      ? translate('component.builtinLifecycleManagerDialog.engineAvailable', { engine: engineLabel(runtime.engine) })
      : runtime.unavailableReason?.match(/^.*?\.(?:\s|$)/)?.[0].trim() ?? translate('component.builtinLifecycleManagerDialog.engineUnavailable')
    : null;
  const DetailIcon = isClipType
    ? Shapes
    : kind === 'capture'
    ? AppWindow
    : kind === 'inspector' && selected?.stableRef.includes('media')
    ? FileAudio
    : kind === 'inspector' && selected?.stableRef === 'inspector:file-format-v1'
    ? FileType2
    : kind === 'suggestion' ? Lightbulb : ScanSearch;

  return (
    <AppDialog
      isOpen={isOpen}
      onClose={onClose}
      labelledBy={`${kind}-manager-title`}
      panelClassName="theme-panel @container flex max-h-[90vh] w-full max-w-4xl flex-col overflow-hidden border"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose} className="shrink-0">
          <AppDialogHeading id={`${kind}-manager-title`} title={title} description={description} icon={<HeadingIcon />} />
        </AppDialogHeader>
        <AppDialogBody className="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-y-auto text-xs @xl:grid-cols-[minmax(0,3fr)_minmax(0,7fr)]">
          <section className="theme-surface flex min-h-[220px] flex-col overflow-hidden rounded-xl border @xl:min-h-0">
            <RegistryPanelHeader title={title} />
            <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
              {items.map((item) => {
                const displayName = localizedBuiltinName('library', item.stableRef, item.name, item.isBuiltin);
                const displayDescription = localizedBuiltinDescription('library', item.stableRef, item.description, item.isBuiltin);
                const ItemIcon = item.stableRef === 'capture:clip-type-v1'
                  ? Shapes
                  : item.stableRef === 'capture:source-attribution-v1'
                  ? AppWindow
                  : HeadingIcon;
                return <RegistryListItem
                  key={item.stableRef}
                  selected={selectedRef === item.stableRef}
                  onSelect={() => setSelectedRef(item.stableRef)}
                  icon={<ItemIcon className="h-4 w-4" />}
                  title={displayName}
                  subtitle={displayDescription}
                />;
              })}
            </div>
          </section>
          <section className="theme-surface flex min-w-0 flex-col overflow-hidden rounded-xl border">
            <RegistryPanelHeader title={kind === 'capture'
              ? translate('component.builtinLifecycleManagerDialog.captureDetails')
              : kind === 'inspector'
                ? translate('component.builtinLifecycleManagerDialog.inspectorDetails')
                : translate('component.builtinLifecycleManagerDialog.suggestionDetails')} />
            <div className="min-h-0 flex-1 space-y-4 overflow-y-auto p-4">
              {error && <div role="alert" className="theme-status-danger rounded-lg border px-3 py-2">{translate('component.builtinLifecycleManagerDialog.definitionsCouldNotBeLoaded')}</div>}
              {selected && <>
                {(() => {
                  const displayName = localizedBuiltinName('library', selected.stableRef, selected.name, selected.isBuiltin);
                  const displayDescription = localizedBuiltinDescription('library', selected.stableRef, selected.description, selected.isBuiltin);
                  return <>
                <RegistryDetailHeader
                  icon={<DetailIcon className="h-5 w-5" />}
                  title={displayName}
                  meta={displayDescription}
                  trailing={availabilityText ? <span
                    className={`max-w-56 rounded border px-2 py-1 text-end text-[9px] font-semibold ${runtime?.isAvailable ? 'theme-status-success' : 'theme-status-warning'}`}
                    title={runtime?.unavailableReason ?? translate('component.builtinLifecycleManagerDialog.theValueEngineIsReady', { value: engineLabel(runtime?.engine ?? '') })}
                  >{availabilityText}</span> : undefined}
                />
                  </>;
                })()}
                <div className="grid grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-stretch gap-2">
                  <div className="theme-subtle-surface rounded-lg border p-3">
                    <span className="theme-text-muted block text-[9px] font-semibold">{translate('component.builtinLifecycleManagerDialog.worksWith')}</span>
                    <strong className="theme-text-main mt-1 block text-[11px]">{worksWith}</strong>
                  </div>
                  <span className="theme-text-muted self-center" aria-hidden="true">→</span>
                  <div className="theme-subtle-surface rounded-lg border p-3">
                    <span className="theme-text-muted block text-[9px] font-semibold">{translate('component.builtinLifecycleManagerDialog.provides')}</span>
                    <strong className="theme-text-main mt-1 block text-[11px]">{provides}</strong>
                  </div>
                </div>
                <details className="theme-subtle-surface rounded-lg border px-3 py-2">
                  <summary className="theme-text-main cursor-pointer text-[10px] font-semibold">{translate('common.technicalDetails')}</summary>
                  <div className="theme-divider mt-3 space-y-3 border-t pt-3 text-[10px]">
                    <p className="theme-text-subtle leading-relaxed">
                      {kind === 'capture'
                        ? translate('component.builtinLifecycleManagerDialog.captureStableReferenceUsage', { command: 'pasted registry list --kind capture --json' })
                        : translate('component.builtinLifecycleManagerDialog.stableReferenceUsage', { kind, command: `pasted ${kind} get <ref> --json` })}
                    </p>
                    <div className="space-y-1">
                      <span className="theme-text-muted block font-semibold">{translate('common.stableReference')}</span>
                      <code className="theme-input block break-all rounded-lg border px-3 py-2">{selected.stableRef}</code>
                    </div>
                    {contract && <div className="grid grid-cols-1 gap-2 @md:grid-cols-2">
                      <div><span className="theme-text-muted block font-semibold">{translate('component.builtinLifecycleManagerDialog.analysisPass')}</span><code>{contract.pass}</code></div>
                      <div><span className="theme-text-muted block font-semibold">{translate('common.priority')}</span><code>{contract.priority}</code></div>
                      <div><span className="theme-text-muted block font-semibold">{translate('component.builtinLifecycleManagerDialog.requires')}</span><code>{contract.requires.join(' + ')}</code></div>
                      <div><span className="theme-text-muted block font-semibold">{translate('component.builtinLifecycleManagerDialog.provides')}</span><code>{contract.provides.join(' + ')}</code></div>
                    </div>}
                    {isClipType && <p className="theme-text-subtle leading-relaxed">
                      {translate('component.builtinLifecycleManagerDialog.everyClipHasOneStructuralClipTypeCopiesContainingMultipleFilesRemain')}
                    </p>}
                    {kind === 'capture' && !isClipType && <p className="theme-text-subtle leading-relaxed">{translate('component.builtinLifecycleManagerDialog.applicationNamesAreRecordedDuringCaptureIconsAreResolvedOnlyWhenDisplayed')}</p>}
                  </div>
                </details>
              </>}
            </div>
          </section>
        </AppDialogBody>
        <AppDialogFooter className="shrink-0">
          <AppDialogButton onClick={requestClose}>{translate('common.close')}</AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
}
