import { useCallback, useEffect, useId, useRef, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { ActionButton } from './AppDialogLayout';

const dialogStack: symbol[] = [];

interface AppDialogControls {
  requestClose: () => void;
}

interface AppDialogProps {
  isOpen: boolean;
  onClose: () => void;
  labelledBy: string;
  children: ReactNode | ((controls: AppDialogControls) => ReactNode);
  isDirty?: boolean;
  discardMessage?: string;
  overlayClassName?: string;
  panelClassName?: string;
}

export function AppDialog({
  isOpen,
  onClose,
  labelledBy,
  children,
  isDirty = false,
  discardMessage = 'Discard your unsaved changes?',
  overlayClassName = 'p-4',
  panelClassName = '',
}: AppDialogProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const discardPanelRef = useRef<HTMLDivElement>(null);
  const tokenRef = useRef(Symbol('app-dialog'));
  const discardTitleId = useId();
  const [isDiscardConfirmOpen, setIsDiscardConfirmOpen] = useState(false);
  const onCloseRef = useRef(onClose);
  const isDirtyRef = useRef(isDirty);
  const isDiscardConfirmOpenRef = useRef(isDiscardConfirmOpen);
  const discardMessageRef = useRef(discardMessage);
  onCloseRef.current = onClose;
  isDirtyRef.current = isDirty;
  isDiscardConfirmOpenRef.current = isDiscardConfirmOpen;
  discardMessageRef.current = discardMessage;

  const requestClose = useCallback(() => {
    if (isDirtyRef.current) {
      setIsDiscardConfirmOpen(true);
      return;
    }
    onCloseRef.current();
  }, []);

  const discardAndClose = useCallback(() => {
    setIsDiscardConfirmOpen(false);
    onCloseRef.current();
  }, []);

  useEffect(() => {
    if (!isOpen) setIsDiscardConfirmOpen(false);
  }, [isOpen]);

  useEffect(() => {
    if (!isDiscardConfirmOpen) return;
    const focusTimer = window.requestAnimationFrame(() => {
      discardPanelRef.current?.querySelector<HTMLElement>('[autofocus]')?.focus();
    });
    return () => window.cancelAnimationFrame(focusTimer);
  }, [isDiscardConfirmOpen]);

  useEffect(() => {
    if (!isOpen) return;
    const token = tokenRef.current;
    dialogStack.push(token);
    const previousFocus = document.activeElement as HTMLElement | null;
    const focusTimer = window.requestAnimationFrame(() => {
      const initialFocus = panelRef.current?.querySelector<HTMLElement>('[autofocus]')
        ?? panelRef.current?.querySelector<HTMLElement>(
          'button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
        );
      initialFocus?.focus();
    });

    const handleKeyDown = (event: KeyboardEvent) => {
      if (dialogStack[dialogStack.length - 1] !== token) return;
      if (event.key === 'Escape') {
        event.preventDefault();
        if (isDiscardConfirmOpenRef.current) {
          setIsDiscardConfirmOpen(false);
        } else {
          requestClose();
        }
        return;
      }
      const activePanel = isDiscardConfirmOpenRef.current ? discardPanelRef.current : panelRef.current;
      if (event.key !== 'Tab' || !activePanel) return;
      const focusable = activePanel.querySelectorAll<HTMLElement>(
        'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
      );
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.cancelAnimationFrame(focusTimer);
      window.removeEventListener('keydown', handleKeyDown);
      const index = dialogStack.lastIndexOf(token);
      if (index >= 0) dialogStack.splice(index, 1);
      previousFocus?.focus();
    };
  }, [isOpen, requestClose]);

  if (!isOpen) return null;

  return createPortal(
    <div
      className={`app-dialog-overlay fixed inset-0 flex items-center justify-center select-none animate-in fade-in duration-150 ${overlayClassName}`}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) requestClose();
      }}
    >
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={labelledBy}
        className={`app-dialog-panel ${panelClassName}`}
      >
        {typeof children === 'function' ? children({ requestClose }) : children}
      </div>
      {isDiscardConfirmOpen && (
        <div
          className="app-dialog-confirm-overlay fixed inset-0 flex items-center justify-center p-4"
          onMouseDown={(event) => {
            event.stopPropagation();
            if (event.target === event.currentTarget) setIsDiscardConfirmOpen(false);
          }}
        >
          <div
            ref={discardPanelRef}
            role="alertdialog"
            aria-modal="true"
            aria-labelledby={discardTitleId}
            className="app-dialog-confirm-panel theme-panel w-full max-w-sm overflow-hidden rounded-2xl border shadow-2xl"
          >
            <div className="app-dialog-body">
              <h2 id={discardTitleId} className="app-dialog-title">Discard changes?</h2>
              <p className="app-dialog-description mt-2">{discardMessageRef.current}</p>
            </div>
            <div className="app-dialog-footer">
              <ActionButton
                onClick={() => setIsDiscardConfirmOpen(false)}
                autoFocus
              >
                Keep Editing
              </ActionButton>
              <ActionButton variant="danger" onClick={discardAndClose}>
                Discard
              </ActionButton>
            </div>
          </div>
        </div>
      )}
    </div>,
    document.body,
  );
}
