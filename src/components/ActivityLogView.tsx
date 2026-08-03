import React, { useState, useEffect } from 'react';
import { safeInvoke as invoke } from '../utils/tauri';
import {
  Activity,
  Trash2,
  RotateCcw,
  ShieldAlert,
  ShieldCheck,
  Edit3,
  Trash,
  Search,
  Pause,
  Play,
} from 'lucide-react';

export interface ActivityLog {
  id: number;
  event_type: String;
  description: String;
  created_at: String;
}

export const ActivityLogView: React.FC = () => {
  const [logs, setLogs] = useState<ActivityLog[]>([]);
  const [filter, setFilter] = useState('');

  const fetchLogs = async () => {
    try {
      const res = await invoke<ActivityLog[]>('get_activity_logs', { limit: 200, offset: 0 });
      setLogs(res);
    } catch (e) {
      console.error('Failed to fetch activity logs:', e);
    }
  };

  useEffect(() => {
    fetchLogs();

    const interval = setInterval(() => {
      fetchLogs();
    }, 5000);

    return () => {
      clearInterval(interval);
    };
  }, []);

  const handleClearLogs = async () => {
    try {
      await invoke('clear_activity_logs');
      setLogs([]);
    } catch (e) {
      console.error('Failed to clear logs:', e);
    }
  };

  const getEventBadge = (type: string) => {
    switch (type) {
      case 'recording_manually_paused':
        return (
          <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded bg-orange-500/20 text-orange-400 border border-orange-500/30 text-[11px] font-semibold">
            <Pause className="w-3.5 h-3.5" />
            <span>Manually Paused</span>
          </div>
        );
      case 'recording_manually_resumed':
        return (
          <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 text-[11px] font-semibold">
            <Play className="w-3.5 h-3.5" />
            <span>Manually Resumed</span>
          </div>
        );
      case 'recording_auto_paused':
        return (
          <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded bg-amber-500/20 text-amber-400 border border-amber-500/30 text-[11px] font-semibold">
            <ShieldAlert className="w-3.5 h-3.5" />
            <span>Auto-Paused</span>
          </div>
        );
      case 'recording_auto_resumed':
        return (
          <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded bg-green-500/20 text-green-400 border border-green-500/30 text-[11px] font-semibold">
            <ShieldCheck className="w-3.5 h-3.5" />
            <span>Auto-Resumed</span>
          </div>
        );
      case 'clip_trashed':
        return (
          <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded bg-rose-500/20 text-rose-400 border border-rose-500/30 text-[11px] font-semibold">
            <Trash2 className="w-3.5 h-3.5" />
            <span>Trashed</span>
          </div>
        );
      case 'clip_restored':
        return (
          <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded bg-cyan-500/20 text-cyan-400 border border-cyan-500/30 text-[11px] font-semibold">
            <RotateCcw className="w-3.5 h-3.5" />
            <span>Restored</span>
          </div>
        );
      case 'trash_emptied':
      case 'clip_deleted':
        return (
          <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded bg-red-600/20 text-red-400 border border-red-500/30 text-[11px] font-semibold">
            <Trash className="w-3.5 h-3.5" />
            <span>Purged</span>
          </div>
        );
      case 'note_updated':
        return (
          <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded bg-purple-500/20 text-purple-300 border border-purple-500/30 text-[11px] font-semibold">
            <Edit3 className="w-3.5 h-3.5" />
            <span>Note</span>
          </div>
        );
      default:
        return (
          <div className="flex items-center space-x-1.5 px-2 py-0.5 rounded bg-gray-700/50 text-gray-300 border border-gray-600 text-[11px] font-semibold">
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
    if (selectedTypeFilter === 'trashed') return l.event_type === 'clip_trashed';
    if (selectedTypeFilter === 'restored') return l.event_type === 'clip_restored';
    if (selectedTypeFilter === 'purged') return l.event_type === 'clip_deleted' || l.event_type === 'trash_emptied';
    if (selectedTypeFilter === 'paused') return l.event_type === 'recording_auto_paused' || l.event_type === 'recording_manually_paused';
    if (selectedTypeFilter === 'resumed') return l.event_type === 'recording_auto_resumed' || l.event_type === 'recording_manually_resumed';
    if (selectedTypeFilter === 'notes') return l.event_type === 'note_updated';
    return true;
  });

  return (
    <div className="tools-page activity-page flex-1 bg-[#141414] text-gray-100 font-sans h-screen flex flex-col overflow-hidden">
      {/* Header Bar */}
      <div className="h-[60px] border-b border-[#2b2b2b] bg-[#171717]/80 backdrop-blur-md px-6 flex items-center justify-between shrink-0 no-drag">
        <div className="flex items-center space-x-2.5">
          <Activity className="w-5 h-5 text-cyan-400" />
          <h2 className="text-sm font-bold text-gray-100 uppercase tracking-wider">
            Activity Log
          </h2>
        </div>

        <div className="flex items-center space-x-2.5">
          {/* Event Type Filter Selector */}
          <select
            value={selectedTypeFilter}
            onChange={(e) => setSelectedTypeFilter(e.target.value)}
            className="bg-[#212121] border border-gray-700 rounded-xl px-3 py-1.5 text-xs text-gray-200 focus:outline-none focus:border-cyan-500 font-medium"
          >
            <option value="all">All Event Types</option>
            <option value="trashed">Trashed</option>
            <option value="restored">Restored</option>
            <option value="purged">Purged / Permanently Deleted</option>
            <option value="paused">Auto-Paused</option>
            <option value="resumed">Auto-Resumed</option>
            <option value="notes">Notes Updated</option>
          </select>

          <div className="relative">
            <Search className="w-3.5 h-3.5 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
            <input
              type="text"
              placeholder="Search activity..."
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              className="bg-[#212121] border border-gray-700 rounded-xl pl-8 pr-3 py-1.5 text-xs text-gray-200 focus:outline-none focus:border-cyan-500 w-44"
            />
          </div>

          <button
            onClick={handleClearLogs}
            disabled={logs.length === 0}
            className="flex items-center space-x-1.5 px-3 py-1.5 bg-gray-800 hover:bg-gray-700 disabled:opacity-40 text-gray-200 border border-gray-600 rounded-xl text-xs font-semibold transition-all cursor-pointer"
          >
            <Trash2 className="w-3.5 h-3.5" />
            <span>Clear Log</span>
          </button>
        </div>
      </div>

      {/* Timeline Content List */}
      <div className="flex-1 overflow-y-auto p-6 space-y-3">
        {filteredLogs.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center text-gray-500 space-y-2">
            <Activity className="w-10 h-10 opacity-30" />
            <p className="text-xs font-medium">No activity recorded yet.</p>
          </div>
        ) : (
          filteredLogs.map((log) => (
            <div
              key={log.id}
              className="bg-[#212121] border border-gray-800 rounded-xl p-3.5 flex items-center justify-between hover:border-gray-700 transition-colors shadow-sm"
            >
              <div className="flex items-center space-x-3.5 min-w-0 flex-1 pr-4">
                {getEventBadge(log.event_type as string)}
                <span className="text-xs text-gray-200 truncate font-medium">
                  {log.description}
                </span>
              </div>

              <span className="text-[11px] font-mono text-gray-400 shrink-0">
                {new Date(log.created_at as string).toLocaleTimeString([], {
                  hour: '2-digit',
                  minute: '2-digit',
                  second: '2-digit',
                })}
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  );
};
