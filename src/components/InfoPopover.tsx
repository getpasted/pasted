import { Info } from 'lucide-react';
import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';

interface InfoPopoverProps {
  label: string;
  children: ReactNode;
  tone?: 'info' | 'warning' | 'danger';
}

const POPOVER_WIDTH = 200;
const VIEWPORT_MARGIN = 12;

export function InfoPopover({ label, children, tone = 'info' }: InfoPopoverProps) {
  const descriptionId = useId();
  const triggerRef = useRef<HTMLButtonElement>(null);
  const contentRef = useRef<HTMLSpanElement>(null);
  const closeTimerRef = useRef<number | null>(null);
  const [isOpen, setIsOpen] = useState(false);
  const [position, setPosition] = useState({ left: 0, top: 0 });

  const cancelClose = useCallback(() => {
    if (closeTimerRef.current !== null) {
      window.clearTimeout(closeTimerRef.current);
      closeTimerRef.current = null;
    }
  }, []);

  const closeSoon = useCallback(() => {
    cancelClose();
    closeTimerRef.current = window.setTimeout(() => setIsOpen(false), 90);
  }, [cancelClose]);

  const updatePosition = useCallback(() => {
    const trigger = triggerRef.current;
    if (!trigger) return;
    const rect = trigger.getBoundingClientRect();
    const contentHeight = contentRef.current?.getBoundingClientRect().height ?? 0;
    const maximumLeft = Math.max(VIEWPORT_MARGIN, window.innerWidth - POPOVER_WIDTH - VIEWPORT_MARGIN);
    const left = Math.min(Math.max(VIEWPORT_MARGIN, rect.left - 8), maximumLeft);
    const below = rect.bottom + 6;
    const top = contentHeight > 0 && below + contentHeight > window.innerHeight - VIEWPORT_MARGIN
      ? Math.max(VIEWPORT_MARGIN, rect.top - contentHeight - 6)
      : below;
    setPosition({ left, top });
  }, []);

  const open = useCallback(() => {
    cancelClose();
    setIsOpen(true);
  }, [cancelClose]);

  useLayoutEffect(() => {
    if (isOpen) updatePosition();
  }, [isOpen, updatePosition]);

  useEffect(() => {
    if (!isOpen) return undefined;
    const reposition = () => updatePosition();
    window.addEventListener('resize', reposition);
    window.addEventListener('scroll', reposition, true);
    return () => {
      window.removeEventListener('resize', reposition);
      window.removeEventListener('scroll', reposition, true);
    };
  }, [isOpen, updatePosition]);

  useEffect(() => () => cancelClose(), [cancelClose]);

  return (
    <span className={`info-popover is-${tone}`} onPointerEnter={open} onPointerLeave={closeSoon}>
      <button
        ref={triggerRef}
        type="button"
        className="info-popover-trigger"
        aria-label={label}
        aria-describedby={isOpen ? descriptionId : undefined}
        aria-expanded={isOpen}
        onFocus={open}
        onBlur={closeSoon}
        onKeyDown={(event) => {
          if (event.key === 'Escape') {
            cancelClose();
            setIsOpen(false);
            event.currentTarget.blur();
          }
        }}
      >
        <Info className="h-3.5 w-3.5" />
      </button>
      {isOpen && createPortal(
        <span
          ref={contentRef}
          id={descriptionId}
          role="tooltip"
          className={`info-popover-content is-open is-${tone}`}
          style={{ left: position.left, top: position.top }}
          onPointerEnter={cancelClose}
          onPointerLeave={closeSoon}
        >
          {children}
        </span>,
        document.body,
      )}
    </span>
  );
}
