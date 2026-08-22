export interface FileClipPreview {
  index: number;
  dataUrl: string | null;
  textContent: string | null;
  width: number | null;
  height: number | null;
  availability: 'available' | 'missing' | 'inaccessible' | 'unavailable';
  cached: boolean;
}
