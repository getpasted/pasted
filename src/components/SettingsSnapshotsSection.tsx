import { useCallback, useEffect, useRef, useState } from 'react';
import { Download, Plus, Trash2 } from 'lucide-react';
import { safeInvoke as invoke } from '../utils/tauri';
import { APP_EVENTS } from '../utils/appEvents';
import { useAppEvent } from '../hooks/useAppEvent';
import { useLocalization } from '../localization/LocalizationProvider';
import type { AppSettings } from '../types';
import { ActionButton } from './AppDialogLayout';
import { SettingsNumberStepper } from './SettingsNumberStepper';
import { SettingsSubsectionHeader } from './SettingsSubsectionHeader';
import { SettingsLoadingState } from './SettingsLoadingState';
import { ConfirmationDialog } from './ConfirmationDialog';
import { LibraryTransitionDialog } from './LibraryTransitionDialog';
import { restoreSnapshotWithTransition } from './fullBackupRestore';
import { snapshotCountdownParts } from './snapshotCountdown';
import { useToast } from './ToastProvider';

interface Snapshot { id: string; createdAt: string; clipCount: number; sizeBytes: number }
interface SnapshotStatus {
  automaticCreationFailed: boolean;
  nextAutomaticSnapshotAt: string | null;
  waitingForNewClips: boolean;
}

export function SettingsSnapshotsSection({
  settings,
  onUpdateSettings,
}: {
  settings: AppSettings;
  onUpdateSettings: (settings: Partial<AppSettings>) => void;
}) {
  const { t, formatDateTime, formatNumber } = useLocalization();
  const { showToast } = useToast();
  const [snapshots, setSnapshots] = useState<Snapshot[] | null>(null);
  const [loadFailed, setLoadFailed] = useState(false);
  const [deleteFailed, setDeleteFailed] = useState(false);
  const [restoreFailed, setRestoreFailed] = useState(false);
  const [retentionFailed, setRetentionFailed] = useState(false);
  const [automaticCreationFailed, setAutomaticCreationFailed] = useState(false);
  const [nextAutomaticSnapshotAt, setNextAutomaticSnapshotAt] = useState<string | null>(null);
  const [waitingForNewClips, setWaitingForNewClips] = useState(false);
  const [nowMs, setNowMs] = useState(Date.now());
  const [creating, setCreating] = useState(false);
  const [createFailed, setCreateFailed] = useState(false);
  const [exportingId, setExportingId] = useState<string | null>(null);
  const [selected, setSelected] = useState<Snapshot | null>(null);
  const [deleting, setDeleting] = useState<Snapshot | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [restoring, setRestoring] = useState(false);
  const refreshVersion = useRef(0);
  const refresh = useCallback(async () => {
    const version = ++refreshVersion.current;
    void invoke<SnapshotStatus>('get_snapshot_status')
      .then(status => {
        if (version === refreshVersion.current) {
          setAutomaticCreationFailed(status.automaticCreationFailed);
          setNextAutomaticSnapshotAt(status.nextAutomaticSnapshotAt);
          setWaitingForNewClips(status.waitingForNewClips);
        }
      })
      .catch(() => { /* Snapshot metadata remains available without runtime status. */ });
    try {
      const values = await invoke<Snapshot[]>('list_snapshots');
      if (version === refreshVersion.current) {
        setSnapshots(values);
        setLoadFailed(false);
      }
    } catch {
      if (version === refreshVersion.current) setLoadFailed(true);
    }
  }, []);
  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => { void refresh(); }, 30000);
    return () => { refreshVersion.current += 1; window.clearInterval(timer); };
  }, [refresh, settings.snapshotIntervalMinutes]);
  useAppEvent(APP_EVENTS.snapshotsChanged, () => { void refresh(); });
  useAppEvent(APP_EVENTS.clipAdded, () => { void refresh(); });
  useEffect(() => {
    const timer = window.setInterval(() => setNowMs(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, []);
  useEffect(() => {
    let cancelled = false;
    void invoke('enforce_snapshot_retention', { keepCount: settings.snapshotKeepCount })
      .then(() => {
        if (!cancelled) {
          setRetentionFailed(false);
          void refresh();
        }
      })
      .catch(() => { if (!cancelled) setRetentionFailed(true); });
    return () => { cancelled = true; };
  }, [refresh, settings.snapshotKeepCount]);
  const restore = async () => {
    if (!selected) return;
    const id = selected.id;
    setSelected(null);
    setRestoring(true);
    setRestoreFailed(false);
    try {
      const result = await restoreSnapshotWithTransition(id);
      if (result !== 'restarting') setRestoring(false);
    } catch { setRestoring(false); setRestoreFailed(true); }
  };
  const deleteSnapshot = async () => {
    if (!deleting || deletingId) return;
    const id = deleting.id;
    refreshVersion.current += 1;
    setDeletingId(id);
    setDeleteFailed(false);
    try {
      await invoke('delete_snapshot', { id });
      setSnapshots(values => values?.filter(snapshot => snapshot.id !== id) ?? []);
      setDeleting(null);
      void refresh();
    } catch { setDeleteFailed(true); setDeleting(null); }
    finally { setDeletingId(null); }
  };
  const createSnapshot = async () => {
    if (creating) return;
    setCreating(true);
    setCreateFailed(false);
    try {
      const snapshot = await invoke<Snapshot>('create_snapshot');
      setSnapshots(values => [snapshot, ...(values ?? []).filter(value => value.id !== snapshot.id)]);
      void refresh();
    } catch { setCreateFailed(true); }
    finally { setCreating(false); }
  };
  const exportSnapshot = async (snapshot: Snapshot) => {
    if (exportingId) return;
    setExportingId(snapshot.id);
    try {
      const report = await invoke<{ path: string } | null>('export_snapshot', { id: snapshot.id });
      if (report) showToast({ tone: 'success', message: t('snapshots.exported') });
    } catch {
      showToast({ tone: 'error', message: t('snapshots.exportFailed') });
    } finally { setExportingId(null); }
  };
  const countdown = snapshotCountdownParts(nextAutomaticSnapshotAt, nowMs);
  const countdownText = settings.snapshotKeepCount === 0
    ? t('snapshots.automaticPaused')
    : waitingForNewClips
      ? t('snapshots.waitingForNewClips')
      : countdown
        ? t('snapshots.nextIn', {
          time: countdown.hours > 0
            ? `${formatNumber(countdown.hours, { minimumIntegerDigits: 2 })}:${formatNumber(countdown.minutes, { minimumIntegerDigits: 2 })}:${formatNumber(countdown.seconds, { minimumIntegerDigits: 2 })}`
            : `${formatNumber(countdown.minutes, { minimumIntegerDigits: 2 })}:${formatNumber(countdown.seconds, { minimumIntegerDigits: 2 })}`,
        })
        : t('snapshots.scheduleUnavailable');
  return <section className="space-y-3 border-t theme-divider pt-5" aria-labelledby="snapshots-title">
    <SettingsSubsectionHeader id="snapshots-title" title={t('snapshots.title')} description={t('snapshots.description')} />
    <div className="theme-surface space-y-3 rounded-xl border p-3">
      <div className="theme-surface space-y-4 rounded-lg border p-3">
        <SettingsNumberStepper label={t('snapshots.interval')} description={t('snapshots.intervalDescription')}
          value={settings.snapshotIntervalMinutes} min={1} max={10080} disabled={restoring || creating}
          onChange={value => onUpdateSettings({ snapshotIntervalMinutes: value })} />
        <SettingsNumberStepper label={t('snapshots.keep')} description={t('snapshots.optionsNote')}
          value={settings.snapshotKeepCount} min={0} max={10000} disabled={restoring || creating}
          onChange={value => onUpdateSettings({ snapshotKeepCount: value })} />
      </div>
      {loadFailed && <p className="theme-status-danger-text text-xs" role="alert">{t('snapshots.loadFailed')}</p>}
      {deleteFailed && <p className="theme-status-danger-text text-xs" role="alert">{t('snapshots.deleteFailed')}</p>}
      {restoreFailed && <p className="theme-status-danger-text text-xs" role="alert">{t('snapshots.restoreFailed')}</p>}
      {retentionFailed && <p className="theme-status-danger-text text-xs" role="alert">{t('snapshots.retentionFailed')}</p>}
      {createFailed && <p className="theme-status-danger-text text-xs" role="alert">{t('snapshots.createFailed')}</p>}
      {automaticCreationFailed && <p className="theme-status-danger-text text-xs" role="alert">{t('snapshots.automaticFailed')}</p>}
      <div className="theme-surface overflow-hidden rounded-lg border">
        <div className="theme-subtle-surface flex items-center justify-between gap-3 border-b theme-divider px-3 py-2.5">
          <div className="min-w-0">
            <p className="theme-text-main text-[11px] font-semibold">{t('snapshots.saved')}</p>
            <p className="theme-text-muted mt-0.5 text-[10px] leading-tight">{countdownText}</p>
          </div>
          <ActionButton onClick={() => void createSnapshot()} disabled={creating || restoring || exportingId !== null} className="shrink-0 disabled:opacity-45">
            <Plus className="h-4 w-4" aria-hidden="true" />
            <span>{creating ? t('snapshots.creating') : t('snapshots.create')}</span>
          </ActionButton>
        </div>
        {snapshots === null ? <SettingsLoadingState label={t('snapshots.loading')} className="min-h-20" />
          : snapshots.length === 0 ? <p className="theme-text-muted p-3 text-xs">{t('snapshots.empty')}</p> :
        <ul className="theme-divide max-h-64 divide-y overflow-auto">
          {snapshots.map(snapshot => <li key={snapshot.id} className="flex items-center justify-between gap-3 p-3">
            <div className="min-w-0">
              <p className="theme-text-main text-xs">{formatDateTime(snapshot.createdAt, { dateStyle: 'medium', timeStyle: 'short' })}</p>
              <p className="theme-text-muted mt-1 text-xs">{t('snapshots.details', { count: formatNumber(snapshot.clipCount), size: formatNumber(snapshot.sizeBytes / 1024 / 1024, { maximumFractionDigits: 1 }) })}</p>
            </div>
            <div className="flex shrink-0 items-center gap-2">
              <ActionButton disabled={restoring || creating || deletingId === snapshot.id || exportingId !== null} onClick={() => setSelected(snapshot)}>{t('snapshots.restore')}</ActionButton>
              {settings.enableBackups && <ActionButton disabled={restoring || creating || deletingId === snapshot.id || exportingId !== null}
                onClick={() => void exportSnapshot(snapshot)} className="h-8 min-h-8 w-8 justify-center p-0 disabled:opacity-45"
                title={t('snapshots.export')} aria-label={t('snapshots.exportLabel', { date: formatDateTime(snapshot.createdAt, { dateStyle: 'medium', timeStyle: 'short' }) })}>
                <Download className="h-4 w-4 shrink-0" aria-hidden="true" />
              </ActionButton>}
              <ActionButton disabled={restoring || creating || deletingId === snapshot.id || exportingId !== null} onClick={() => setDeleting(snapshot)}
                className="h-8 min-h-8 w-8 justify-center p-0 disabled:opacity-45"
                title={t('snapshots.delete')} aria-label={t('snapshots.deleteLabel', { date: formatDateTime(snapshot.createdAt, { dateStyle: 'medium', timeStyle: 'short' }) })}>
                <Trash2 className="h-4 w-4 shrink-0" aria-hidden="true" />
              </ActionButton>
            </div>
          </li>)}
        </ul>}
      </div>
    </div>
    <ConfirmationDialog onCancel={() => setSelected(null)} request={selected ? {
      title: t('snapshots.restoreTitle'),
      description: t('snapshots.restoreDescription', { date: formatDateTime(selected.createdAt, { dateStyle: 'medium', timeStyle: 'short' }) }),
      confirmLabel: t('snapshots.restoreConfirm'), onConfirm: restore,
    } : null} />
    <ConfirmationDialog onCancel={() => { if (!deletingId) setDeleting(null); }} request={deleting ? {
      title: t('snapshots.deleteTitle'), tone: 'danger',
      description: t('snapshots.deleteDescription', { date: formatDateTime(deleting.createdAt, { dateStyle: 'medium', timeStyle: 'short' }) }),
      confirmLabel: t('common.delete'), confirmDisabled: deletingId !== null, onConfirm: deleteSnapshot,
    } : null} />
    <LibraryTransitionDialog isOpen={restoring} variant="import" title={t('snapshots.restoring')} description={t('snapshots.restoringDescription')} />
  </section>;
}
