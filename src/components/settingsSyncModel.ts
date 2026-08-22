export interface LibraryLocationInfo {
  path: string;
  directory: string;
  isDefault: boolean;
}

export interface LibraryMoveReport {
  location: LibraryLocationInfo;
  recoveryPath: string;
}

export interface StorageProtectionInfo {
  status: 'protected' | 'notDetected' | 'unknown';
  technology: string | null;
  summary: string;
  detail: string;
}

interface LibraryArchiveInspection {
  schemaVersion: number;
  clipCount: number;
  binCount: number;
  operationCount: number;
  transformCount: number;
  classifierCount: number;
  contentTypeCount: number;
}

export type ImportKind = 'clips' | 'activity' | 'organization' | 'backup';

interface ActivityImportReport {
  scannedCount: number;
  importedCount: number;
  duplicateCount: number;
  retainedCount: number;
}

interface ClipImportReport {
  scannedCount: number;
  importedCount: number;
  duplicateCount: number;
}

export interface ImportFileInspection {
  path: string;
  name: string;
  kind: ImportKind;
  format: 'json' | 'csv' | 'backup';
  sizeBytes: number;
  report?: ClipImportReport | ActivityImportReport;
  library?: LibraryArchiveInspection;
  backup?: {
    formatVersion: number;
    createdAt: string;
    sizeBytes: number;
  };
}

export type ExportMode = 'custom' | 'full';
export type ExportFormat = 'json' | 'csv';
export type VisibleExportFormat = ExportFormat | 'backup';
export type ExportDataId = 'clips' | 'organization' | 'activity' | 'settings' | 'recovery' | 'interface';
