import { safeInvoke as invoke } from '../utils/tauri';

export interface ActivityLog {
  id: number;
  event_type: string;
  description: string;
  created_at: string;
  observed_at: string;
  severity_text: 'info' | 'warn' | 'error';
  category: string;
  outcome: 'success' | 'failure' | 'unknown';
  attributes: Record<string, unknown>;
}

export const activityApi = {
  list: (limit: number, offset: number) => invoke<ActivityLog[]>('get_activity_logs', { limit, offset }),
  clear: () => invoke<void>('clear_activity_logs'),
  exportJson: () => invoke<string>('export_activity_json'),
  exportCsv: () => invoke<string>('export_activity_csv'),
};
