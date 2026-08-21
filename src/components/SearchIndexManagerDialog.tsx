import { useEffect, useMemo, useState } from 'react';
import { CheckCircle2, Database, FileSearch, RotateCcw, Search, TriangleAlert } from 'lucide-react';
import { translate } from '../localization/runtime';
import { safeInvoke as invoke } from '../utils/tauri';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';
import { RegistryDetailHeader } from './RegistryDetailHeader';
import { RegistryListItem } from './RegistryListItem';
import { RegistryPanelHeader } from './RegistryPanelHeader';
import { useToast } from './ToastProvider';

interface SearchIndexEntry {
  stableRef: string;
  canonicalCount: number;
  indexedCount: number;
  healthy: boolean;
  engine: string;
  includedFields: string[];
}

interface SearchIndexStatus {
  schemaVersion: number;
  indexes: SearchIndexEntry[];
}

const CAPTURED_REF = 'index:captured-clips-v1';

function indexName(stableRef: string) {
  return stableRef === CAPTURED_REF
    ? translate('component.searchIndexManagerDialog.capturedClipIndex')
    : translate('component.searchIndexManagerDialog.extractedTextIndex');
}

function indexDescription(stableRef: string) {
  return stableRef === CAPTURED_REF
    ? translate('component.searchIndexManagerDialog.capturedClipIndexDescription')
    : translate('component.searchIndexManagerDialog.extractedTextIndexDescription');
}

function fieldLabel(field: string) {
  const labels: Record<string, string> = {
    content: translate('component.searchIndexManagerDialog.clipContents'),
    name: translate('common.name'),
    note: translate('component.searchIndexManagerDialog.notes'),
    source: translate('component.searchIndexManagerDialog.sources'),
    extractedText: translate('component.searchIndexManagerDialog.extractedText'),
  };
  return labels[field] ?? field;
}

export function SearchIndexManagerDialog({ isOpen, onClose }: { isOpen: boolean; onClose: () => void }) {
  const { showToast } = useToast();
  const [status, setStatus] = useState<SearchIndexStatus | null>(null);
  const [selectedRef, setSelectedRef] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [rebuildingRef, setRebuildingRef] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen) return;
    setLoading(true);
    invoke<SearchIndexStatus>('get_search_index_status')
      .then((loaded) => {
        setStatus(loaded);
        setSelectedRef((current) => loaded.indexes.some(({ stableRef }) => stableRef === current)
          ? current
          : loaded.indexes[0]?.stableRef ?? null);
        setError(null);
      })
      .catch((reason) => setError(String(reason)))
      .finally(() => setLoading(false));
  }, [isOpen]);

  const selected = useMemo(
    () => status?.indexes.find(({ stableRef }) => stableRef === selectedRef),
    [selectedRef, status],
  );

  const rebuild = async (stableRef: string) => {
    setRebuildingRef(stableRef);
    try {
      const updated = await invoke<SearchIndexStatus>('rebuild_search_index', { stableRef });
      setStatus(updated);
      setError(null);
      showToast({ tone: 'success', message: translate('component.searchIndexManagerDialog.rebuildComplete') });
    } catch (reason) {
      setError(String(reason));
    } finally {
      setRebuildingRef(null);
    }
  };

  return (
    <AppDialog
      isOpen={isOpen}
      onClose={onClose}
      labelledBy="search-index-manager-title"
      panelClassName="theme-panel @container flex max-h-[90vh] w-full max-w-4xl flex-col overflow-hidden border shadow-2xl"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose} className="shrink-0">
          <AppDialogHeading
            id="search-index-manager-title"
            title={translate('component.searchIndexManagerDialog.searchIndex')}
            description={translate('component.searchIndexManagerDialog.description')}
            icon={<Search />}
          />
        </AppDialogHeader>
        <AppDialogBody className="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-y-auto text-xs @xl:grid-cols-[minmax(0,3fr)_minmax(0,7fr)]">
          <section className="theme-surface flex min-h-[220px] flex-col overflow-hidden rounded-xl border @xl:min-h-0">
            <RegistryPanelHeader
              title={translate('component.searchIndexManagerDialog.indexes')}
              actions={<AppDialogButton onClick={() => void rebuild('all')} disabled={loading || rebuildingRef !== null} className="h-7 min-h-7 px-2.5">
                <RotateCcw className="h-3.5 w-3.5" /> {rebuildingRef === 'all' ? translate('component.searchIndexManagerDialog.rebuilding') : translate('component.searchIndexManagerDialog.rebuildAll')}
              </AppDialogButton>}
            />
            <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
              {status?.indexes.map((index) => <RegistryListItem
                key={index.stableRef}
                selected={selectedRef === index.stableRef}
                onSelect={() => setSelectedRef(index.stableRef)}
                icon={index.stableRef === CAPTURED_REF ? <Database className="h-4 w-4" /> : <FileSearch className="h-4 w-4" />}
                title={indexName(index.stableRef)}
                subtitle={indexDescription(index.stableRef)}
                trailing={index.healthy
                  ? <CheckCircle2 className="theme-status-success-text h-4 w-4" aria-label={translate('component.searchIndexManagerDialog.healthy')} />
                  : <TriangleAlert className="theme-status-warning-text h-4 w-4" aria-label={translate('component.searchIndexManagerDialog.needsRebuild')} />}
              />)}
              {!status && !error && <p className="theme-text-muted p-3">{translate('component.searchIndexManagerDialog.loading')}</p>}
            </div>
          </section>
          <section className="theme-surface flex min-w-0 flex-col overflow-hidden rounded-xl border">
            <RegistryPanelHeader
              title={translate('component.searchIndexManagerDialog.indexDetails')}
              actions={<AppDialogButton variant="primary" onClick={() => selected && void rebuild(selected.stableRef)} disabled={!selected || rebuildingRef !== null} className="h-7 min-h-7 px-2.5">
                <RotateCcw className="h-3.5 w-3.5" /> {rebuildingRef === selected?.stableRef ? translate('component.searchIndexManagerDialog.rebuilding') : translate('component.searchIndexManagerDialog.rebuild')}
              </AppDialogButton>}
            />
            <div className="min-h-0 flex-1 space-y-4 overflow-y-auto p-4">
              {error && <div role="alert" className="theme-status-danger rounded-lg border px-3 py-2">{error}</div>}
              {selected && <>
                <RegistryDetailHeader
                  icon={selected.stableRef === CAPTURED_REF ? <Database className="h-5 w-5" /> : <FileSearch className="h-5 w-5" />}
                  title={indexName(selected.stableRef)}
                  meta={indexDescription(selected.stableRef)}
                  trailing={<span className={`rounded border px-2 py-1 text-[9px] font-semibold ${selected.healthy ? 'theme-status-success' : 'theme-status-warning'}`}>
                    {selected.healthy ? translate('component.searchIndexManagerDialog.healthy') : translate('component.searchIndexManagerDialog.needsRebuild')}
                  </span>}
                />
                <div className="grid grid-cols-2 gap-2">
                  <div className="theme-subtle-surface rounded-lg border p-3">
                    <span className="theme-text-muted block text-[9px] font-semibold">{translate('component.searchIndexManagerDialog.coverage')}</span>
                    <strong className="theme-text-main mt-1 block text-lg tabular-nums">{selected.indexedCount} / {selected.canonicalCount}</strong>
                  </div>
                  <div className="theme-subtle-surface rounded-lg border p-3">
                    <span className="theme-text-muted block text-[9px] font-semibold">{translate('component.searchIndexManagerDialog.engine')}</span>
                    <strong className="theme-text-main mt-1 block text-sm">{selected.engine}</strong>
                  </div>
                </div>
                <div className="theme-subtle-surface rounded-lg border p-3">
                  <span className="theme-text-muted block text-[9px] font-semibold">{translate('component.searchIndexManagerDialog.includedFields')}</span>
                  <p className="theme-text-main mt-1 text-[11px]">{selected.includedFields.map(fieldLabel).join(' · ')}</p>
                </div>
                <p className="theme-text-subtle leading-relaxed">{translate('component.searchIndexManagerDialog.rebuildNote')}</p>
                <details className="theme-subtle-surface rounded-lg border px-3 py-2">
                  <summary className="theme-text-main cursor-pointer text-[10px] font-semibold">{translate('common.technicalDetails')}</summary>
                  <div className="theme-divider mt-3 space-y-1 border-t pt-3">
                    <span className="theme-text-muted block text-[9px] font-semibold">{translate('common.stableReference')}</span>
                    <code className="theme-input block break-all rounded-lg border px-3 py-2">{selected.stableRef}</code>
                  </div>
                </details>
              </>}
            </div>
          </section>
        </AppDialogBody>
        <AppDialogFooter className="shrink-0">
          <AppDialogButton onClick={requestClose} disabled={rebuildingRef !== null}>{translate('common.close')}</AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
}
