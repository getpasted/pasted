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
} from 'lucide-react';
import { ToolPageHeader } from './ToolPageHeader';
import { MenuSelect } from './MenuSelect';
import { OverflowText } from './OverflowText';

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
      case 'app_started':
        return (
          <div className="theme-status-success flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <LogIn className="w-3.5 h-3.5" />
            <span>App Opened</span>
          </div>
        );
      case 'app_exit_requested':
        return (
          <div className="theme-badge flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <LogOut className="w-3.5 h-3.5" />
            <span>App Quit</span>
          </div>
        );
      case 'setting_changed':
      case 'settings_changed':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Settings2 className="w-3.5 h-3.5" />
            <span>Settings</span>
          </div>
        );
      case 'content_detector_created':
      case 'content_detector_updated':
      case 'content_detector_deleted':
      case 'content_detectors_restored':
      case 'content_detection_history_rescanned':
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
            <span>Detection</span>
          </div>
        );
      case 'autostart_enabled':
      case 'autostart_disabled':
        return (
          <div className={`${type === 'autostart_enabled' ? 'theme-status-success' : 'theme-badge'} flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold`}>
            <Rocket className="w-3.5 h-3.5" />
            <span>{type === 'autostart_enabled' ? 'Login Start On' : 'Login Start Off'}</span>
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
      case 'clips_trashed':
      case 'clip_auto_trashed':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Trash2 className="w-3.5 h-3.5" />
            <span>{type === 'clip_auto_trashed' ? 'Auto-Trashed' : type === 'clips_trashed' ? 'Batch Trashed' : 'Trashed'}</span>
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
      case 'library_moved':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Database className="w-3.5 h-3.5" />
            <span>Library Moved</span>
          </div>
        );
      case 'bin_deleted':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <FolderMinus className="w-3.5 h-3.5" />
            <span>Bin Deleted</span>
          </div>
        );
      case 'bin_clips_reordered':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <ListOrdered className="w-3.5 h-3.5" />
            <span>Bin Reordered</span>
          </div>
        );
      case 'clip_bin_assigned':
      case 'clips_bin_assigned':
      case 'clip_bin_unassigned':
      case 'clips_bin_unassigned':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <FolderInput className="w-3.5 h-3.5" />
            <span>{type.includes('unassigned') ? 'Bin Removed' : 'Bin Assigned'}</span>
          </div>
        );
      case 'clipboard_capture_ignored':
        return (
          <div className="theme-status-warning flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <FileWarning className="w-3.5 h-3.5" />
            <span>Capture Skipped</span>
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
            <span>{type === 'queue_reordered' ? 'Queue Reordered' : type === 'queue_item_removed' ? 'Queue Removed' : type === 'queue_recording_started' ? 'Queue Recording' : type === 'queue_recording_stopped' ? 'Queue Stopped' : 'Queued'}</span>
          </div>
        );
      case 'queue_item_pasted':
      case 'queue_all_pasted':
        return (
          <div className="theme-status-success flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <ListOrdered className="w-3.5 h-3.5" />
            <span>Queue Pasted</span>
          </div>
        );
      case 'queue_paste_failed':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <ListOrdered className="w-3.5 h-3.5" />
            <span>Queue Failed</span>
          </div>
        );
      case 'hud_clip_pasted':
        return (
          <div className="theme-status-success flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <ClipboardPaste className="w-3.5 h-3.5" />
            <span>HUD Pasted</span>
          </div>
        );
      case 'hud_paste_failed':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <ClipboardPaste className="w-3.5 h-3.5" />
            <span>HUD Failed</span>
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
            <span>{isProtected ? 'Protected' : 'Unprotected'}</span>
          </div>
        );
      }
      case 'clip_pinned':
      case 'clips_pinned':
      case 'clip_unpinned':
      case 'clips_unpinned': {
        const isPinned = !type.includes('unpinned');
        return (
          <div className={`${isPinned ? 'theme-status-warning' : 'theme-badge'} flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold`}>
            <Pin className="w-3.5 h-3.5" />
            <span>{isPinned ? 'Pinned' : 'Unpinned'}</span>
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
      case 'transformation_execution_succeeded':
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
      case 'transformation_execution_failed':
      case 'bin_transform_failed':
        return (
          <div className="theme-status-danger flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Workflow className="w-3.5 h-3.5" />
            <span>Transform Failed</span>
          </div>
        );
      case 'transform_execution_cancelled':
      case 'transformation_execution_cancelled':
      case 'transform_draft_cancelled':
      case 'transform_test_cancelled':
        return (
          <div className="theme-badge flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Workflow className="w-3.5 h-3.5" />
            <span>{type === 'transform_draft_cancelled' ? 'Draft Cancelled' : type === 'transform_test_cancelled' ? 'Test Cancelled' : 'Transform Cancelled'}</span>
          </div>
        );
      case 'intelligence_connection_fallback':
        return (
          <div className="theme-status-warning flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Workflow className="w-3.5 h-3.5" />
            <span>Connection Fallback</span>
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
    if (selectedTypeFilter === 'restored') return l.event_type === 'clip_restored';
    if (selectedTypeFilter === 'revisions') return l.event_type === 'clip_revision_restored';
    if (selectedTypeFilter === 'purged') return l.event_type === 'clip_deleted' || l.event_type === 'trash_emptied' || l.event_type === 'clips_purged_all';
    if (selectedTypeFilter === 'protection') return l.event_type === 'clip_protected_toggled' || l.event_type === 'clips_protected_toggled';
    if (selectedTypeFilter === 'pinning') return l.event_type.includes('pinned');
    if (selectedTypeFilter === 'paused') return l.event_type === 'recording_auto_paused' || l.event_type === 'recording_manually_paused';
    if (selectedTypeFilter === 'resumed') return l.event_type === 'recording_auto_resumed' || l.event_type === 'recording_manually_resumed';
    if (selectedTypeFilter === 'notes') return l.event_type === 'note_updated';
    if (selectedTypeFilter === 'skipped') return l.event_type === 'clipboard_capture_ignored';
    if (selectedTypeFilter === 'transforms') return l.event_type.startsWith('transform_') || l.event_type.startsWith('transformation_') || l.event_type.startsWith('bin_transform_') || l.event_type === 'clip_transformed' || l.event_type === 'intelligence_connection_fallback';
    if (selectedTypeFilter === 'queue') return l.event_type.startsWith('queue_');
    if (selectedTypeFilter === 'hud') return l.event_type.startsWith('hud_');
    if (selectedTypeFilter === 'bins') return l.event_type.startsWith('bin_') || l.event_type.includes('_bin_');
    if (selectedTypeFilter === 'app') return l.event_type.startsWith('app_');
    if (selectedTypeFilter === 'settings') return l.event_type.startsWith('setting_') || l.event_type.startsWith('settings_') || l.event_type.startsWith('autostart_');
    if (selectedTypeFilter === 'storage') return l.event_type.startsWith('library_');
    if (selectedTypeFilter === 'detection') return l.event_type.startsWith('content_detector') || l.event_type.startsWith('content_detection') || l.event_type.startsWith('content_type');
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
              { value: 'app', label: 'App Opened or Quit', group: 'Application' },
              { value: 'settings', label: 'Settings Changed', group: 'Application' },
              { value: 'detection', label: 'Detection Changed', group: 'Application' },
              { value: 'storage', label: 'Storage Changed', group: 'Application' },
              { value: 'paused', label: 'Recording Paused', group: 'Capture' },
              { value: 'resumed', label: 'Recording Resumed', group: 'Capture' },
              { value: 'skipped', label: 'Skipped Captures', group: 'Capture' },
              { value: 'trashed', label: 'Trashed', group: 'History' },
              { value: 'restored', label: 'Restored from Trash', group: 'History' },
              { value: 'revisions', label: 'Revision Restored', group: 'History' },
              { value: 'purged', label: 'Permanently Deleted', group: 'History' },
              { value: 'protection', label: 'Protection Changed', group: 'Organization' },
              { value: 'pinning', label: 'Pinning Changed', group: 'Organization' },
              { value: 'notes', label: 'Notes Updated', group: 'Organization' },
              { value: 'bins', label: 'Bins', group: 'Organization' },
              { value: 'transforms', label: 'Transforms', group: 'Automation' },
              { value: 'queue', label: 'Copy Queue', group: 'Automation' },
              { value: 'hud', label: 'Quick HUD', group: 'Automation' },
            ]}
          />

          <div className="relative">
            <Search className="theme-text-muted w-3.5 h-3.5 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              placeholder="Search activity..."
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              className="theme-input ui-field-radius border pl-8 pr-3 py-1.5 text-xs focus:outline-none w-44"
            />
          </div>

          <button
            onClick={handleClearLogs}
            disabled={logs.length === 0}
            className="theme-secondary-button ui-control-radius flex h-[34px] items-center space-x-1.5 px-3 disabled:opacity-40 border text-xs font-semibold transition-[background-color,border-color,color,opacity] cursor-pointer"
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
                <OverflowText text={log.description} className="theme-text-main text-xs truncate font-medium" />
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
