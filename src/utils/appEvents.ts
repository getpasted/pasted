export { APP_EVENTS } from './appEvents.generated';

export interface AppSettingChangedEvent {
  key: string;
  value: string;
}

export interface ClipboardPauseChangedEvent {
  isPaused: boolean;
  autoPausedBy: string | null;
}

export interface ClipLibraryChangedEvent {
  clipIds: number[];
}
