import React, { useState, useEffect } from 'react';
import { safeInvoke as invoke } from '../utils/tauri';
import {
  BarChart3,
  PieChart,
  HardDrive,
  Cpu,
  Layers,
  TrendingUp,
  Calendar,
} from 'lucide-react';
import { ToolPageHeader } from './ToolPageHeader';
import { OverflowText } from './OverflowText';
import { ContentTypeIcon } from './ContentTypeIcon';
import { useContentTypes } from './ContentTypeProvider';
import type { ClipContentType } from '../types';

interface SourceStat {
  name: string;
  count: number;
}

interface TypeStat {
  content_type: string;
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
  content_types: TypeStat[];
  daily_activity: DailyStat[];
}

export const AnalyticsView: React.FC = () => {
  const { definitions: registeredContentTypes, groups: contentTypeGroups } = useContentTypes();
  const [summary, setSummary] = useState<AnalyticsSummary | null>(null);

  const loadStats = async () => {
    try {
      const data = await invoke<AnalyticsSummary>('get_analytics_summary');
      setSummary(data);
    } catch (e) {
      console.error('Failed to fetch analytics summary:', e);
    }
  };

  useEffect(() => {
    loadStats();
  }, []);

  const totalClips = summary?.total_clips || 0;
  const totalChars = summary?.total_chars || 0;
  const topSources = summary?.top_sources || [];
  const contentTypes = summary?.content_types || [];
  const dailyActivity = summary?.daily_activity || [];

  const getTypeCount = (type: string) => {
    return contentTypes.find((t) => t.content_type === type)?.count || 0;
  };
  const visibleContentTypes = [
    ...registeredContentTypes
      .filter(({ id }) => getTypeCount(id) > 0)
      .map(({ id, label, group }) => ({ value: id as ClipContentType, label, group: contentTypeGroups.find(({ id: groupId }) => groupId === group)?.label ?? group })),
    ...contentTypes
      .filter(({ content_type }) => !registeredContentTypes.some(({ id }) => id === content_type))
      .map(({ content_type }) => ({ value: content_type as ClipContentType, label: content_type, group: 'custom' })),
  ];

  return (
    <div className="tools-page analytics-page flex-1 h-screen overflow-hidden font-sans select-none flex flex-col">
      <ToolPageHeader
        icon={<BarChart3 className="w-4 h-4" />}
        title="Analytics & Insights"
        description="Clipboard trends, sources, and storage efficiency"
      />

      <div className="tools-scroll-region flex-1 overflow-y-auto p-6">

      {/* Top Stat Cards Grid */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        <div className="theme-panel p-4 rounded-xl border flex items-center space-x-4">
          <div className="theme-status-info p-3 rounded-lg border">
            <Layers className="w-6 h-6" />
          </div>
          <div>
            <div className="theme-title text-2xl font-extrabold font-mono">
              {totalClips.toLocaleString()}
            </div>
            <div className="theme-text-muted text-xs font-medium">Total Clips Saved</div>
          </div>
        </div>

        <div className="theme-panel p-4 rounded-xl border flex items-center space-x-4">
          <div className="theme-status-success p-3 rounded-lg border">
            <HardDrive className="w-6 h-6" />
          </div>
          <div>
            <div className="theme-title text-2xl font-extrabold font-mono">
              {totalChars.toLocaleString()}
            </div>
            <div className="theme-text-muted text-xs font-medium">Text Characters Saved</div>
          </div>
        </div>

        <div className="theme-panel p-4 rounded-xl border flex items-center space-x-4">
          <div className="theme-status-warning p-3 rounded-lg border">
            <TrendingUp className="w-6 h-6" />
          </div>
          <div>
            <OverflowText as="div" text={topSources[0]?.name || '—'} className="theme-title text-2xl font-extrabold font-mono truncate max-w-[140px]" />
            <div className="theme-text-muted text-xs font-medium">Top Source</div>
          </div>
        </div>
      </div>

      {/* Detailed Insights Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
        {/* Top Sources */}
        <div className="theme-panel p-5 rounded-xl border flex flex-col">
          <h2 className="theme-title text-sm font-bold mb-4 flex items-center space-x-2">
            <Cpu className="w-4 h-4 theme-status-info-text" />
            <span>Top Sources</span>
          </h2>
          <div className="space-y-3 flex-1">
            {topSources.length === 0 ? (
              <div className="theme-text-subtle text-xs py-6 text-center">No source data recorded yet</div>
            ) : (
              topSources.map((source) => {
                const pct = Math.round((source.count / Math.max(1, totalClips)) * 100);
                return (
                  <div key={source.name} className="space-y-1">
                    <div className="flex justify-between text-xs font-mono">
                      <span className="theme-text-main font-medium">{source.name}</span>
                      <span className="theme-text-muted">{source.count} clips ({pct}%)</span>
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
        </div>

        {/* Content Type Breakdown */}
        <div className="theme-panel p-5 rounded-xl border flex flex-col">
          <h2 className="theme-title text-sm font-bold mb-4 flex items-center space-x-2">
            <PieChart className="w-4 h-4 theme-status-info-text" />
            <span>Content Type Breakdown</span>
          </h2>
          <div className="grid grid-cols-2 gap-3 flex-1">
            {visibleContentTypes.length === 0 ? (
              <div className="theme-text-subtle col-span-2 py-6 text-center text-xs">No content types recorded yet</div>
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
        </div>
      </div>

      {/* Daily Activity Timeline */}
      <div className="theme-panel p-5 rounded-xl border">
        <h2 className="theme-title text-sm font-bold mb-4 flex items-center space-x-2">
          <Calendar className="w-4 h-4 theme-status-info-text" />
          <span>Daily Clipboard Activity (Recent Days)</span>
        </h2>
        <div className="space-y-2">
          {dailyActivity.length === 0 ? (
            <div className="theme-text-subtle text-xs py-4 text-center">No daily activity recorded</div>
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
                  <span className="theme-text-main font-bold w-12 text-right">{day.count}</span>
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
