import { useEffect, useState } from 'react';
import { ScanText, Square } from 'lucide-react';
import type { OcrBackfillStatus } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { analysisApi } from '../api/analysis';
import { ActionButton } from './AppDialogLayout';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
import { useToast } from './ToastProvider';
import type { ContentExtractor } from './contentExtractorModel';
import { translate } from '../localization/runtime';
import { actionableOcrCount, OCR_STATUS_CARD_KEYS, shouldRetryFailedOcr } from './ocrStatusModel';

const EMPTY_OCR_STATUS: OcrBackfillStatus = {
  totalImages: 0,
  eligibleCount: 0,
  queuedCount: 0,
  runningCount: 0,
  completedCount: 0,
  noTextCount: 0,
  failedCount: 0,
};

export function SettingsOcrPanel({
  extractorRevision,
  searchEnabled,
  onSearchClips,
}: {
  extractorRevision: number;
  searchEnabled: boolean;
  onSearchClips: (clipIds: number[]) => void;
}) {
  const { showToast } = useToast();
  const [status, setStatus] = useState<OcrBackfillStatus>(EMPTY_OCR_STATUS);
  const [extractors, setExtractors] = useState<ContentExtractor[]>([]);

  const refresh = async () => {
    try {
      setStatus(await invoke<OcrBackfillStatus>('get_ocr_backfill_status'));
      setExtractors(await analysisApi.listExtractors<ContentExtractor>());
    } catch (error) {
      showToast({ tone: 'error', message: translate('component.settingsOcrPanel.ocrStatusCouldNotBeLoadedValue', { value: String(error) }) });
    }
  };

  useEffect(() => {
    let cancelled = false;
    let polling = false;
    const poll = async () => {
      if (polling) return;
      polling = true;
      try {
        const next = await invoke<OcrBackfillStatus>('get_ocr_backfill_status');
        if (!cancelled) {
          setStatus(next);
        }
      } catch {
        // The next user action will surface a concrete error.
      } finally {
        polling = false;
      }
    };
    void poll();
    void analysisApi.listExtractors<ContentExtractor>()
      .then((loaded) => {
        if (!cancelled) setExtractors(loaded);
      })
      .catch(() => { /* The next user action will surface a concrete error. */ });
    const timer = window.setInterval(() => void poll(), 1000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [extractorRevision]);

  const run = async (operation: () => Promise<unknown>) => {
    try {
      await operation();
      await refresh();
    } catch (error) {
      showToast({ tone: 'error', message: translate('component.settingsOcrPanel.ocrOperationFailedValue', { value: String(error) }) });
    }
  };

  const showStatusClips = async (group: string) => {
    try {
      const clipIds = await invoke<number[]>('get_ocr_backfill_clip_ids', { group });
      if (clipIds.length > 0) onSearchClips(clipIds);
    } catch (error) {
      showToast({ tone: 'error', message: translate('component.settingsOcrPanel.ocrStatusCouldNotBeLoadedValue', { value: String(error) }) });
    }
  };

  const busy = status.runningCount > 0 || status.queuedCount > 0;
  const actionableCount = actionableOcrCount(status);
  const scan = () => shouldRetryFailedOcr(status)
    ? invoke('retry_failed_ocr')
    : invoke('start_ocr_backfill');
  const activeExtractor = extractors.find((extractor) => (
    extractor.enabled && extractor.isAvailable && extractor.recipe.accepts.includes('image')
  ));
  const statusText = !activeExtractor
    ? translate('component.settingsOcrPanel.noAvailableImageTextExtractorIsEnabled')
    : actionableCount > 0
    ? translate('component.settingsOcrPanel.eligibleImages', { count: actionableCount })
    : translate('component.settingsOcrPanel.allEligibleImagesHaveBeenScanned');

  return (
    <section className="theme-surface overflow-hidden rounded-2xl border" aria-labelledby="ocr-maintenance-title">
      <div className="space-y-4 p-5">
        <SettingsSubsectionHeader
          id="ocr-maintenance-title"
          icon={<ScanText className="h-4 w-4" />}
          title={translate('component.settingsOcrPanel.ocr')}
          description={translate('component.settingsOcrPanel.automaticallyMakesTextInImagesSearchable')}
        />
        <div className="grid grid-cols-3 gap-2 sm:grid-cols-6">
          {OCR_STATUS_CARD_KEYS.map(([label, field]) => (
            <button
              key={label}
              type="button"
              disabled={!searchEnabled || status[field] === 0}
              onClick={() => void showStatusClips(label)}
              className="ocr-status-card theme-card-idle theme-focusable cursor-pointer border px-2 py-2 text-center disabled:cursor-default"
            >
              <strong className="theme-title block text-sm tabular-nums">{status[field]}</strong>
              <span className="theme-text-muted text-[9px]">{translate(`component.settingsOcrPanel.status.${label}`)}</span>
            </button>
          ))}
        </div>
      </div>
      <div className="theme-divider flex items-center justify-between gap-3 border-t px-5 py-3">
        <p className="theme-text-muted text-[11px] leading-relaxed">{statusText}</p>
        {busy ? (
          <ActionButton onClick={() => void run(() => invoke('cancel_ocr_backfill'))}>
            <Square className="h-3.5 w-3.5" /> {translate('common.cancel')}
          </ActionButton>
        ) : (
          <ActionButton variant="primary" disabled={actionableCount === 0 || !activeExtractor} onClick={() => void run(scan)}>
            <ScanText className="h-3.5 w-3.5" /> {translate('component.settingsOcrPanel.scan')}
          </ActionButton>
        )}
      </div>
    </section>
  );
}
