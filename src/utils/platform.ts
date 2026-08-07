export type DesktopPlatform = 'macos' | 'windows' | 'linux' | 'unknown';

export const detectDesktopPlatform = (
  platform = typeof navigator === 'undefined' ? '' : navigator.platform,
  userAgent = typeof navigator === 'undefined' ? '' : navigator.userAgent,
): DesktopPlatform => {
  const fingerprint = `${platform} ${userAgent}`.toLowerCase();
  if (fingerprint.includes('mac') || fingerprint.includes('iphone') || fingerprint.includes('ipad')) {
    return 'macos';
  }
  if (fingerprint.includes('win')) return 'windows';
  if (fingerprint.includes('linux') || fingerprint.includes('x11')) return 'linux';
  return 'unknown';
};

export const applyDesktopPlatform = (root: HTMLElement = document.documentElement) => {
  const platform = detectDesktopPlatform();
  root.dataset.platform = platform;
  return platform;
};
