import { scheduleBackupClientStatePersistence } from './backupClientState';
import { DEFAULT_APP_UI_STATE, parseAppUiState, type AppUiState } from './appUiStateCodec';

export * from './appUiStateCodec';

export const APP_UI_STATE_KEY = 'pasted_app_ui_state';

export function readAppUiState(): AppUiState {
  try {
    const saved = localStorage.getItem(APP_UI_STATE_KEY);
    return saved ? parseAppUiState(JSON.parse(saved)) : DEFAULT_APP_UI_STATE;
  } catch {
    return DEFAULT_APP_UI_STATE;
  }
}

export function writeAppUiState(state: AppUiState) {
  try {
    localStorage.setItem(APP_UI_STATE_KEY, JSON.stringify(state));
    scheduleBackupClientStatePersistence();
  } catch {
    // UI state is best-effort; the database and clipboard library remain authoritative.
  }
}
