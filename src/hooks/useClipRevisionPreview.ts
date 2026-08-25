import { useState } from 'react';

import type { ClipVersion } from '../types';

export function useClipRevisionPreview() {
  const [selectedVersion, setSelectedVersion] = useState<ClipVersion | null>(null);
  const [transientPreview, setTransientPreview] = useState<ClipVersion | null | undefined>();

  return {
    previewedVersion: transientPreview !== undefined ? transientPreview : selectedVersion,
    clearPreview: () => {
      setSelectedVersion(null);
      setTransientPreview(undefined);
    },
    beginTransientPreview: (version: ClipVersion) => setTransientPreview(
      version.is_current ? null : version,
    ),
    endTransientPreview: () => setTransientPreview(undefined),
    togglePreview: (version: ClipVersion) => setSelectedVersion((current) => (
      version.is_current || current?.id === version.id ? null : version
    )),
  };
}
