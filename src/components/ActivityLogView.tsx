import React, { useState, useEffect } from 'react';
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
} from 'lucide-react';
import { ToolPageHeader } from './ToolPageHeader';
import { MenuSelect } from './MenuSelect';

export interface ActivityLog {
  id: number;
  event_type: string;
  description: string;
  created_at: string;
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

  const getEventBadge = (type: string, description: string) => {
    switch (type) {
      case 'recording_manually_paused':
        return (
          <div className="theme-status-warning flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Pause className="w-3.5 h-3.5" />
            <span>Manually Paused</span>
          </div>
        );
      case 'recording_manually_resumed':
        return (
          <div className="theme-status-success flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Play className="w-3.5 h-3.5" />
            <span>Manually Resumed</span>
          </div>
        );
      case 'recording_auto_paused':
        return (
          <div className="theme-status-warning flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <ShieldAlert className="w-3.5 h-3.5" />
            <span>Auto-Paused</span>
          </div>
        );
      case 'recording_auto_resumed':
        return (
          <div className="theme-status-success flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <ShieldCheck className="w-3.5 h-3.5" />
            <span>Auto-Resumed</span>
          </div>
        );
      case 'clip_trashed':
      case 'clip_auto_trashed':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Trash2 className="w-3.5 h-3.5" />
            <span>{type === 'clip_auto_trashed' ? 'Auto-Trashed' : 'Trashed'}</span>
          </div>
        );
      case 'clips_trashed_all':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Trash2 className="w-3.5 h-3.5" />
            <span>Trashed All</span>
          </div>
        );
      case 'clip_restored':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <RotateCcw className="w-3.5 h-3.5" />
            <span>Restored</span>
          </div>
        );
      case 'clip_revision_restored':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <History className="w-3.5 h-3.5" />
            <span>Revision Restored</span>
          </div>
        );
      case 'bin_deleted':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <FolderMinus className="w-3.5 h-3.5" />
            <span>Bin Deleted</span>
          </div>
        );
      case 'trash_emptied':
      case 'clip_deleted':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Trash className="w-3.5 h-3.5" />
            <span>Purged</span>
          </div>
        );
      case 'clips_purged_all':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Trash className="w-3.5 h-3.5" />
            <span>Purged All</span>
          </div>
        );
      case 'clip_protected_toggled': {
        const isProtected = description.startsWith('Protected ');
        return (
          <div className={`flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold ${
            isProtected
              ? 'theme-status-info'
              : 'theme-badge'
          }`}>
            {isProtected ? <ShieldCheck className="w-3.5 h-3.5" /> : <ShieldOff className="w-3.5 h-3.5" />}
            <span>{isProtected ? 'Protected' : 'Unprotected'}</span>
          </div>
        );
      }
      case 'note_updated':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Edit3 className="w-3.5 h-3.5" />
            <span>Note</span>
          </div>
        );
      case 'transform_drafted':
      case 'transform_tested':
      case 'transform_saved':
      case 'transform_updated':
      case 'transform_executed':
      case 'bin_transform_executed':
      case 'bin_transform_no_change':
      case 'clip_transformed':
        return (
          <div className="theme-status-success flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Workflow className="w-3.5 h-3.5" />
            <span>{type === 'transform_drafted' ? 'Drafted' : type === 'transform_tested' ? 'Tested' : type === 'transform_saved' ? 'Saved' : type === 'transform_updated' ? 'Updated' : type === 'bin_transform_no_change' ? 'No Change' : 'Transformed'}</span>
          </div>
        );
      case 'transform_draft_failed':
      case 'transform_test_failed':
      case 'transform_execution_failed':
      case 'bin_transform_failed':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Workflow className="w-3.5 h-3.5" />
            <span>Transform Failed</span>
          </div>
        );
      case 'transform_execution_cancelled':
      case 'transform_draft_cancelled':
      case 'transform_test_cancelled':
        return (
          <div className="theme-badge flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Workflow className="w-3.5 h-3.5" />
            <span>{type === 'transform_draft_cancelled' ? 'Draft Cancelled' : type === 'transform_test_cancelled' ? 'Test Cancelled' : 'Transform Cancelled'}</span>
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
    if (selectedTypeFilter === 'trashed') return l.event_type === 'clip_trashed' || l.event_type === 'clip_auto_trashed' || l.event_type === 'clips_trashed_all';
    if (selectedTypeFilter === 'restored') return l.event_type === 'clip_restored';
    if (selectedTypeFilter === 'revisions') return l.event_type === 'clip_revision_restored';
    if (selectedTypeFilter === 'purged') return l.event_type === 'clip_deleted' || l.event_type === 'trash_emptied' || l.event_type === 'clips_purged_all';
    if (selectedTypeFilter === 'protection') return l.event_type === 'clip_protected_toggled';
    if (selectedTypeFilter === 'paused') return l.event_type === 'recording_auto_paused' || l.event_type === 'recording_manually_paused';
    if (selectedTypeFilter === 'resumed') return l.event_type === 'recording_auto_resumed' || l.event_type === 'recording_manually_resumed';
    if (selectedTypeFilter === 'notes') return l.event_type === 'note_updated';
    if (selectedTypeFilter === 'transforms') return l.event_type.startsWith('transform_') || l.event_type.startsWith('bin_transform_') || l.event_type === 'clip_transformed';
    return true;
  });

  return (
    <div className="tools-page activity-page flex-1 font-sans h-screen flex flex-col overflow-hidden">
      <ToolPageHeader
        icon={<Activity className="w-4 h-4" />}
        title="Activity Log"
        actions={(
          <div className="flex items-center space-x-2.5">
          <MenuSelect
            value={selectedTypeFilter}
            onChange={setSelectedTypeFilter}
            label="Filter Activity"
            leadingIcon={<ListFilter className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />}
            className="min-w-44"
            options={[
              { value: 'all', label: 'All Event Types' },
              { value: 'trashed', label: 'Trashed' },
              { value: 'restored', label: 'Restored from Trash' },
              { value: 'revisions', label: 'Revision Restored' },
              { value: 'purged', label: 'Permanently Deleted' },
              { value: 'protection', label: 'Protection Changed' },
              { value: 'paused', label: 'Recording Paused' },
              { value: 'resumed', label: 'Recording Resumed' },
              { value: 'notes', label: 'Notes Updated' },
              { value: 'transforms', label: 'Transforms' },
            ]}
          />

          <div className="relative">
            <Search className="theme-text-muted w-3.5 h-3.5 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              placeholder="Search activity..."
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              className="theme-input border rounded-xl pl-8 pr-3 py-1.5 text-xs focus:outline-none w-44"
            />
          </div>

          <button
            onClick={handleClearLogs}
            disabled={logs.length === 0}
            className="theme-secondary-button flex items-center space-x-1.5 px-3 py-1.5 disabled:opacity-40 border rounded-xl text-xs font-semibold transition-[background-color,border-color,color,opacity] cursor-pointer"
          >
            <Trash2 className="w-3.5 h-3.5" />
            <span>Clear Log</span>
          </button>
          </div>
        )}
      />

      {/* Timeline Content List */}
      <div className="tools-scroll-region flex-1 overflow-y-auto p-6 space-y-3">
        {filteredLogs.length === 0 ? (
          <div className="theme-text-subtle h-full flex flex-col items-center justify-center space-y-2">
            <Activity className="w-10 h-10 opacity-30" />
            <p className="text-xs font-medium">No activity recorded yet.</p>
          </div>
        ) : (
          filteredLogs.map((log) => (
            <div
              key={log.id}
              className="theme-panel border rounded-xl p-3.5 flex items-center justify-between transition-colors"
            >
              <div className="flex items-center space-x-3.5 min-w-0 flex-1 pr-4">
                {getEventBadge(log.event_type, log.description)}
                <span className="theme-text-main text-xs truncate font-medium">
                  {log.description}
                </span>
              </div>

              <span className="theme-text-muted text-[11px] font-mono shrink-0">
                {new Date(log.created_at).toLocaleTimeString([], {
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
