import React, { useState, useEffect, useCallback, useRef } from 'react';
import { activityApi, type ActivityLog } from '../api/activity';
import { Activity, ListFilter, Search, Trash2 } from 'lucide-react';
import { ToolPageHeader } from './ToolPageHeader';
import { MenuSelect } from './MenuSelect';
import { OverflowText } from './OverflowText';
import { useMinuteTick } from '../hooks/useMinuteTick';
import { dateTimeAttribute, formatFullDateTime, formatRelativeTime } from '../utils/date';
import { AppDialog } from './AppDialog';
import { AppDialogBody, AppDialogButton, AppDialogFooter, AppDialogHeader, AppDialogHeading } from './AppDialogLayout';
import { translate } from '../localization/runtime';
import { ActivityEventBadge } from './ActivityEventBadge';
import { activityLogMatches } from './activityLogFilter';

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
      const res = await activityApi.list(ACTIVITY_BATCH_SIZE, 0);
      replaceLogs(res);
      setHasMore(res.length === ACTIVITY_BATCH_SIZE);
    } catch (e) {
      console.error('Failed to fetch activity logs:', e);
    }
  }, [replaceLogs]);

  const refreshNewestLogs = useCallback(async () => {
    try {
      const newest = await activityApi.list(ACTIVITY_BATCH_SIZE, 0);
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
      const older = await activityApi.list(ACTIVITY_BATCH_SIZE, logsRef.current.length);
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
      await activityApi.clear();
      replaceLogs([]);
      setHasMore(false);
      setIsClearConfirmOpen(false);
    } catch (e) {
      console.error('Failed to clear logs:', e);
    }
  };

  const [selectedTypeFilter, setSelectedTypeFilter] = useState('all');

  const filteredLogs = logs.filter((log) => activityLogMatches(log, filter, selectedTypeFilter));

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
              { value: 'revisions', get label() { return translate('component.activityLogView.versionActivity'); }, get group() { return translate('component.activityLogView.history'); } },
              { value: 'purged', get label() { return translate('component.activityLogView.permanentlyDeleted'); }, get group() { return translate('component.activityLogView.history'); } },
              { value: 'protection', get label() { return translate('component.activityLogView.protectionChanged'); }, get group() { return translate('component.activityLogView.organization'); } },
              { value: 'pinning', get label() { return translate('component.activityLogView.pinningChanged'); }, get group() { return translate('component.activityLogView.organization'); } },
              { value: 'notes', get label() { return translate('component.activityLogView.notesUpdated'); }, get group() { return translate('component.activityLogView.organization'); } },
              { value: 'names', get label() { return translate('common.name'); }, get group() { return translate('component.activityLogView.organization'); } },
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
                <ActivityEventBadge type={log.event_type} description={log.description} />
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
