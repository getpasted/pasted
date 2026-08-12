import type {
  ButtonHTMLAttributes,
  HTMLAttributes,
  MouseEventHandler,
  ReactNode,
} from 'react';
import { X } from 'lucide-react';

function joinClasses(...classes: Array<string | undefined | false>) {
  return classes.filter(Boolean).join(' ');
}

export function AppDialogHeader({
  children,
  onClose,
  closeLabel = 'Close dialog',
  onMouseDown,
  className,
}: {
  children: ReactNode;
  onClose: () => void;
  closeLabel?: string;
  onMouseDown?: MouseEventHandler<HTMLElement>;
  className?: string;
}) {
  return (
    <header className={joinClasses('app-dialog-header', className)} onMouseDown={onMouseDown}>
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
  variant?: 'secondary' | 'primary' | 'warning' | 'danger';
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
