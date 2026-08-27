export interface AppUpdateStatus {
  configured: boolean;
  currentVersion: string;
  channel: 'stable' | 'prerelease';
}

export interface AvailableAppUpdate {
  currentVersion: string;
  channel: 'stable' | 'prerelease';
  available: boolean;
  version: string | null;
  notes: string | null;
  pubDate: string | null;
}
