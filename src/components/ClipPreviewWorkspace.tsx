import type { ComponentProps } from 'react';
import { AlertTriangle, Sliders } from 'lucide-react';

import { formatTransformRequestPhase, translate } from '../localization/runtime';
import type { ClipVersion } from '../types';
import { dateTimeAttribute, formatFullDateTime, formatRelativeTime } from '../utils/date';
import type { IntelligenceRequestStatus } from '../hooks/useIntelligenceRequestStatus';
import { useMinuteTick } from '../hooks/useMinuteTick';
import { ClipPreviewContent } from './ClipPreviewContent';

type PreviewContentProps = ComponentProps<typeof ClipPreviewContent>;

export function ClipPreviewWorkspace({
  activeManualTransformName,
  contentProps,
  isManualTransformRunning,
  isTransforming,
  onOpenIntelligence,
  onResetTransform,
  previewedVersion,
  transformError,
  transformRequestStatus,
}: {
  activeManualTransformName: string | null;
  contentProps: PreviewContentProps;
  isManualTransformRunning: boolean;
  isTransforming: boolean;
  onOpenIntelligence?: () => void;
  onResetTransform: () => void;
  previewedVersion: ClipVersion | null;
  transformError?: string;
  transformRequestStatus: IntelligenceRequestStatus;
}) {
  const relativeTimeNow = useMinuteTick();
  const hasPreviewBanner = Boolean(activeManualTransformName || previewedVersion);
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
            className="cursor-pointer font-semibold underline underline-offset-2"
            onClick={onOpenIntelligence}
          >
            {transformError}
          </button>
          : <span>{transformError}</span>}
      </div>
    </div>}
    {hasPreviewBanner && <div className="active-filter-banner flex items-center justify-between px-3 py-2 border rounded-lg">
      <div className="flex items-center space-x-2">
        <Sliders className="w-4 h-4" />
        <span className="flex items-center gap-1.5">
          <span>
            {previewedVersion
              ? translate('component.clipPreview.previewingRevision')
              : isManualTransformRunning
                ? formatTransformRequestPhase(transformRequestStatus)
                : translate('component.clipPreview.previewing')}
          </span>
          <strong>{previewedVersion
            ? <time
              dateTime={dateTimeAttribute(previewedVersion.created_at)}
              title={formatFullDateTime(previewedVersion.created_at)}
            >
              {formatRelativeTime(previewedVersion.created_at, relativeTimeNow)}
            </time>
            : activeManualTransformName}</strong>
        </span>
      </div>
      <button onClick={onResetTransform} className="active-filter-reset text-xs underline">
        {translate('common.reset')}
      </button>
    </div>}

    <ClipPreviewContent {...contentProps} />
  </div>;
}
