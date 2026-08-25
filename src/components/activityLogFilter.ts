import type { ActivityLog } from '../api/activity';

const hasPrefix = (eventType: string, prefixes: string[]) =>
  prefixes.some((prefix) => eventType.startsWith(prefix));

export function activityLogMatches(log: ActivityLog, query: string, typeFilter: string): boolean {
  const eventType = log.event_type;
  const normalizedQuery = query.toLowerCase();
  if (!log.description.toLowerCase().includes(normalizedQuery)
    && !eventType.toLowerCase().includes(normalizedQuery)) return false;

  switch (typeFilter) {
    case 'all': return true;
    case 'trashed': return ['clip_trashed', 'clips_trashed', 'clip_auto_trashed', 'clips_trashed_all'].includes(eventType);
    case 'restored': return ['clip_restored', 'clips_restored_all'].includes(eventType);
    case 'revisions': return ['clip_revision_restored', 'clip_version_deleted'].includes(eventType);
    case 'purged': return ['clip_deleted', 'trash_emptied', 'clips_purged_all'].includes(eventType);
    case 'protection': return ['clip_protected_toggled', 'clips_protected_toggled', 'clip_hotkey_changed', 'bin_protection_changed'].includes(eventType);
    case 'pinning': return eventType.includes('pinned');
    case 'paused': return ['recording_auto_paused', 'recording_manually_paused'].includes(eventType);
    case 'resumed': return ['recording_auto_resumed', 'recording_manually_resumed'].includes(eventType);
    case 'notes': return eventType === 'note_updated';
    case 'names': return eventType === 'clip_name_updated';
    case 'skipped': return eventType === 'clipboard_capture_ignored';
    case 'transforms': return hasPrefix(eventType, ['transform_', 'transformation_', 'bin_transform_', 'operation_', 'pipeline_'])
      || ['library_item_enabled_changed', 'clip_transformed', 'intelligence_connection_fallback'].includes(eventType);
    case 'queue': return eventType.startsWith('queue_');
    case 'hud': return eventType.startsWith('hud_');
    case 'bins': return eventType.startsWith('bin_') || eventType.includes('_bin_');
    case 'app': return eventType.startsWith('app_');
    case 'settings': return hasPrefix(eventType, ['setting_', 'settings_', 'autostart_']);
    case 'analysis': return hasPrefix(eventType, ['content_classifier', 'content_classification', 'content_detector', 'content_detection', 'content_extractor', 'content_type', 'file_format']);
    case 'storage': return hasPrefix(eventType, ['library_', 'backup_', 'data_export_'])
      || ['external_history_imported', 'clips_imported'].includes(eventType);
    default: return true;
  }
}
