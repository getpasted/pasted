import type { ComponentProps } from 'react';
import { AlertTriangle } from 'lucide-react';

import { translate } from '../localization/runtime';
import type { ClipVersion } from '../types';
import type { IntelligenceRequestStatus } from '../hooks/useIntelligenceRequestStatus';
import { useMinuteTick } from '../hooks/useMinuteTick';
import { ClipPreviewBanner } from './ClipPreviewBanner';
import { ClipPreviewContent } from './ClipPreviewContent';

type PreviewContentProps = ComponentProps<typeof ClipPreviewContent>;

export function ClipPreviewWorkspace({
  activeManualTransformName,
  canSaveVersion,
  contentProps,
  isManualTransformRunning,
  isTransforming,
  isSavingVersion,
  onCancelVersionPreview,
  onOpenIntelligence,
  onResetTransform,
  onSaveVersion,
  previewedVersion,
  transformError,
  transformRequestStatus,
}: {
  activeManualTransformName: string | null;
  canSaveVersion: boolean;
  contentProps: PreviewContentProps;
  isManualTransformRunning: boolean;
  isTransforming: boolean;
  isSavingVersion: boolean;
  onCancelVersionPreview: () => void;
  onOpenIntelligence?: () => void;
  onResetTransform: () => void;
  onSaveVersion: () => void;
  previewedVersion: ClipVersion | null;
  transformError?: string;
  transformRequestStatus: IntelligenceRequestStatus;
}) {
  const relativeTimeNow = useMinuteTick();
  return <div className="clip-preview-workspace overlay-scroll-region flex-1 overflow-y-auto p-4 space-y-4 font-mono text-xs">
    {transformError && !isTransforming && <div
      className="theme-status-warning flex items-start gap-2 rounded-lg border px-3 py-2"
      role="status"
    >
      <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
      <div className="min-w-0">
        <strong className="block">{translate('component.clipPreview.transformFailed')}</strong>
        <span>{translate('component.clipPreview.theClipStayedInItsBinAndItsContentWasNotReplaced')} </span>
        {transformError === 'Power on a provider and try again.' && onOpenIntelligence
          ? <button
            type="button"
            className="theme-inline-action cursor-pointer font-semibold underline underline-offset-2"
            onClick={onOpenIntelligence}
          >
            {transformError}
          </button>
          : <span>{transformError}</span>}
      </div>
    </div>}
    <ClipPreviewBanner
      activeManualTransformName={activeManualTransformName}
      canSaveVersion={canSaveVersion}
      isManualTransformRunning={isManualTransformRunning}
      isSavingVersion={isSavingVersion}
      onCancelVersionPreview={onCancelVersionPreview}
      onResetTransform={onResetTransform}
      onSaveVersion={onSaveVersion}
      previewedVersion={previewedVersion}
      relativeTimeNow={relativeTimeNow}
      transformRequestStatus={transformRequestStatus}
    />

    <ClipPreviewContent {...contentProps} />
  </div>;
}
