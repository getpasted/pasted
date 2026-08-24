import { settingDefault } from './settingsContract.ts';

export const DEFAULT_REVISION_HISTORY_LIMIT = settingDefault('revisionHistoryLimit');
export const DEFAULT_ANALYSIS_ATTEMPTS_PER_CLIP = settingDefault('analysisAttemptsPerClip');

export function isAnalysisFunctionalityEnabled(
  settings: { enableOcr: boolean; enableTranscriptions: boolean },
): boolean {
  return settings.enableOcr || settings.enableTranscriptions;
}

export function storedRetentionNumber(
  saved: Record<string, string>,
  key: string,
  fallback: number,
): number {
  const value = Number(saved[key]);
  return Number.isFinite(value) ? value : fallback;
}
