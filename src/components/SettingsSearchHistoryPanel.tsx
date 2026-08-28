import { ChevronLeft, ChevronRight, LoaderCircle, Play, Search, Trash2 } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { searchHistoryApi } from '../api/searchHistory';
import { useLocalization } from '../localization/LocalizationProvider';
import { translate } from '../localization/runtime';
import type { ClipSearchRequest, SearchHistoryEntry } from '../types';
import { dateTimeAttribute, formatFullDateTime, formatRelativeTime } from '../utils/date';
import { searchHistoryRequestQuery, searchHistoryRequestSummary } from '../utils/searchHistory';
import { ActionButton } from './AppDialogLayout';
import { ConfirmationDialog, type ConfirmationDialogRequest } from './ConfirmationDialog';
import { SettingsPanelHeader } from './SettingsPanelHeader';
import { SettingsLoadingState } from './SettingsLoadingState';

const PAGE_SIZE = 50;

export function SettingsSearchHistoryPanel({
  onRunSearch,
}: {
  onRunSearch: (request: ClipSearchRequest) => void;
}) {
  const { formatNumber } = useLocalization();
  const [entries, setEntries] = useState<SearchHistoryEntry[]>([]);
  const [totalCount, setTotalCount] = useState(0);
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [deletingIds, setDeletingIds] = useState<Set<number>>(() => new Set());
  const [confirmation, setConfirmation] = useState<ConfirmationDialogRequest | null>(null);

  const load = useCallback(async (offset = 0) => {
    setLoading(true);
    setError(false);
    try {
      const page = await searchHistoryApi.list(PAGE_SIZE, offset);
      setEntries(page.items);
      setTotalCount(page.totalCount);
      setOffset(offset);
    } catch (reason) {
      console.error('Failed to load Search history:', reason);
      setError(true);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void load(); }, [load]);

  const deleteEntry = async (id: number) => {
    setDeletingIds((current) => new Set(current).add(id));
    try {
      await searchHistoryApi.delete(id);
      if (entries.length === 1 && offset > 0) {
        await load(Math.max(0, offset - PAGE_SIZE));
      } else {
        setEntries((current) => current.filter((entry) => entry.id !== id));
        setTotalCount((current) => Math.max(0, current - 1));
      }
    } catch (reason) {
      console.error('Failed to delete Search history entry:', reason);
      setError(true);
    } finally {
      setDeletingIds((current) => {
        const next = new Set(current);
        next.delete(id);
        return next;
      });
    }
  };

  const requestClear = () => setConfirmation({
    title: translate('component.settingsSearchHistoryPanel.clearSearchHistory'),
    description: translate('component.settingsSearchHistoryPanel.clearSearchHistoryDescription'),
    confirmLabel: translate('component.settingsSearchHistoryPanel.clearAll'),
    tone: 'danger',
    onConfirm: async () => {
      await searchHistoryApi.clear();
      setEntries([]);
      setTotalCount(0);
      setOffset(0);
      setConfirmation(null);
    },
  });

  return <>
    <div className="space-y-5 text-xs">
      <SettingsPanelHeader
        icon={Search}
        title={translate('component.settingsSearchHistoryPanel.searchHistory')}
        description={translate('component.settingsSearchHistoryPanel.reviewRerunAndRemoveSearches')}
      />

      <div className="flex items-center justify-between gap-4">
        <span className="theme-text-muted">
          {translate('component.settingsSearchHistoryPanel.searchCount', { count: totalCount })}
        </span>
        <ActionButton variant="danger" onClick={requestClear} disabled={totalCount === 0} className="shrink-0">
          <Trash2 className="h-3.5 w-3.5" />
          <span>{translate('component.settingsSearchHistoryPanel.clearAllEllipsis')}</span>
        </ActionButton>
      </div>

      {error && (
        <div className="theme-status-danger rounded-xl border px-3 py-2" role="alert">
          {translate('component.settingsSearchHistoryPanel.couldNotLoadSearchHistory')}
        </div>
      )}

      {loading && entries.length === 0 && (
        <SettingsLoadingState label={translate('component.settingsSearchHistoryPanel.loading')} className="theme-surface min-h-24 rounded-xl border px-4 py-8" />
      )}

      {!loading && entries.length === 0 && !error && (
        <div className="theme-surface theme-text-muted rounded-xl border px-4 py-8 text-center">
          {translate('component.settingsSearchHistoryPanel.noSearchHistory')}
        </div>
      )}

      {entries.length > 0 && (
        <ol className={`theme-surface overflow-hidden rounded-xl border transition-opacity ${loading ? 'opacity-60' : ''}`} aria-busy={loading}>
          {entries.map((entry, index) => {
            const runnableQuery = searchHistoryRequestQuery(entry.request);
            const filterSummary = searchHistoryRequestSummary(entry.request);
            return <li key={entry.id} className={`${index > 0 ? 'theme-divider border-t' : ''} flex items-start gap-3 px-3 py-3`}>
              <div className="min-w-0 flex-1">
                <div className="theme-text-main break-words font-mono text-[11px]" dir="auto">{entry.request.query}</div>
                {filterSummary && <div className="theme-text-muted mt-1 break-words font-mono text-[10px]" dir="ltr">{filterSummary}</div>}
                <div className="theme-text-subtle mt-1 flex flex-wrap gap-x-3 gap-y-1 text-[10px]">
                  <span>{translate('component.settingsSearchHistoryPanel.resultCount', { count: entry.resultCount })}</span>
                  <span>{translate('component.settingsSearchHistoryPanel.usedCount', { count: entry.useCount })}</span>
                  <time dateTime={dateTimeAttribute(entry.lastUsedAt)} title={formatFullDateTime(entry.lastUsedAt)}>
                    {formatRelativeTime(entry.lastUsedAt)}
                  </time>
                </div>
              </div>
              <div className="flex shrink-0 items-center gap-1">
                <button type="button" disabled={!runnableQuery} onClick={() => onRunSearch(entry.request)} className="theme-menu-item grid h-8 w-8 place-items-center rounded-lg disabled:cursor-not-allowed disabled:opacity-40" title={runnableQuery ? translate('component.settingsSearchHistoryPanel.runSearch') : translate('component.settingsSearchHistoryPanel.cannotRunSearch')} aria-label={runnableQuery ? translate('component.settingsSearchHistoryPanel.runSearch') : translate('component.settingsSearchHistoryPanel.cannotRunSearch')}>
                  <Play className="h-3.5 w-3.5" />
                </button>
                <button type="button" disabled={deletingIds.has(entry.id)} onClick={() => void deleteEntry(entry.id)} className="theme-menu-item grid h-8 w-8 place-items-center rounded-lg disabled:cursor-not-allowed disabled:opacity-40" title={translate('component.settingsSearchHistoryPanel.deleteSearch')} aria-label={translate('component.settingsSearchHistoryPanel.deleteSearch')}>
                  <Trash2 className="h-3.5 w-3.5" />
                </button>
              </div>
            </li>;
          })}
        </ol>
      )}

      {totalCount > (PAGE_SIZE) && (
        <div className="flex flex-wrap items-center justify-between gap-3">
          <span className="theme-text-muted text-[10px]">
            {translate('component.settingsSearchHistoryPanel.showingRange', {
              start: formatNumber(offset + 1),
              end: formatNumber(offset + entries.length),
              count: formatNumber(totalCount),
            })}
          </span>
          <div className="flex items-center gap-2">
            <ActionButton onClick={() => void load(Math.max(0, offset - PAGE_SIZE))} disabled={loading || offset === 0}>
              <ChevronLeft className="h-3.5 w-3.5 rtl:-scale-x-100" />
              {translate('component.settingsSearchHistoryPanel.previousPage')}
            </ActionButton>
            <ActionButton onClick={() => void load(offset + PAGE_SIZE)} disabled={loading || offset + entries.length >= totalCount}>
              {loading && <LoaderCircle className="h-3.5 w-3.5 animate-spin" aria-hidden="true" />}
              {translate('component.settingsSearchHistoryPanel.nextPage')}
              <ChevronRight className="h-3.5 w-3.5 rtl:-scale-x-100" />
            </ActionButton>
          </div>
        </div>
      )}
    </div>
    <ConfirmationDialog request={confirmation} onCancel={() => setConfirmation(null)} />
  </>;
}
