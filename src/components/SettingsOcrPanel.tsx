import { useEffect, useState } from 'react';
import { ScanText, Square } from 'lucide-react';
import type { OcrBackfillStatus } from '../types';
import { safeInvoke as invoke } from '../utils/tauri';
import { ActionButton } from './AppDialogLayout';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
import { useToast } from './ToastProvider';

const EMPTY_OCR_STATUS: OcrBackfillStatus = {
  totalImages: 0,
  eligibleCount: 0,
  queuedCount: 0,
  runningCount: 0,
  completedCount: 0,
  noTextCount: 0,
  failedCount: 0,
};

export function SettingsOcrPanel() {
  const { showToast } = useToast();
  const [status, setStatus] = useState<OcrBackfillStatus>(EMPTY_OCR_STATUS);

  const refresh = async () => {
    try {
      setStatus(await invoke<OcrBackfillStatus>('get_ocr_backfill_status'));
    } catch (error) {
      showToast({ tone: 'error', message: `OCR status could not be loaded: ${String(error)}` });
    }
  };

  useEffect(() => {
    let cancelled = false;
    const poll = () => {
      invoke<OcrBackfillStatus>('get_ocr_backfill_status')
        .then((next) => { if (!cancelled) setStatus(next); })
        .catch(() => { /* The next user action will surface a concrete error. */ });
    };
    poll();
    const timer = window.setInterval(poll, 1000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  const run = async (operation: () => Promise<unknown>) => {
    try {
      await operation();
      await refresh();
    } catch (error) {
      showToast({ tone: 'error', message: `OCR operation failed: ${String(error)}` });
    }
  };

  const busy = status.runningCount > 0 || status.queuedCount > 0;

  return (
    <section className="theme-surface space-y-4 rounded-2xl border p-5" aria-labelledby="ocr-maintenance-title">
      <SettingsSubsectionHeader
        id="ocr-maintenance-title"
        icon={<ScanText className="h-4 w-4" />}
        title="OCR"
        description={status.eligibleCount > 0
          ? `${status.eligibleCount} image${status.eligibleCount === 1 ? '' : 's'} can be scanned for searchable text.`
          : 'All eligible images have been scanned.'}
        actions={busy ? (
          <ActionButton onClick={() => void run(() => invoke('cancel_ocr_backfill'))}>
            <Square className="h-3.5 w-3.5" /> Cancel
          </ActionButton>
        ) : (
          <ActionButton variant="primary" disabled={status.eligibleCount === 0} onClick={() => void run(() => invoke('start_ocr_backfill'))}>
            <ScanText className="h-3.5 w-3.5" /> Scan Existing
          </ActionButton>
        )}
      />
      <div className="grid grid-cols-3 gap-2 sm:grid-cols-6">
        {[
          ['Images', status.totalImages],
          ['Waiting', status.eligibleCount],
          ['Running', status.runningCount],
          ['Complete', status.completedCount],
          ['No text', status.noTextCount],
          ['Failed', status.failedCount],
        ].map(([label, value]) => (
          <div key={label} className="theme-card-idle border px-2 py-2 text-center">
            <strong className="theme-title block text-sm tabular-nums">{value}</strong>
            <span className="theme-text-muted text-[9px]">{label}</span>
          </div>
        ))}
      </div>
      {!busy && status.failedCount > 0 && (
        <ActionButton onClick={() => void run(() => invoke('retry_failed_ocr'))} className="w-full">
          Retry {status.failedCount} Failed Scan{status.failedCount === 1 ? '' : 's'}
        </ActionButton>
      )}
    </section>
  );
}
