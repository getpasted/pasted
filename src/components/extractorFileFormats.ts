export type ExtractorFileFormatGroup = 'any' | 'audio' | 'images' | 'video' | 'documents';

export const EXTRACTOR_FILE_FORMAT_GROUPS: ReadonlyArray<{
  id: ExtractorFileFormatGroup;
  formats: readonly string[];
}> = [
  { id: 'any', formats: ['*'] },
  { id: 'audio', formats: ['aac', 'flac', 'm4a', 'mp3', 'ogg', 'wav'] },
  { id: 'images', formats: ['bmp', 'gif', 'heif', 'jpg', 'png', 'tif', 'webp'] },
  { id: 'video', formats: ['avi', 'flv', 'm4v', 'mkv', 'mov', 'mp4', 'mpg', 'webm', 'wmv'] },
  { id: 'documents', formats: ['pdf'] },
];

export const EXTRACTOR_FILE_FORMAT_OPTIONS = EXTRACTOR_FILE_FORMAT_GROUPS.flatMap(({ formats }) => formats);
