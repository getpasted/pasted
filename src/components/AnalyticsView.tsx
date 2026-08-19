import React, { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { analysisApi } from '../api/analysis';
import {
  BarChart3,
  PieChart,
  HardDrive,
  Cpu,
  Layers,
  TrendingUp,
  Calendar,
  AlertCircle,
  LoaderCircle,
  FileText,
  Image,
  Files,
} from 'lucide-react';
import { ToolPageHeader } from './ToolPageHeader';
import { OverflowText } from './OverflowText';
import { ContentTypeIcon } from './ContentTypeIcon';
import { useContentTypes } from './ContentTypeProvider';
import type { ClipContentType } from '../types';
import { useFeatures } from '../hooks/useFeatures';
import { useLocalization } from '../localization/LocalizationProvider';
import { translate } from '../localization/runtime';
import { localizedContentTypeGroupLabel } from '../localization/presentation';
import { contentTypeLabel } from '../utils/contentTypes';
import { APP_EVENTS } from '../utils/appEvents';

interface SourceStat {
  name: string;
  count: number;
}

interface TypeStat {
  content_type: string;
  count: number;
}

interface ClipTypeStat {
  clip_type: string;
  count: number;
}

interface FileFormatStat {
  file_format: string;
  count: number;
}

interface DailyStat {
  date: string;
  count: number;
}

interface AnalyticsSummary {
  total_clips: number;
  total_chars: number;
  top_sources: SourceStat[];
  clip_types: ClipTypeStat[];
  file_formats: FileFormatStat[];
  content_types: TypeStat[];
  daily_activity: DailyStat[];
}

export const AnalyticsView: React.FC = () => {
  const features = useFeatures();
  const { formatNumber } = useLocalization();
  const { definitions: registeredContentTypes, groups: contentTypeGroups } = useContentTypes();
  const [summary, setSummary] = useState<AnalyticsSummary | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);

  const loadStats = async () => {
    setLoadError(null);
    try {
      const data = await analysisApi.analyticsSummary<AnalyticsSummary>();
      setSummary(data);
    } catch (e) {
      console.error('Failed to fetch analytics summary:', e);
      setLoadError(translate('component.analyticsView.couldNotLoadHistorySummary'));
    }
  };

  useEffect(() => {
    let disposed = false;
    let midnightTimer: ReturnType<typeof setTimeout> | undefined;
    const unlisteners: Array<Promise<() => void>> = [];
    const refresh = () => {
      if (!disposed) void loadStats();
    };
    const scheduleLocalMidnightRefresh = () => {
      const now = new Date();
      const nextMidnight = new Date(
        now.getFullYear(),
        now.getMonth(),
        now.getDate() + 1,
        0,
        0,
        1,
      );
      midnightTimer = setTimeout(() => {
        refresh();
        scheduleLocalMidnightRefresh();
      }, Math.max(1, nextMidnight.getTime() - now.getTime()));
    };

    refresh();
    scheduleLocalMidnightRefresh();
    if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
      unlisteners.push(listen(APP_EVENTS.clipAdded, refresh));
      unlisteners.push(listen(APP_EVENTS.clipLibraryChanged, refresh));
      unlisteners.push(listen('tauri://focus', refresh));
    }

    return () => {
      disposed = true;
      if (midnightTimer) clearTimeout(midnightTimer);
      unlisteners.forEach((unlisten) => void unlisten.then((stop) => stop()));
    };
  }, []);

  if (!summary) {
    return (
      <div className="tools-page analytics-page flex h-screen flex-1 select-none flex-col overflow-hidden font-sans">
        <ToolPageHeader
          icon={<BarChart3 className="w-4 h-4" />}
          title={translate('destination.insights')}
          description={translate('component.analyticsView.currentHistoryCompositionAndRecentCaptureTrends')}
        />
        <div className="theme-text-muted flex flex-1 flex-col items-center justify-center gap-3 p-6 text-xs" role={loadError ? 'alert' : 'status'}>
          {loadError ? <AlertCircle className="theme-danger-text h-7 w-7" /> : <LoaderCircle className="h-7 w-7 animate-spin" />}
          <p>{loadError ?? translate('component.analyticsView.loadingInsights')}</p>
          {loadError && (
            <button type="button" onClick={() => void loadStats()} className="theme-secondary-button ui-control-radius border px-3 py-1.5 font-semibold">
              {translate('component.analyticsView.tryAgain')}
            </button>
          )}
        </div>
      </div>
    );
  }

  const totalClips = summary.total_clips;
  const totalChars = summary.total_chars;
  const topSources = summary.top_sources;
  const clipTypes = summary.clip_types;
  const fileFormats = summary.file_formats;
  const contentTypes = summary.content_types;
  const dailyActivity = summary.daily_activity;

  const getTypeCount = (type: string) => {
    return contentTypes.find((t) => t.content_type === type)?.count || 0;
  };
  const visibleContentTypes = [
    ...registeredContentTypes
      .filter(({ id }) => getTypeCount(id) > 0)
      .map(({ id, group }) => {
        const groupDefinition = contentTypeGroups.find(({ id: groupId }) => groupId === group);
        return {
          value: id as ClipContentType,
          label: contentTypeLabel(id),
          group: groupDefinition
            ? localizedContentTypeGroupLabel(groupDefinition.id, groupDefinition.label, groupDefinition.isBuiltin, groupDefinition.defaults?.label)
            : group,
        };
      }),
    ...contentTypes
      .filter(({ content_type }) => !registeredContentTypes.some(({ id }) => id === content_type))
      .map(({ content_type }) => ({ value: content_type as ClipContentType, label: content_type, group: 'custom' })),
  ];
  const structuralTypes = [
    { value: 'text', get label() { return translate('component.analyticsView.text'); }, icon: FileText },
    { value: 'image', get label() { return translate('component.analyticsView.image'); }, icon: Image },
    { value: 'file', get label() { return translate('component.analyticsView.files'); }, icon: Files },
  ];

  return (
    <div className="tools-page analytics-page flex-1 h-screen overflow-hidden font-sans select-none flex flex-col">
      <ToolPageHeader
        icon={<BarChart3 className="w-4 h-4" />}
        title={translate('destination.insights')}
        description={translate('component.analyticsView.currentHistoryCompositionAndRecentCaptureTrends')}
      />

      <div className="tools-scroll-region flex-1 overflow-y-auto p-6">

      {/* Top Stat Cards Grid */}
      <div className={`grid grid-cols-1 gap-4 mb-6 ${features.sources ? 'md:grid-cols-3' : 'md:grid-cols-2'}`}>
        <div className="theme-panel p-4 rounded-xl border flex items-center space-x-4">
          <div className="theme-status-info p-3 rounded-lg border">
            <Layers className="w-6 h-6" />
          </div>
          <div>
            <div className="theme-title text-2xl font-extrabold font-mono">
              {formatNumber(totalClips)}
            </div>
            <div className="theme-text-muted text-xs font-medium">{translate('component.analyticsView.clipsInHistory')}</div>
          </div>
        </div>

        <div className="theme-panel p-4 rounded-xl border flex items-center space-x-4">
          <div className="theme-status-success p-3 rounded-lg border">
            <HardDrive className="w-6 h-6" />
          </div>
          <div>
            <div className="theme-title text-2xl font-extrabold font-mono">
              {formatNumber(totalChars)}
            </div>
            <div className="theme-text-muted text-xs font-medium">{translate('component.analyticsView.textCharactersInHistory')}</div>
          </div>
        </div>

        {features.sources && <div className="theme-panel p-4 rounded-xl border flex items-center space-x-4">
          <div className="theme-status-warning p-3 rounded-lg border">
            <TrendingUp className="w-6 h-6" />
          </div>
          <div>
            <OverflowText as="div" text={topSources[0]?.name || '—'} className="theme-title text-2xl font-extrabold font-mono truncate max-w-[140px]" />
            <div className="theme-text-muted text-xs font-medium">{translate('component.analyticsView.topSourceInHistory')}</div>
          </div>
        </div>}
      </div>

      {/* Detailed Insights Grid */}
      <div className="grid grid-cols-1 gap-6 mb-6 lg:grid-cols-2">
        {/* Top Sources */}
        {features.sources && <div className="theme-panel p-5 rounded-xl border flex flex-col">
          <h2 className="theme-title text-sm font-bold mb-4 flex items-center space-x-2">
            <Cpu className="w-4 h-4 theme-status-info-text" />
            <span>{translate('component.analyticsView.topSourcesInHistory')}</span>
          </h2>
          <div className="space-y-3 flex-1">
            {topSources.length === 0 ? (
              <div className="theme-text-subtle text-xs py-6 text-center">{translate('component.analyticsView.noSourceDataRecordedYet')}</div>
            ) : (
              topSources.map((source) => {
                const pct = Math.round((source.count / Math.max(1, totalClips)) * 100);
                return (
                  <div key={source.name} className="space-y-1">
                    <div className="flex justify-between text-xs font-mono">
                      <span className="theme-text-main font-medium">{source.name}</span>
                      <span className="theme-text-muted">{translate('component.analyticsView.sourceCountPercent', { count: source.count, percent: pct })}</span>
                    </div>
                    <div className="theme-track w-full h-2 rounded-full overflow-hidden">
                      <div
                        className="analytics-progress h-full rounded-full transition-[width] duration-500"
                        style={{ width: `${pct}%` }}
                      />
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </div>}

        {features.clipTypes && <div className="theme-panel p-5 rounded-xl border flex flex-col">
          <h2 className="theme-title text-sm font-bold mb-4 flex items-center space-x-2">
            <Layers className="w-4 h-4 theme-status-info-text" />
            <span>{translate('component.analyticsView.clipsByClipType')}</span>
          </h2>
          <div className="grid grid-cols-3 gap-3 flex-1">
            {structuralTypes.map(({ value, label, icon: Icon }) => (
              <div key={value} className="theme-surface p-3 rounded-lg border flex items-center space-x-3">
                <Icon className="w-4 h-4 theme-text-muted shrink-0" />
                <div className="min-w-0">
                  <div className="theme-title text-sm font-bold font-mono">{clipTypes.find((type) => type.clip_type === value)?.count ?? 0}</div>
                  <div className="theme-text-muted truncate text-[11px]">{label}</div>
                </div>
              </div>
            ))}
          </div>
        </div>}

        {features.fileFormats && <div className="theme-panel p-5 rounded-xl border flex flex-col">
          <h2 className="theme-title text-sm font-bold mb-4 flex items-center space-x-2">
            <Files className="w-4 h-4 theme-status-info-text" />
            <span>{translate('component.analyticsView.clipsByFileFormat')}</span>
          </h2>
          <div className="grid grid-cols-2 gap-3 flex-1">
            {fileFormats.length === 0 ? (
              <div className="theme-text-subtle col-span-2 py-6 text-center text-xs">{translate('component.analyticsView.noFileFormatsRecordedYet')}</div>
            ) : fileFormats.map(({ file_format: format, count }) => (
              <div key={format} className="theme-surface p-3 rounded-lg border flex items-center space-x-3">
                <FileText className="w-4 h-4 theme-text-muted shrink-0" />
                <div className="min-w-0">
                  <div className="theme-title text-sm font-bold font-mono">{count}</div>
                  <div className="theme-text-muted truncate text-[11px]">{format.toUpperCase()}</div>
                </div>
              </div>
            ))}
          </div>
        </div>}

        {/* Content Type Breakdown */}
        {features.types && <div className="theme-panel p-5 rounded-xl border flex flex-col">
          <h2 className="theme-title text-sm font-bold mb-4 flex items-center space-x-2">
            <PieChart className="w-4 h-4 theme-status-info-text" />
            <span>{translate('component.analyticsView.clipsByContentType')}</span>
          </h2>
          <div className="grid grid-cols-2 gap-3 flex-1">
            {visibleContentTypes.length === 0 ? (
              <div className="theme-text-subtle col-span-2 py-6 text-center text-xs">{translate('component.analyticsView.noContentTypesRecordedYet')}</div>
            ) : visibleContentTypes.map(({ value, label }) => (
              <div key={value} className="theme-surface p-3 rounded-lg border flex items-center space-x-3">
                <ContentTypeIcon type={value} className="w-4 h-4 theme-text-muted shrink-0" />
                <div className="min-w-0">
                  <div className="theme-title text-sm font-bold font-mono">{getTypeCount(value)}</div>
                  <div className="theme-text-muted truncate text-[11px]">{label}</div>
                </div>
              </div>
            ))}
          </div>
        </div>}
      </div>

      {/* Daily Activity Timeline */}
      <div className="theme-panel p-5 rounded-xl border">
        <h2 className="theme-title text-sm font-bold mb-4 flex items-center space-x-2">
          <Calendar className="w-4 h-4 theme-status-info-text" />
          <span>{translate('component.analyticsView.clipsAddedToHistoryLast14Days')}</span>
        </h2>
        <div className="space-y-2">
          {dailyActivity.length === 0 ? (
            <div className="theme-text-subtle text-xs py-4 text-center">{translate('component.analyticsView.noDailyActivityRecorded')}</div>
          ) : (
            dailyActivity.map((day) => {
              const maxDay = Math.max(1, ...dailyActivity.map((d) => d.count));
              const pct = Math.round((day.count / maxDay) * 100);
              return (
                <div key={day.date} className="flex items-center space-x-3 text-xs font-mono">
                  <span className="theme-text-muted w-24 shrink-0">{day.date}</span>
                  <div className="theme-track flex-1 h-2 rounded-full overflow-hidden">
                    <div
                      className="analytics-progress h-full rounded-full"
                      style={{ width: `${pct}%` }}
                    />
                  </div>
                  <span className="theme-text-main font-bold w-12 text-end">{day.count}</span>
                </div>
              );
            })
          )}
        </div>
      </div>
      </div>
    </div>
  );
};
