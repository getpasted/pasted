import React, { useState, useEffect, useCallback, useRef } from 'react';
import { safeInvoke as invoke } from '../utils/tauri';
import {
  Activity,
  Trash2,
  RotateCcw,
  ShieldAlert,
  ShieldCheck,
  ShieldOff,
  Edit3,
  Trash,
  Search,
  Pause,
  Play,
  Workflow,
  History,
  ListFilter,
  FolderMinus,
  FolderInput,
  FileWarning,
  ListOrdered,
  ClipboardPaste,
  LogIn,
  LogOut,
  Pin,
  Rocket,
  Settings2,
  Database,
  Radar,
  LockKeyhole,
  Keyboard,
} from 'lucide-react';
import { ToolPageHeader } from './ToolPageHeader';
import { MenuSelect } from './MenuSelect';
import { OverflowText } from './OverflowText';
import { useMinuteTick } from '../hooks/useMinuteTick';
import { dateTimeAttribute, formatFullDateTime, formatRelativeTime } from '../utils/date';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';
import { translate } from '../localization/runtime';

export interface ActivityLog {
  id: number;
  event_type: string;
  description: string;
  created_at: string;
  observed_at: string;
  severity_text: 'info' | 'warn' | 'error';
  category: string;
  outcome: 'success' | 'failure' | 'unknown';
  attributes: Record<string, unknown>;
}

const ACTIVITY_BATCH_SIZE = 200;

export const ActivityLogView: React.FC = () => {
  const [logs, setLogs] = useState<ActivityLog[]>([]);
  const [filter, setFilter] = useState('');
  const [hasMore, setHasMore] = useState(true);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  const [isClearConfirmOpen, setIsClearConfirmOpen] = useState(false);
  const logsRef = useRef<ActivityLog[]>([]);
  const loadingMoreRef = useRef(false);
  const loadMoreMarkerRef = useRef<HTMLDivElement>(null);
  const relativeTimeNow = useMinuteTick();

  const replaceLogs = useCallback((next: ActivityLog[]) => {
    logsRef.current = next;
    setLogs(next);
  }, []);

  const fetchInitialLogs = useCallback(async () => {
    try {
      const res = await invoke<ActivityLog[]>('get_activity_logs', { limit: ACTIVITY_BATCH_SIZE, offset: 0 });
      replaceLogs(res);
      setHasMore(res.length === ACTIVITY_BATCH_SIZE);
    } catch (e) {
      console.error('Failed to fetch activity logs:', e);
    }
  }, [replaceLogs]);

  const refreshNewestLogs = useCallback(async () => {
    try {
      const newest = await invoke<ActivityLog[]>('get_activity_logs', { limit: ACTIVITY_BATCH_SIZE, offset: 0 });
      const newestIds = new Set(newest.map(({ id }) => id));
      replaceLogs([...newest, ...logsRef.current.filter(({ id }) => !newestIds.has(id))]);
      if (newest.length < ACTIVITY_BATCH_SIZE) setHasMore(false);
    } catch (error) {
      console.error('Failed to refresh recent activity:', error);
    }
  }, [replaceLogs]);

  const loadMore = useCallback(async () => {
    if (!hasMore || loadingMoreRef.current) return;
    loadingMoreRef.current = true;
    setIsLoadingMore(true);
    try {
      const older = await invoke<ActivityLog[]>('get_activity_logs', {
        limit: ACTIVITY_BATCH_SIZE,
        offset: logsRef.current.length,
      });
      const knownIds = new Set(logsRef.current.map(({ id }) => id));
      replaceLogs([...logsRef.current, ...older.filter(({ id }) => !knownIds.has(id))]);
      setHasMore(older.length === ACTIVITY_BATCH_SIZE);
    } catch (error) {
      console.error('Failed to load older activity:', error);
    } finally {
      loadingMoreRef.current = false;
      setIsLoadingMore(false);
    }
  }, [hasMore, replaceLogs]);

  useEffect(() => {
    void fetchInitialLogs();

    const interval = setInterval(() => {
      void refreshNewestLogs();
    }, 5000);

    return () => {
      clearInterval(interval);
    };
  }, [fetchInitialLogs, refreshNewestLogs]);

  useEffect(() => {
    const marker = loadMoreMarkerRef.current;
    if (!marker || !hasMore) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) void loadMore();
      },
      { rootMargin: '300px 0px' },
    );
    observer.observe(marker);
    return () => observer.disconnect();
  }, [hasMore, loadMore]);

  const handleClearLogs = async () => {
    try {
      await invoke('clear_activity_logs');
      replaceLogs([]);
      setHasMore(false);
      setIsClearConfirmOpen(false);
    } catch (e) {
      console.error('Failed to clear logs:', e);
    }
  };

  const getEventBadge = (type: string, description: string) => {
    switch (type) {
      case 'recording_manually_paused':
        return (
          <div className="theme-status-warning flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Pause className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.manuallyPaused')}</span>
          </div>
        );
      case 'app_started':
        return (
          <div className="theme-status-success flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <LogIn className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.appOpened')}</span>
          </div>
        );
      case 'app_exit_requested':
        return (
          <div className="theme-badge flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <LogOut className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.appQuit')}</span>
          </div>
        );
      case 'app_lock_enabled':
      case 'app_lock_disabled':
      case 'app_lock_passphrase_changed':
      case 'app_lock_reset':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <LockKeyhole className="w-3.5 h-3.5" />
            <span>{translate('format.labelStatus', {
              label: translate('component.activityLogView.appLock'),
              status: type === 'app_lock_passphrase_changed' ? translate('component.activityLogView.passphraseChanged') : type === 'app_lock_reset' ? translate('component.activityLogView.recoveryReset') : type.endsWith('_enabled') ? translate('common.enabled') : translate('component.activityLogView.disabled'),
            })}</span>
          </div>
        );
      case 'setting_changed':
      case 'settings_changed':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Settings2 className="w-3.5 h-3.5" />
            <span>{translate('destination.settings')}</span>
          </div>
        );
      case 'content_extractor_enabled':
      case 'content_extractor_disabled':
      case 'content_classifier_enabled':
      case 'content_classifier_disabled':
      // Imported Activity retains its original versioned event names.
      case 'content_detector_enabled':
      case 'content_detector_disabled': {
        const enabled = type.endsWith('_enabled');
        const participant = type.startsWith('content_extractor')
          ? translate('component.activityLogView.extractor')
          : translate('component.activityLogView.classifier');
        return (
          <div className={`${enabled ? 'theme-status-success' : 'theme-badge'} flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold`}>
            <Radar className="w-3.5 h-3.5" />
            <span>{translate('format.labelStatus', { label: participant, status: enabled ? translate('common.enabled') : translate('component.activityLogView.disabled') })}</span>
          </div>
        );
      }
      case 'content_classifier_created':
      case 'content_classifier_updated':
      case 'content_classifier_deleted':
      case 'content_classifier_applied':
      case 'content_classifiers_restored':
      case 'content_detector_created':
      case 'content_detector_updated':
      case 'content_detector_deleted':
      case 'content_detector_applied':
      case 'content_detectors_restored':
      case 'content_extractor_updated':
      case 'content_extractor_created':
      case 'content_extractor_deleted':
      case 'content_extractors_restored':
      case 'content_classification_history_rescanned':
      case 'content_detection_history_rescanned':
      case 'file_format_history_rescanned':
      case 'content_type_created':
      case 'content_type_updated':
      case 'content_type_archived':
      case 'content_type_restored':
      case 'content_types_restored':
      case 'content_type_group_created':
      case 'content_type_group_updated':
      case 'content_type_group_archived':
      case 'content_type_group_deleted':
      case 'content_type_group_restored':
      case 'content_type_groups_restored':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Radar className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.analysis')}</span>
          </div>
        );
      case 'operation_created':
      case 'operation_updated':
      case 'operation_deleted':
      case 'library_item_enabled_changed':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Workflow className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.transforms')}</span>
          </div>
        );
      case 'autostart_enabled':
      case 'autostart_disabled':
        return (
          <div className={`${type === 'autostart_enabled' ? 'theme-status-success' : 'theme-badge'} flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold`}>
            <Rocket className="w-3.5 h-3.5" />
            <span>{type === 'autostart_enabled' ? translate('component.activityLogView.loginStartOn') : translate('component.activityLogView.loginStartOff')}</span>
          </div>
        );
      case 'recording_manually_resumed':
        return (
          <div className="theme-status-success flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Play className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.manuallyResumed')}</span>
          </div>
        );
      case 'recording_auto_paused':
        return (
          <div className="theme-status-warning flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <ShieldAlert className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.autoPaused')}</span>
          </div>
        );
      case 'recording_auto_resumed':
        return (
          <div className="theme-status-success flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <ShieldCheck className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.autoResumed')}</span>
          </div>
        );
      case 'clip_trashed':
      case 'clips_trashed':
      case 'clip_auto_trashed':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Trash2 className="w-3.5 h-3.5" />
            <span>{type === 'clip_auto_trashed' ? translate('component.activityLogView.autoTrashed') : type === 'clips_trashed' ? translate('component.activityLogView.batchTrashed') : translate('component.activityLogView.trashed')}</span>
          </div>
        );
      case 'clips_trashed_all':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Trash2 className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.trashedAll')}</span>
          </div>
        );
      case 'clip_restored':
      case 'clips_restored_all':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <RotateCcw className="w-3.5 h-3.5" />
            <span>{type === 'clips_restored_all' ? translate('component.activityLogView.restoredAll') : translate('component.activityLogView.restored')}</span>
          </div>
        );
      case 'clip_revision_restored':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <History className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.revisionRestored')}</span>
          </div>
        );
      case 'library_moved':
      case 'external_history_imported':
      case 'clips_imported':
      case 'backup_created':
      case 'backup_recovery_completed':
      case 'data_export_completed':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Database className="w-3.5 h-3.5" />
            <span>{type === 'external_history_imported' || type === 'clips_imported'
              ? type === 'clips_imported' ? translate('component.activityLogView.clipsImported') : translate('component.activityLogView.historyImported')
              : type === 'backup_created'
                ? translate('component.activityLogView.backupCreated')
                : type === 'backup_recovery_completed'
                  ? translate('component.activityLogView.backupRecovered')
                  : type === 'data_export_completed'
                    ? translate('component.activityLogView.dataExported')
                    : translate('component.activityLogView.libraryMoved')}</span>
          </div>
        );
      case 'bin_deleted':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <FolderMinus className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.binDeleted')}</span>
          </div>
        );
      case 'bin_clips_reordered':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <ListOrdered className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.binReordered')}</span>
          </div>
        );
      case 'clip_bin_assigned':
      case 'clips_bin_assigned':
      case 'clip_bin_unassigned':
      case 'clips_bin_unassigned':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <FolderInput className="w-3.5 h-3.5" />
            <span>{type.includes('unassigned') ? translate('component.activityLogView.binRemoved') : translate('component.activityLogView.binAssigned')}</span>
          </div>
        );
      case 'clipboard_capture_ignored':
        return (
          <div className="theme-status-warning flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <FileWarning className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.captureSkipped')}</span>
          </div>
        );
      case 'queue_item_added':
      case 'queue_item_recorded':
      case 'queue_reordered':
      case 'queue_recording_started':
      case 'queue_recording_stopped':
      case 'queue_item_removed':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <ListOrdered className="w-3.5 h-3.5" />
            <span>{type === 'queue_reordered' ? translate('component.activityLogView.queueReordered') : type === 'queue_item_removed' ? translate('component.activityLogView.queueRemoved') : type === 'queue_recording_started' ? translate('component.activityLogView.queueRecording') : type === 'queue_recording_stopped' ? translate('component.activityLogView.queueStopped') : translate('component.activityLogView.queued')}</span>
          </div>
        );
      case 'queue_item_pasted':
      case 'queue_all_pasted':
        return (
          <div className="theme-status-success flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <ListOrdered className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.queuePasted')}</span>
          </div>
        );
      case 'queue_paste_failed':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <ListOrdered className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.queueFailed')}</span>
          </div>
        );
      case 'hud_clip_pasted':
        return (
          <div className="theme-status-success flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <ClipboardPaste className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.hudPasted')}</span>
          </div>
        );
      case 'app_hotkey_clip_pasted':
        return (
          <div className="theme-status-success flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Keyboard className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.clipHotkey')}</span>
          </div>
        );
      case 'hud_paste_failed':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <ClipboardPaste className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.hudFailed')}</span>
          </div>
        );
      case 'app_hotkey_clip_paste_failed':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Keyboard className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.clipHotkey')}</span>
          </div>
        );
      case 'trash_emptied':
      case 'clip_deleted':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Trash className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.purged')}</span>
          </div>
        );
      case 'clips_purged_all':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Trash className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.purgedAll')}</span>
          </div>
        );
      case 'clip_protected_toggled':
      case 'clips_protected_toggled': {
        const isProtected = description.startsWith('Protected ');
        return (
          <div className={`flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold ${
            isProtected
              ? 'theme-status-info'
              : 'theme-badge'
          }`}>
            {isProtected ? <ShieldCheck className="w-3.5 h-3.5" /> : <ShieldOff className="w-3.5 h-3.5" />}
            <span>{isProtected ? translate('component.activityLogView.protected') : translate('component.activityLogView.unprotected')}</span>
          </div>
        );
      }
      case 'clip_hotkey_changed':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Keyboard className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.clipHotkey')}</span>
          </div>
        );
      case 'bin_protection_changed':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <ShieldCheck className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.binProtection')}</span>
          </div>
        );
      case 'clip_pinned':
      case 'clips_pinned':
      case 'clip_unpinned':
      case 'clips_unpinned': {
        const isPinned = !type.includes('unpinned');
        return (
          <div className={`${isPinned ? 'theme-status-warning' : 'theme-badge'} flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold`}>
            <Pin className="w-3.5 h-3.5" />
            <span>{isPinned ? translate('component.activityLogView.pinned') : translate('component.activityLogView.unpinned')}</span>
          </div>
        );
      }
      case 'note_updated':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Edit3 className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.note')}</span>
          </div>
        );
      case 'transform_drafted':
      case 'transform_tested':
      case 'transform_saved':
      case 'transform_updated':
      case 'transform_deleted':
      case 'transform_executed':
      case 'transformation_execution_succeeded':
      case 'bin_transform_executed':
      case 'bin_transform_no_change':
      case 'clip_transformed':
        return (
          <div className="theme-status-success flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Workflow className="w-3.5 h-3.5" />
            <span>{type === 'transform_drafted' ? translate('component.activityLogView.drafted') : type === 'transform_tested' ? translate('component.activityLogView.tested') : type === 'transform_saved' ? translate('common.saved') : type === 'transform_updated' ? translate('component.activityLogView.updated') : type === 'transform_deleted' ? translate('component.activityLogView.deleted') : type === 'bin_transform_no_change' ? translate('component.activityLogView.noChange') : translate('component.activityLogView.transformed')}</span>
          </div>
        );
      case 'transform_draft_failed':
      case 'transform_test_failed':
      case 'transform_execution_failed':
      case 'transformation_execution_failed':
      case 'bin_transform_failed':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Workflow className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.transformFailed')}</span>
          </div>
        );
      case 'transform_execution_cancelled':
      case 'transformation_execution_cancelled':
      case 'transform_draft_cancelled':
      case 'transform_test_cancelled':
        return (
          <div className="theme-badge flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Workflow className="w-3.5 h-3.5" />
            <span>{type === 'transform_draft_cancelled' ? translate('component.activityLogView.draftCancelled') : type === 'transform_test_cancelled' ? translate('component.activityLogView.testCancelled') : translate('component.activityLogView.transformCancelled')}</span>
          </div>
        );
      case 'intelligence_connection_fallback':
        return (
          <div className="theme-status-warning flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Workflow className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.connectionFallback')}</span>
          </div>
        );
      default:
        return (
          <div className="theme-badge flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Activity className="w-3.5 h-3.5" />
            <span>{type}</span>
          </div>
        );
    }
  };

  const [selectedTypeFilter, setSelectedTypeFilter] = useState('all');

  const filteredLogs = logs.filter((l) => {
    const matchesSearch =
      l.description.toLowerCase().includes(filter.toLowerCase()) ||
      l.event_type.toLowerCase().includes(filter.toLowerCase());
    if (!matchesSearch) return false;
    if (selectedTypeFilter === 'all') return true;
    if (selectedTypeFilter === 'trashed') return l.event_type === 'clip_trashed' || l.event_type === 'clips_trashed' || l.event_type === 'clip_auto_trashed' || l.event_type === 'clips_trashed_all';
    if (selectedTypeFilter === 'restored') return l.event_type === 'clip_restored' || l.event_type === 'clips_restored_all';
    if (selectedTypeFilter === 'revisions') return l.event_type === 'clip_revision_restored';
    if (selectedTypeFilter === 'purged') return l.event_type === 'clip_deleted' || l.event_type === 'trash_emptied' || l.event_type === 'clips_purged_all';
    if (selectedTypeFilter === 'protection') return l.event_type === 'clip_protected_toggled'
      || l.event_type === 'clips_protected_toggled'
      || l.event_type === 'clip_hotkey_changed'
      || l.event_type === 'bin_protection_changed';
    if (selectedTypeFilter === 'pinning') return l.event_type.includes('pinned');
    if (selectedTypeFilter === 'paused') return l.event_type === 'recording_auto_paused' || l.event_type === 'recording_manually_paused';
    if (selectedTypeFilter === 'resumed') return l.event_type === 'recording_auto_resumed' || l.event_type === 'recording_manually_resumed';
    if (selectedTypeFilter === 'notes') return l.event_type === 'note_updated';
    if (selectedTypeFilter === 'skipped') return l.event_type === 'clipboard_capture_ignored';
    if (selectedTypeFilter === 'transforms') return l.event_type.startsWith('transform_') || l.event_type.startsWith('transformation_') || l.event_type.startsWith('bin_transform_') || l.event_type.startsWith('operation_') || l.event_type.startsWith('pipeline_') || l.event_type === 'library_item_enabled_changed' || l.event_type === 'clip_transformed' || l.event_type === 'intelligence_connection_fallback';
    if (selectedTypeFilter === 'queue') return l.event_type.startsWith('queue_');
    if (selectedTypeFilter === 'hud') return l.event_type.startsWith('hud_');
    if (selectedTypeFilter === 'bins') return l.event_type.startsWith('bin_') || l.event_type.includes('_bin_');
    if (selectedTypeFilter === 'app') return l.event_type.startsWith('app_');
    if (selectedTypeFilter === 'settings') return l.event_type.startsWith('setting_') || l.event_type.startsWith('settings_') || l.event_type.startsWith('autostart_');
    if (selectedTypeFilter === 'analysis') return l.event_type.startsWith('content_classifier')
      || l.event_type.startsWith('content_classification')
      || l.event_type.startsWith('content_detector')
      || l.event_type.startsWith('content_detection')
      || l.event_type.startsWith('content_extractor')
      || l.event_type.startsWith('content_type')
      || l.event_type.startsWith('file_format');
    if (selectedTypeFilter === 'storage') return l.event_type.startsWith('library_')
      || l.event_type.startsWith('backup_')
      || l.event_type.startsWith('data_export_')
      || l.event_type === 'external_history_imported'
      || l.event_type === 'clips_imported';
    return true;
  });

  return (
    <div className="tools-page activity-page flex-1 font-sans h-screen flex flex-col overflow-hidden">
      <ToolPageHeader
        icon={<Activity className="w-4 h-4" />}
        title={translate('destination.activity')}
        actions={(
          <div className="flex items-center space-x-2.5">
          <MenuSelect
            value={selectedTypeFilter}
            onChange={setSelectedTypeFilter}
            label={translate('component.activityLogView.filterActivity')}
            leadingIcon={<ListFilter className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />}
            className="min-w-44"
            options={[
              { value: 'all', get label() { return translate('component.activityLogView.allEventTypes'); } },
              { value: 'app', get label() { return translate('component.activityLogView.appOpenedOrQuit'); }, get group() { return translate('component.activityLogView.application'); } },
              { value: 'settings', get label() { return translate('component.activityLogView.settingsChanged'); }, get group() { return translate('component.activityLogView.application'); } },
              { value: 'analysis', get label() { return translate('component.activityLogView.analysisChanged'); }, get group() { return translate('component.activityLogView.application'); } },
              { value: 'storage', get label() { return translate('component.activityLogView.storageChanged'); }, get group() { return translate('component.activityLogView.application'); } },
              { value: 'paused', get label() { return translate('component.activityLogView.recordingPaused'); }, get group() { return translate('component.activityLogView.capture'); } },
              { value: 'resumed', get label() { return translate('component.activityLogView.recordingResumed'); }, get group() { return translate('component.activityLogView.capture'); } },
              { value: 'skipped', get label() { return translate('component.activityLogView.skippedCaptures'); }, get group() { return translate('component.activityLogView.capture'); } },
              { value: 'trashed', get label() { return translate('component.activityLogView.trashed'); }, get group() { return translate('component.activityLogView.history'); } },
              { value: 'restored', get label() { return translate('component.activityLogView.restoredFromTrash'); }, get group() { return translate('component.activityLogView.history'); } },
              { value: 'revisions', get label() { return translate('component.activityLogView.revisionRestored'); }, get group() { return translate('component.activityLogView.history'); } },
              { value: 'purged', get label() { return translate('component.activityLogView.permanentlyDeleted'); }, get group() { return translate('component.activityLogView.history'); } },
              { value: 'protection', get label() { return translate('component.activityLogView.protectionChanged'); }, get group() { return translate('component.activityLogView.organization'); } },
              { value: 'pinning', get label() { return translate('component.activityLogView.pinningChanged'); }, get group() { return translate('component.activityLogView.organization'); } },
              { value: 'notes', get label() { return translate('component.activityLogView.notesUpdated'); }, get group() { return translate('component.activityLogView.organization'); } },
              { value: 'bins', get label() { return translate('component.activityLogView.bins'); }, get group() { return translate('component.activityLogView.organization'); } },
              { value: 'transforms', get label() { return translate('component.activityLogView.transforms'); }, get group() { return translate('component.activityLogView.automation'); } },
              { value: 'queue', get label() { return translate('component.activityLogView.copyQueue'); }, get group() { return translate('component.activityLogView.automation'); } },
              { value: 'hud', get label() { return translate('component.activityLogView.hud'); }, get group() { return translate('component.activityLogView.automation'); } },
            ]}
          />

          <div className="relative">
            <Search className="theme-text-muted absolute start-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2" />
            <input
              type="text"
              placeholder={translate('component.activityLogView.searchActivity')}
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              className="theme-input ui-field-radius border ps-8 pe-3 py-1.5 text-xs focus:outline-none w-44"
            />
          </div>

          <button
            onClick={() => setIsClearConfirmOpen(true)}
            disabled={logs.length === 0}
            className="theme-secondary-button ui-control-radius flex h-[34px] items-center space-x-1.5 px-3 disabled:opacity-40 border text-xs font-semibold transition-[background-color,border-color,color,opacity] cursor-pointer"
          >
            <Trash2 className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.clearActivity')}</span>
          </button>
          </div>
        )}
      />

      {/* Timeline Content List */}
      <div className="tools-scroll-region flex-1 overflow-y-auto p-6 space-y-3">
        {filteredLogs.length === 0 ? (
          <div className="theme-text-subtle h-full flex flex-col items-center justify-center space-y-2">
            <Activity className="w-10 h-10 opacity-30" />
            <p className="text-xs font-medium">{logs.length === 0 ? translate('component.activityLogView.noActivityRecordedYet') : translate('component.activityLogView.noMatchingActivity')}</p>
          </div>
        ) : (
          filteredLogs.map((log) => (
            <div
              key={log.id}
              className="theme-panel border rounded-xl p-3.5 flex items-center justify-between transition-colors"
            >
              <div className="flex items-center space-x-3.5 min-w-0 flex-1 pe-4">
                {getEventBadge(log.event_type, log.description)}
                <OverflowText text={log.description} className="theme-text-main text-xs truncate font-medium" />
              </div>

              <time
                className="theme-text-muted text-[11px] font-mono shrink-0"
                dateTime={dateTimeAttribute(log.created_at)}
                title={formatFullDateTime(log.created_at)}
              >
                {formatRelativeTime(log.created_at, relativeTimeNow)}
              </time>
            </div>
          ))
        )}
        {isLoadingMore && <p className="theme-text-muted py-2 text-center text-xs" role="status">{translate('component.activityLogView.loadingOlderActivity')}</p>}
        <div ref={loadMoreMarkerRef} className="h-px" aria-hidden="true" />
      </div>
      <AppDialog
        isOpen={isClearConfirmOpen}
        onClose={() => setIsClearConfirmOpen(false)}
        labelledBy="clear-activity-title"
        panelClassName="app-dialog-danger theme-panel w-full max-w-md rounded-2xl border overflow-hidden font-sans"
      >
        {({ requestClose }) => <>
          <AppDialogHeader onClose={requestClose}>
            <AppDialogHeading
              id="clear-activity-title"
              title={translate('component.activityLogView.clearActivity2')}
              description={translate('common.thisActionCannotBeUndone')}
              icon={<Trash2 />}
              tone="danger"
            />
          </AppDialogHeader>
          <AppDialogBody>
            <p className="app-dialog-message theme-surface rounded-xl border p-3 text-xs leading-relaxed">
              {translate('component.activityLogView.permanentlyRemoveEveryRetainedActivityEntryClipsAndOtherLibraryDataAre')}
            </p>
          </AppDialogBody>
          <AppDialogFooter>
            <AppDialogButton onClick={requestClose} autoFocus>{translate('common.cancel')}</AppDialogButton>
            <AppDialogButton variant="danger" onClick={() => void handleClearLogs()}>{translate('component.activityLogView.clearActivity')}</AppDialogButton>
          </AppDialogFooter>
        </>}
      </AppDialog>
    </div>
  );
};
