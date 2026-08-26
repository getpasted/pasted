export interface RetentionSettings {
  keepClipCount: number;
  keepClipAgeDays: number;
  revisionHistoryLimit: number;
  analysisAttemptsPerClip: number;
  activityLogCapacity: number;
  activityLogAgeDays: number;
  searchHistoryLimit: number;
  searchHistoryAgeDays: number;
  trashCapacityCount: number;
  trashAgeDays: number;
}
