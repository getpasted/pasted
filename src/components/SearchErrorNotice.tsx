import { AlertCircle } from 'lucide-react';
import { translate } from '../localization/runtime';

interface SearchErrorNoticeProps {
  onRetry: () => void;
}

export function SearchErrorNotice({ onRetry }: SearchErrorNoticeProps) {
  return (
    <div role="alert" className="theme-status-danger flex items-center gap-2 rounded-lg border px-3 py-2 text-xs">
      <AlertCircle className="h-4 w-4 shrink-0" aria-hidden="true" />
      <span className="min-w-0 flex-1">{translate('component.searchErrorNotice.searchCouldNotBeCompleted')}</span>
      <button
        type="button"
        onClick={onRetry}
        className="theme-secondary-button shrink-0 rounded-md border px-2 py-1 text-[11px] font-semibold"
      >
        {translate('component.searchErrorNotice.retrySearch')}
      </button>
    </div>
  );
}
