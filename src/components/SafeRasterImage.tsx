import React from 'react';
import { decodeSafeRasterDataUrl } from '../utils/safeRasterImage';

type SafeRasterImageProps = Omit<React.ImgHTMLAttributes<HTMLImageElement>, 'src'> & {
  source: string | null | undefined;
  alt: string;
};

export function SafeRasterImage({ source, ...props }: SafeRasterImageProps) {
  const [objectUrl, setObjectUrl] = React.useState<string | null>(null);

  React.useEffect(() => {
    setObjectUrl(null);
    const raster = decodeSafeRasterDataUrl(source);
    if (!raster || typeof URL.createObjectURL !== 'function') return undefined;
    const nextObjectUrl = URL.createObjectURL(new Blob([raster.bytes], { type: raster.mimeType }));
    setObjectUrl(nextObjectUrl);
    return () => URL.revokeObjectURL(nextObjectUrl);
  }, [source]);

  if (!objectUrl) return null;
  return <img {...props} src={objectUrl} />;
}
