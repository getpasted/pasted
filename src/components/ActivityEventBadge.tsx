import {
  Activity,
  ClipboardPaste,
  Database,
  Edit3,
  FileWarning,
  FolderInput,
  FolderMinus,
  History,
  Keyboard,
  ListOrdered,
  LockKeyhole,
  LogIn,
  LogOut,
  Pause,
  Pin,
  Play,
  Radar,
  Rocket,
  RotateCcw,
  Settings2,
  ShieldAlert,
  ShieldCheck,
  ShieldOff,
  Trash,
  Trash2,
  Workflow,
} from 'lucide-react';
import { translate } from '../localization/runtime';

export function ActivityEventBadge({ type, description }: { type: string; description: string }) {
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
      case 'bin_concealment_changed':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <FolderInput className="w-3.5 h-3.5" />
            <span>{translate('component.activityLogView.bins')}</span>
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
      case 'clip_name_updated':
        return (
          <div className="theme-status-info flex items-center space-x-1.5 px-2 py-0.5 rounded border text-[11px] font-semibold">
            <Edit3 className="w-3.5 h-3.5" />
            <span>{translate('common.name')}</span>
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
}
