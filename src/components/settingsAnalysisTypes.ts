export interface SettingsAnalysisPanelProps {
  contentClassificationEnabled: boolean;
  fileFormatsEnabled: boolean;
  ocrEnabled: boolean;
  transcriptionsEnabled: boolean;
  transformationsEnabled: boolean;
  typesEnabled: boolean;
  sourcesEnabled: boolean;
  searchEnabled: boolean;
  onOpenIntelligence?: () => void;
  onSearchClips: (clipIds: number[]) => void;
}
