import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
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
    <div className="flex-1 h-screen bg-[#171717] text-white p-6 overflow-y-auto font-sans select-none">
      {/* Header */}
      <div className="flex items-center justify-between pb-6 border-b border-gray-800 mb-6">
        <div>
          <h1 className="text-xl font-bold text-gray-100 flex items-center space-x-2">
            <BarChart3 className="w-5 h-5 text-cyan-400" />
            <span>Analytics & Insights</span>
          </h1>
          <p className="text-xs text-gray-400 mt-1">
            Telemetry, app source breakdowns, and clipboard storage efficiency metrics
          </p>
        </div>
      </div>

      {/* Top Stat Cards Grid */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        <div className="p-4 rounded-xl bg-[#212121] border border-gray-800 flex items-center space-x-4">
          <div className="p-3 rounded-lg bg-blue-500/10 border border-blue-500/20 text-blue-400">
            <Layers className="w-6 h-6" />
          </div>
          <div>
            <div className="text-2xl font-extrabold text-gray-100 font-mono">
              {totalClips.toLocaleString()}
            </div>
            <div className="text-xs text-gray-400 font-medium">Total Clips Saved</div>
          </div>
        </div>

        <div className="p-4 rounded-xl bg-[#212121] border border-gray-800 flex items-center space-x-4">
          <div className="p-3 rounded-lg bg-emerald-500/10 border border-emerald-500/20 text-emerald-400">
            <HardDrive className="w-6 h-6" />
          </div>
          <div>
            <div className="text-2xl font-extrabold text-gray-100 font-mono">
              {kbSaved} <span className="text-xs text-emerald-400">KB</span>
            </div>
            <div className="text-xs text-gray-400 font-medium">Storage Compressed</div>
          </div>
        </div>

        <div className="p-4 rounded-xl bg-[#212121] border border-gray-800 flex items-center space-x-4">
          <div className="p-3 rounded-lg bg-amber-500/10 border border-amber-500/20 text-amber-400">
            <TrendingUp className="w-6 h-6" />
          </div>
          <div>
            <div className="text-2xl font-extrabold text-gray-100 font-mono truncate max-w-[140px]">
              {topApps[0]?.name || 'VS Code'}
            </div>
            <div className="text-xs text-gray-400 font-medium">Top Copied App</div>
          </div>
        </div>
      </div>

      {/* Detailed Insights Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
        {/* Top Applications */}
        <div className="p-5 rounded-xl bg-[#212121] border border-gray-800 flex flex-col">
          <h2 className="text-sm font-bold text-gray-200 mb-4 flex items-center space-x-2">
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
                      <span className="text-gray-300 font-medium">{app.name}</span>
                      <span className="text-gray-400">{app.count} clips ({pct}%)</span>
                    </div>
                    <div className="w-full h-2 bg-gray-900 rounded-full overflow-hidden">
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
        <div className="p-5 rounded-xl bg-[#212121] border border-gray-800 flex flex-col">
          <h2 className="text-sm font-bold text-gray-200 mb-4 flex items-center space-x-2">
            <PieChart className="w-4 h-4 text-purple-400" />
            <span>Content Type Breakdown</span>
          </h2>
          <div className="grid grid-cols-2 gap-3 flex-1">
            <div className="p-3 rounded-lg bg-gray-900/80 border border-gray-800 flex items-center space-x-3">
              <FileText className="w-4 h-4 text-gray-400 shrink-0" />
              <div>
                <div className="text-sm font-bold text-white font-mono">{getTypeCount('text')}</div>
                <div className="text-[11px] text-gray-400">Plain Text</div>
              </div>
            </div>
            <div className="p-3 rounded-lg bg-gray-900/80 border border-gray-800 flex items-center space-x-3">
              <Code className="w-4 h-4 text-emerald-400 shrink-0" />
              <div>
                <div className="text-sm font-bold text-white font-mono">{getTypeCount('code')}</div>
                <div className="text-[11px] text-gray-400">Code Snippets</div>
              </div>
            </div>
            <div className="p-3 rounded-lg bg-gray-900/80 border border-gray-800 flex items-center space-x-3">
              <LinkIcon className="w-4 h-4 text-blue-400 shrink-0" />
              <div>
                <div className="text-sm font-bold text-white font-mono">{getTypeCount('link')}</div>
                <div className="text-[11px] text-gray-400">Links / URLs</div>
              </div>
            </div>
            <div className="p-3 rounded-lg bg-gray-900/80 border border-gray-800 flex items-center space-x-3">
              <ImageIcon className="w-4 h-4 text-pink-400 shrink-0" />
              <div>
                <div className="text-sm font-bold text-white font-mono">{getTypeCount('image')}</div>
                <div className="text-[11px] text-gray-400">Images</div>
              </div>
            </div>
            <div className="p-3 rounded-lg bg-gray-900/80 border border-gray-800 flex items-center space-x-3 col-span-2">
              <Palette className="w-4 h-4 text-amber-400 shrink-0" />
              <div>
                <div className="text-sm font-bold text-white font-mono">{getTypeCount('color')}</div>
                <div className="text-[11px] text-gray-400">Color Swatches</div>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Daily Activity Timeline */}
      <div className="p-5 rounded-xl bg-[#212121] border border-gray-800">
        <h2 className="text-sm font-bold text-gray-200 mb-4 flex items-center space-x-2">
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
                  <span className="text-gray-400 w-24 shrink-0">{day.date}</span>
                  <div className="flex-1 h-2 bg-gray-900 rounded-full overflow-hidden">
                    <div
                      className="h-full bg-emerald-500/80 rounded-full"
                      style={{ width: `${pct}%` }}
                    />
                  </div>
                  <span className="text-gray-300 font-bold w-12 text-right">{day.count}</span>
                </div>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
};
