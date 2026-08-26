import { FileText } from 'lucide-react';

import { translate } from '../localization/runtime';

export function ClipPreviewEmptyState() {
  return <div className="clip-preview-empty flex-1 col-preview h-screen flex flex-col items-center justify-center p-8 select-none">
    <div className="clip-preview-empty-icon theme-surface elevation-raised w-16 h-16 rounded-2xl border flex items-center justify-center mb-4">
      <FileText className="w-8 h-8" />
    </div>
    <p className="theme-text-main text-sm font-medium">{translate('component.clipPreview.noClipSelected')}</p>
    <p className="theme-text-muted text-xs mt-1 max-w-xs text-center">
      {translate('component.clipPreview.selectAnItemFromHistoryOrRightClickToCopyTransformAdd')}
    </p>
  </div>;
}
