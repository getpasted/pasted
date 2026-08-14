export const MAX_RENDERABLE_RASTER_DATA_URL_BYTES = 192 * 1024 * 1024;

export type SafeRasterImage = {
  bytes: Uint8Array;
  mimeType: 'image/png' | 'image/jpeg' | 'image/webp';
};

function hasPngSignature(bytes: Uint8Array): boolean {
  const signature = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
  return bytes.length >= signature.length
    && signature.every((value, index) => bytes[index] === value);
}

function hasJpegSignature(bytes: Uint8Array): boolean {
  return bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff;
}

function hasWebpSignature(bytes: Uint8Array): boolean {
  return bytes.length >= 12
    && String.fromCharCode(...bytes.subarray(0, 4)) === 'RIFF'
    && String.fromCharCode(...bytes.subarray(8, 12)) === 'WEBP';
}

export function decodeSafeRasterDataUrl(value: unknown): SafeRasterImage | null {
  if (typeof value !== 'string' || value.length === 0 || value.length > MAX_RENDERABLE_RASTER_DATA_URL_BYTES) {
    return null;
  }
  const separator = value.indexOf(',');
  if (separator < 0) return null;
  const header = value.slice(0, separator);
  const payload = value.slice(separator + 1);
  const mimeType = header === 'data:image/png;base64'
    ? 'image/png'
    : header === 'data:image/jpeg;base64' || header === 'data:image/jpg;base64'
      ? 'image/jpeg'
      : header === 'data:image/webp;base64'
        ? 'image/webp'
        : null;
  if (!mimeType || payload.length === 0 || payload.length % 4 !== 0 || !/^[A-Za-z0-9+/]+={0,2}$/.test(payload)) {
    return null;
  }
  try {
    const decoded = atob(payload);
    const bytes = Uint8Array.from(decoded, (character) => character.charCodeAt(0));
    const signatureMatches = mimeType === 'image/png'
      ? hasPngSignature(bytes)
      : mimeType === 'image/jpeg'
        ? hasJpegSignature(bytes)
        : hasWebpSignature(bytes);
    return signatureMatches ? { bytes, mimeType } : null;
  } catch {
    return null;
  }
}
