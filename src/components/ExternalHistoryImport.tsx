import { useEffect, useState } from 'react';
import { CheckCircle2, DatabaseZap, FileUp, FolderOpen, LoaderCircle } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { useToast } from './ToastProvider';
import { ActionButton } from './AppDialogLayout';
import { SettingsAccentTile } from './SettingsAccentTile';

export interface ExternalImportSourceInfo {
  id: string;
  label: string;
  description: string;
  available: boolean;
  detected: boolean;
  defaultPath: string | null;
  supportsCustomFile: boolean;
  selectionKind: 'file' | 'folder';
}

export interface ExternalImportReport {
  source: string;
  scannedCount: number;
  importedCount: number;
  duplicateCount: number;
  skippedCount: number;
  historyCapacityAdjustedTo?: number | null;
}

interface ExternalHistoryImportProps {
  compact?: boolean;
  onImported?: (report: ExternalImportReport) => void | Promise<void>;
}

function reportSummary(report: ExternalImportReport) {
  const skipped = report.duplicateCount + report.skippedCount;
  return `${report.importedCount} imported${skipped > 0 ? ` · ${skipped} skipped` : ''}${
    report.historyCapacityAdjustedTo ? ' · capacity expanded' : ''
  }`;
}

export function ExternalHistoryImport({ compact = false, onImported }: ExternalHistoryImportProps) {
  const { showToast } = useToast();
  const [sources, setSources] = useState<ExternalImportSourceInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [importingSource, setImportingSource] = useState<string | null>(null);
  const [reports, setReports] = useState<Record<string, ExternalImportReport>>({});

  useEffect(() => {
    let cancelled = false;
    invoke<ExternalImportSourceInfo[]>('get_external_import_sources')
      .then((result) => {
        if (!cancelled) setSources(result);
      })
      .catch((error) => {
        console.error('Could not detect import sources:', error);
        if (!cancelled) showToast({ tone: 'error', message: 'Could not check for existing clipboard history.' });
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [showToast]);

  const runImport = async (source: ExternalImportSourceInfo, chooseFile: boolean) => {
    setImportingSource(source.id);
    try {
      const report = await invoke<ExternalImportReport | null>('import_external_history', {
        source: source.id,
        chooseFile,
      });
      if (!report) return;
      setReports((current) => ({ ...current, [source.id]: report }));
      await onImported?.(report);
      showToast({
        tone: 'success',
        message: `${source.label}: ${reportSummary(report)}.`,
      });
    } catch (error) {
      console.error(`${source.label} import failed:`, error);
      showToast({ tone: 'error', message: String(error), durationMs: 8000 });
    } finally {
      setImportingSource(null);
    }
  };

  if (loading) {
    return (
      <div className="theme-text-muted flex items-center gap-2 py-3 text-[11px]" role="status">
        <LoaderCircle className="h-3.5 w-3.5 animate-spin" />
        Looking for clipboard history on this computer…
      </div>
    );
  }

  const detectedCount = sources.filter((source) => source.detected).length;

  return (
    <div className={compact ? 'space-y-2' : 'space-y-3'}>
      {!compact && (
        <p className="theme-text-muted text-[11px] leading-relaxed">
          Pasted reads the source file without modifying it, imports supported text history, and skips duplicates.
        </p>
      )}
      {detectedCount === 0 && (
        <div className="theme-subtle-surface rounded-xl border px-3 py-2.5">
          <p className="theme-text-main text-xs font-semibold">No supported history was detected automatically.</p>
          <p className="theme-text-muted mt-0.5 text-[11px]">Choose a source file below if it lives somewhere else.</p>
        </div>
      )}
      <div className="theme-surface overflow-hidden rounded-xl border">
        {sources.map((source, index) => {
          const isImporting = importingSource === source.id;
          const report = reports[source.id];
          return (
            <div
              key={source.id}
              className={`flex items-center gap-3 px-3 ${compact ? 'py-2' : 'py-3'} ${index > 0 ? 'theme-divider border-t' : ''}`}
            >
              <SettingsAccentTile>
                {report ? <CheckCircle2 className="h-4 w-4 theme-status-success-text" /> : <DatabaseZap className="h-4 w-4" />}
              </SettingsAccentTile>
              <span className="min-w-0 flex-1">
                <span className="theme-title flex items-center gap-2 text-xs font-bold">
                  {source.label}
                  {source.detected && !report && (
                    <span className="theme-badge rounded border px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wider">Detected</span>
                  )}
                </span>
                <span className="theme-text-muted mt-0.5 block truncate text-[10px]" title={source.defaultPath ?? undefined}>
                  {report
                    ? reportSummary(report)
                    : source.detected && !source.available
                      ? `${source.description} · Choose its data folder to allow access.`
                      : source.description}
                </span>
              </span>
              <span className="flex shrink-0 items-center gap-1.5">
                {source.available && (
                  <ActionButton
                    variant="primary"
                    disabled={importingSource !== null}
                    onClick={() => void runImport(source, false)}
                    className="h-8 min-h-8 w-8 justify-center p-0 disabled:opacity-45"
                    aria-label={`Import ${source.label} history`}
                    title={`Import detected ${source.label} history`}
                  >
                    {isImporting ? <LoaderCircle className="h-4 w-4 shrink-0 animate-spin" /> : <DatabaseZap className="h-4 w-4 shrink-0" />}
                  </ActionButton>
                )}
                <ActionButton
                  disabled={importingSource !== null}
                  onClick={() => void runImport(source, true)}
                  className="h-8 min-h-8 w-8 justify-center p-0 disabled:opacity-45"
                  aria-label={`Choose a ${source.label} history ${source.selectionKind}`}
                  title={`Choose a ${source.label} history ${source.selectionKind}`}
                >
                  {source.selectionKind === 'folder'
                    ? <FolderOpen className="h-4 w-4 shrink-0" />
                    : <FileUp className="h-4 w-4 shrink-0" />}
                </ActionButton>
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
