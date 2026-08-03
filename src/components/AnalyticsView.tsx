import React, { useState, useEffect } from 'react';
import { safeInvoke as invoke } from '../utils/tauri';
import {
  BarChart3,
  PieChart,
  HardDrive,
  Cpu,
  Layers,
  TrendingUp,
  FileText,
  Code,
  Image as ImageIcon,
  Link as LinkIcon,
  Palette,
  Calendar,
} from 'lucide-react';

interface AppStat {
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
  kb_saved: number;
  top_apps: AppStat[];
  content_types: TypeStat[];
  daily_activity: DailyStat[];
}

export const AnalyticsView: React.FC = () => {
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
  const kbSaved = (summary?.kb_saved || 0).toFixed(1);
  const topApps = summary?.top_apps || [];
  const contentTypes = summary?.content_types || [];
  const dailyActivity = summary?.daily_activity || [];

  const getTypeCount = (type: string) => {
    return contentTypes.find((t) => t.content_type === type)?.count || 0;
  };

  return (
    <div className="tools-page analytics-page flex-1 h-screen p-6 overflow-y-auto font-sans select-none">
      {/* Header */}
      <div className="theme-divider flex items-center justify-between pb-6 border-b mb-6">
        <div>
          <h1 className="theme-title text-xl font-bold flex items-center space-x-2">
            <BarChart3 className="w-5 h-5 text-cyan-400" />
            <span>Analytics & Insights</span>
          </h1>
          <p className="theme-text-muted text-xs mt-1">
            Telemetry, app source breakdowns, and clipboard storage efficiency metrics
          </p>
        </div>
      </div>

      {/* Top Stat Cards Grid */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        <div className="theme-panel p-4 rounded-xl border flex items-center space-x-4">
          <div className="p-3 rounded-lg bg-blue-500/10 border border-blue-500/20 text-blue-400">
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
          <div className="p-3 rounded-lg bg-emerald-500/10 border border-emerald-500/20 text-emerald-400">
            <HardDrive className="w-6 h-6" />
          </div>
          <div>
            <div className="theme-title text-2xl font-extrabold font-mono">
              {kbSaved} <span className="text-xs text-emerald-400">KB</span>
            </div>
            <div className="theme-text-muted text-xs font-medium">Storage Compressed</div>
          </div>
        </div>

        <div className="theme-panel p-4 rounded-xl border flex items-center space-x-4">
          <div className="p-3 rounded-lg bg-amber-500/10 border border-amber-500/20 text-amber-400">
            <TrendingUp className="w-6 h-6" />
          </div>
          <div>
            <div className="theme-title text-2xl font-extrabold font-mono truncate max-w-[140px]">
              {topApps[0]?.name || 'VS Code'}
            </div>
            <div className="theme-text-muted text-xs font-medium">Top Copied App</div>
          </div>
        </div>
      </div>

      {/* Detailed Insights Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
        {/* Top Applications */}
        <div className="theme-panel p-5 rounded-xl border flex flex-col">
          <h2 className="theme-title text-sm font-bold mb-4 flex items-center space-x-2">
            <Cpu className="w-4 h-4 text-cyan-400" />
            <span>Top Source Applications</span>
          </h2>
          <div className="space-y-3 flex-1">
            {topApps.length === 0 ? (
              <div className="text-xs text-gray-500 py-6 text-center">No app data recorded yet</div>
            ) : (
              topApps.map((app) => {
                const pct = Math.round((app.count / Math.max(1, totalClips)) * 100);
                return (
                  <div key={app.name} className="space-y-1">
                    <div className="flex justify-between text-xs font-mono">
                      <span className="theme-text-main font-medium">{app.name}</span>
                      <span className="theme-text-muted">{app.count} clips ({pct}%)</span>
                    </div>
                    <div className="theme-track w-full h-2 rounded-full overflow-hidden">
                      <div
                        className="h-full bg-cyan-500/80 rounded-full transition-all duration-500"
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
            <PieChart className="w-4 h-4 text-purple-400" />
            <span>Content Type Breakdown</span>
          </h2>
          <div className="grid grid-cols-2 gap-3 flex-1">
            <div className="theme-surface p-3 rounded-lg border flex items-center space-x-3">
              <FileText className="w-4 h-4 text-gray-400 shrink-0" />
              <div>
                <div className="theme-title text-sm font-bold font-mono">{getTypeCount('text')}</div>
                <div className="theme-text-muted text-[11px]">Plain Text</div>
              </div>
            </div>
            <div className="theme-surface p-3 rounded-lg border flex items-center space-x-3">
              <Code className="w-4 h-4 text-emerald-400 shrink-0" />
              <div>
                <div className="theme-title text-sm font-bold font-mono">{getTypeCount('code')}</div>
                <div className="theme-text-muted text-[11px]">Code Snippets</div>
              </div>
            </div>
            <div className="theme-surface p-3 rounded-lg border flex items-center space-x-3">
              <LinkIcon className="w-4 h-4 text-blue-400 shrink-0" />
              <div>
                <div className="theme-title text-sm font-bold font-mono">{getTypeCount('link')}</div>
                <div className="theme-text-muted text-[11px]">Links / URLs</div>
              </div>
            </div>
            <div className="theme-surface p-3 rounded-lg border flex items-center space-x-3">
              <ImageIcon className="w-4 h-4 text-pink-400 shrink-0" />
              <div>
                <div className="theme-title text-sm font-bold font-mono">{getTypeCount('image')}</div>
                <div className="theme-text-muted text-[11px]">Images</div>
              </div>
            </div>
            <div className="theme-surface p-3 rounded-lg border flex items-center space-x-3 col-span-2">
              <Palette className="w-4 h-4 text-amber-400 shrink-0" />
              <div>
                <div className="theme-title text-sm font-bold font-mono">{getTypeCount('color')}</div>
                <div className="theme-text-muted text-[11px]">Color Swatches</div>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Daily Activity Timeline */}
      <div className="theme-panel p-5 rounded-xl border">
        <h2 className="theme-title text-sm font-bold mb-4 flex items-center space-x-2">
          <Calendar className="w-4 h-4 text-emerald-400" />
          <span>Daily Clipboard Activity (Recent Days)</span>
        </h2>
        <div className="space-y-2">
          {dailyActivity.length === 0 ? (
            <div className="text-xs text-gray-500 py-4 text-center">No daily activity recorded</div>
          ) : (
            dailyActivity.map((day) => {
              const maxDay = Math.max(1, ...dailyActivity.map((d) => d.count));
              const pct = Math.round((day.count / maxDay) * 100);
              return (
                <div key={day.date} className="flex items-center space-x-3 text-xs font-mono">
                  <span className="theme-text-muted w-24 shrink-0">{day.date}</span>
                  <div className="theme-track flex-1 h-2 rounded-full overflow-hidden">
                    <div
                      className="h-full bg-emerald-500/80 rounded-full"
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
  );
};
