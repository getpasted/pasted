import { AlertTriangle } from 'lucide-react';
import { useId, type ReactNode } from 'react';
import { AppDialog } from './AppDialog';
import { translate } from '../localization/runtime';
import {
  AppDialogBody,
  AppDialogButton,
  AppDialogFooter,
  AppDialogHeader,
  AppDialogHeading,
} from './AppDialogLayout';

export interface ConfirmationDialogRequest {
  title: string;
  description: string;
  details?: ReactNode;
  confirmLabel: string;
  confirmDisabled?: boolean;
  icon?: ReactNode;
  tone?: 'info' | 'warning' | 'danger';
  onConfirm: () => void | Promise<void>;
}

export function ConfirmationDialog({
  request,
  onCancel,
}: {
  request: ConfirmationDialogRequest | null;
  onCancel: () => void;
}) {
  const titleId = useId();
  if (!request) return null;

  const tone = request.tone ?? 'warning';
  const confirmVariant = tone === 'info' ? 'solid-primary' : tone;
  return (
    <AppDialog
      isOpen
      onClose={onCancel}
      labelledBy={titleId}
      panelClassName="theme-panel w-full max-w-md overflow-hidden border"
    >
      {({ requestClose }) => <>
        <AppDialogHeader onClose={requestClose}>
          <AppDialogHeading
            id={titleId}
            title={request.title}
            description={request.description}
            icon={request.icon ?? <AlertTriangle />}
            tone={tone}
          />
        </AppDialogHeader>
        {request.details && (
          <AppDialogBody>
            <div className={typeof request.details === 'string'
              ? 'theme-surface theme-text-muted rounded-xl border p-3 text-xs leading-relaxed'
              : 'theme-text-muted text-xs leading-relaxed'}>
              {request.details}
            </div>
          </AppDialogBody>
        )}
        <AppDialogFooter>
          <AppDialogButton onClick={requestClose} autoFocus>{translate('common.cancel')}</AppDialogButton>
          <AppDialogButton variant={confirmVariant} onClick={request.onConfirm} disabled={request.confirmDisabled}>{request.confirmLabel}</AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
}
