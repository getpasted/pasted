import { AlertCircle } from 'lucide-react';
import { translate } from '../localization/runtime';
import { ActionButton } from './AppDialogLayout';

interface SearchErrorNoticeProps {
  onRetry: () => void;
}

export function SearchErrorNotice({ onRetry }: SearchErrorNoticeProps) {
  return (
    <div role="alert" className="theme-status-danger flex items-center gap-2 rounded-lg border px-3 py-2 text-xs">
      <AlertCircle className="h-4 w-4 shrink-0" aria-hidden="true" />
      <span className="min-w-0 flex-1">{translate('component.searchErrorNotice.searchCouldNotBeCompleted')}</span>
      <ActionButton
        onClick={onRetry}
        className="shrink-0"
      >
        {translate('component.searchErrorNotice.retrySearch')}
      </ActionButton>
    </div>
  );
}
