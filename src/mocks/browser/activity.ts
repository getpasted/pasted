import { handled, unhandled, type BrowserMockResult } from './result';

export function handleActivityBrowserMock(command: string): BrowserMockResult {
  switch (command) {
    case 'get_activity_logs':
      return handled([]);
    case 'export_activity_json':
      return handled(JSON.stringify({ schemaVersion: 1, exportedAt: new Date().toISOString(), resource: { 'service.name': 'Pasted' }, entries: [] }, null, 2));
    case 'export_activity_csv':
      return handled('timestamp,observed_timestamp,event_name,severity_text,body,category,outcome,attributes_json\n');
    case 'clear_activity_logs':
      return handled(undefined);
    default:
      return unhandled;
  }
}
