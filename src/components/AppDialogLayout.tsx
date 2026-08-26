import type {
  ButtonHTMLAttributes,
  HTMLAttributes,
  MouseEventHandler,
  ReactNode,
} from 'react';
import { Check, LoaderCircle, X } from 'lucide-react';
import { translate } from '../localization/runtime';

function joinClasses(...classes: Array<string | undefined | false>) {
  return classes.filter(Boolean).join(' ');
}

export function AppDialogHeader({
  children,
  onClose,
  closeLabel = translate('component.appDialogLayout.closeDialog'),
  onMouseDown,
  onDoubleClick,
  className,
}: {
  children: ReactNode;
  onClose: () => void;
  closeLabel?: string;
  onMouseDown?: MouseEventHandler<HTMLElement>;
  onDoubleClick?: MouseEventHandler<HTMLElement>;
  className?: string;
}) {
  return (
    <header
      className={joinClasses('app-dialog-header', className)}
      onMouseDown={onMouseDown}
      onDoubleClick={onDoubleClick}
    >
      <div className="min-w-0 flex-1">{children}</div>
      <button
        type="button"
        onMouseDown={(event) => event.stopPropagation()}
        onClick={onClose}
        className="app-dialog-close"
        aria-label={closeLabel}
      >
        <X />
      </button>
    </header>
  );
}

export function AppDialogHeading({
  id,
  title,
  description,
  icon,
  tone = 'default',
}: {
  id: string;
  title: ReactNode;
  description?: ReactNode;
  icon?: ReactNode;
  tone?: 'default' | 'info' | 'warning' | 'danger';
}) {
  return (
    <div className={joinClasses('app-dialog-heading', `is-${tone}`)}>
      {icon && <span className="app-dialog-heading-icon">{icon}</span>}
      <div className="min-w-0">
        <h2 id={id} className="app-dialog-title">{title}</h2>
        {description && <p className="app-dialog-description">{description}</p>}
      </div>
    </div>
  );
}

export function AppDialogBody({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={joinClasses('app-dialog-body', className)} {...props} />;
}

export function AppDialogFooter({
  className,
  align = 'end',
  ...props
}: HTMLAttributes<HTMLDivElement> & { align?: 'end' | 'between' }) {
  return (
    <footer
      className={joinClasses('app-dialog-footer', align === 'between' && 'is-between', className)}
      {...props}
    />
  );
}

export function ActionButton({
  variant = 'secondary',
  className,
  type = 'button',
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: 'secondary' | 'primary' | 'warning' | 'danger' | 'solid-primary' | 'solid-danger';
}) {
  return (
    <button
      type={type}
      className={joinClasses('app-dialog-button', `is-${variant}`, className)}
      {...props}
    />
  );
}

export function AppDialogButton(props: Parameters<typeof ActionButton>[0]) {
  return <ActionButton {...props} />;
}

export function SaveButtonContent({
  isSaving = false,
  isSaved = false,
}: {
  isSaving?: boolean;
  isSaved?: boolean;
}) {
  if (isSaving) return <><LoaderCircle className="h-3.5 w-3.5 animate-spin" /><span>{translate('common.saving')}</span></>;
  if (isSaved) return <><Check className="h-3.5 w-3.5" /><span>{translate('common.saved')}</span></>;
  return <span>{translate('common.save')}</span>;
}
