import { LoaderCircle, Save, Sliders, X } from 'lucide-react';

import type { IntelligenceRequestStatus } from '../hooks/useIntelligenceRequestStatus';
import { formatTransformRequestPhase, translate } from '../localization/runtime';
import type { ClipVersion } from '../types';
import { dateTimeAttribute, formatFullDateTime, formatRelativeTime } from '../utils/date';

export function ClipPreviewBanner({
  activeManualTransformName,
  canSaveVersion,
  isManualTransformRunning,
  isSavingVersion,
  onCancelVersionPreview,
  onResetTransform,
  onSaveVersion,
  previewedVersion,
  relativeTimeNow,
  transformRequestStatus,
}: {
  activeManualTransformName: string | null;
  canSaveVersion: boolean;
  isManualTransformRunning: boolean;
  isSavingVersion: boolean;
  onCancelVersionPreview: () => void;
  onResetTransform: () => void;
  onSaveVersion: () => void;
  previewedVersion: ClipVersion | null;
  relativeTimeNow: number;
  transformRequestStatus: IntelligenceRequestStatus;
}) {
  if (!activeManualTransformName && !previewedVersion) return null;

  return <div className="active-filter-banner flex items-center justify-between rounded-lg border px-3 py-2">
    <div className="flex items-center space-x-2">
      <Sliders className="h-4 w-4" />
      <span className="flex items-center gap-1.5">
        <span>{previewedVersion
          ? translate('component.clipPreview.previewingVersion')
          : isManualTransformRunning
            ? formatTransformRequestPhase(transformRequestStatus)
            : translate('component.clipPreview.previewing')}</span>
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
    {previewedVersion
      ? <div className="floating-action-strip flex shrink-0 items-center gap-1 rounded-lg border p-1">
        <button type="button" onClick={onCancelVersionPreview} disabled={isSavingVersion} className="floating-action-button disabled:cursor-not-allowed disabled:opacity-40" title={translate('common.cancel')} aria-label={translate('common.cancel')}>
          <X />
        </button>
        <button type="button" onClick={onSaveVersion} disabled={isSavingVersion || !canSaveVersion} className="floating-action-button is-accent disabled:cursor-not-allowed disabled:opacity-40" title={translate('common.save')} aria-label={translate('common.save')}>
          {isSavingVersion ? <LoaderCircle className="animate-spin" /> : <Save />}
        </button>
      </div>
      : <button type="button" onClick={onResetTransform} className="active-filter-reset text-xs underline">
        {translate('common.reset')}
      </button>}
  </div>;
}
