import { AlertTriangle } from 'lucide-react';
import { useId } from 'react';
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
  details?: string;
  confirmLabel: string;
  tone?: 'warning' | 'danger';
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
            icon={<AlertTriangle />}
            tone={tone}
          />
        </AppDialogHeader>
        {request.details && (
          <AppDialogBody>
            <p className="theme-surface theme-text-muted rounded-xl border p-3 text-xs leading-relaxed">
              {request.details}
            </p>
          </AppDialogBody>
        )}
        <AppDialogFooter>
          <AppDialogButton onClick={requestClose} autoFocus>{translate('common.cancel')}</AppDialogButton>
          <AppDialogButton variant={tone} onClick={request.onConfirm}>{request.confirmLabel}</AppDialogButton>
        </AppDialogFooter>
      </>}
    </AppDialog>
  );
}
