import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { ChevronDown } from 'lucide-react';

export interface MenuSelectOption {
  value: string;
  label: string;
  group?: string;
  count?: number;
  icon?: ReactNode;
  color?: string;
  disabled?: boolean;
}

interface MenuSelectProps {
  value: string;
  options: MenuSelectOption[];
  onChange: (value: string) => void;
  label: string;
  leadingIcon?: ReactNode;
  className?: string;
  compact?: boolean;
}

export function MenuSelect({
  value,
  options,
  onChange,
  label,
  leadingIcon,
  className = '',
  compact = false,
}: MenuSelectProps) {
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const [position, setPosition] = useState({ left: 8, top: 8, width: 220, ready: false });
  const selected = options.find((option) => option.value === value) ?? options[0];

  useLayoutEffect(() => {
    if (!isOpen) return;
    const positionMenu = () => {
      const trigger = triggerRef.current;
      if (!trigger) return;
      const viewportPadding = 8;
      const gap = 6;
      const rect = trigger.getBoundingClientRect();
      const width = Math.min(Math.max(rect.width, 220), window.innerWidth - viewportPadding * 2);
      const measuredHeight = menuRef.current?.getBoundingClientRect().height ?? 0;
      const left = Math.min(Math.max(viewportPadding, rect.left), window.innerWidth - width - viewportPadding);
      const fitsBelow = !measuredHeight || rect.bottom + gap + measuredHeight <= window.innerHeight - viewportPadding;
      const top = fitsBelow ? rect.bottom + gap : Math.max(viewportPadding, rect.top - gap - measuredHeight);
      setPosition({ left, top, width, ready: true });
    };
    positionMenu();
    window.addEventListener('resize', positionMenu);
    window.addEventListener('scroll', positionMenu, true);
    return () => {
      window.removeEventListener('resize', positionMenu);
      window.removeEventListener('scroll', positionMenu, true);
    };
  }, [isOpen, options.length]);

  useEffect(() => {
    if (!isOpen) return;
    const closeOutside = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!triggerRef.current?.contains(target) && !menuRef.current?.contains(target)) setIsOpen(false);
    };
    const closeWithKeyboard = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setIsOpen(false);
        triggerRef.current?.focus();
      }
    };
    window.addEventListener('pointerdown', closeOutside);
    window.addEventListener('keydown', closeWithKeyboard);
    return () => {
      window.removeEventListener('pointerdown', closeOutside);
      window.removeEventListener('keydown', closeWithKeyboard);
    };
  }, [isOpen]);

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        className={`menu-select-trigger flex min-w-0 items-center gap-2 border text-left ${compact ? 'rounded-lg px-2' : 'rounded-xl px-2.5'} ${className}`}
        aria-label={label}
        aria-haspopup="menu"
        aria-expanded={isOpen}
        onClick={() => setIsOpen((open) => !open)}
      >
        {leadingIcon}
        <span
          className={`min-w-0 flex-1 truncate text-xs font-semibold ${compact ? 'py-1.5' : 'py-2'}`}
          style={selected?.color ? { color: selected.color } : undefined}
        >
          {selected?.label ?? 'No selection'}{typeof selected?.count === 'number' ? ` (${selected.count})` : ''}
        </span>
        <ChevronDown className={`h-3.5 w-3.5 shrink-0 transition-transform ${isOpen ? 'rotate-180' : ''}`} aria-hidden="true" />
      </button>

      {isOpen && createPortal(
        <div
          ref={menuRef}
          role="menu"
          aria-label={label}
          className="theme-menu fixed max-h-80 overflow-y-auto rounded-xl border p-1.5 text-xs font-medium select-none"
          style={{ left: position.left, top: position.top, width: position.width, visibility: position.ready ? 'visible' : 'hidden' }}
        >
          {options.map((option, index) => {
            const active = option.value === value;
            const showGroup = option.group && option.group !== options[index - 1]?.group;
            return (
              <div key={option.value} role="none">
                {showGroup && (
                  <div className={`theme-text-subtle px-2.5 pb-1 pt-2 text-[10px] font-bold uppercase tracking-wider ${index > 0 ? 'theme-divider mt-1 border-t' : ''}`}>
                    {option.group}
                  </div>
                )}
                <button
                  type="button"
                  role="menuitemradio"
                  aria-checked={active}
                  disabled={option.disabled}
                  style={option.color ? { color: option.color } : undefined}
                  className={`theme-menu-item flex w-full items-center gap-2 rounded-lg px-2.5 text-left disabled:cursor-not-allowed disabled:opacity-40 ${compact ? 'py-1.5' : 'py-2'} ${active ? 'is-selected' : ''}`}
                  onClick={() => {
                    if (option.disabled) return;
                    onChange(option.value);
                    setIsOpen(false);
                    triggerRef.current?.focus();
                  }}
                >
                  {option.icon && <span className="grid h-4 w-4 shrink-0 place-items-center">{option.icon}</span>}
                  <span className="min-w-0 flex-1 truncate">{option.label}</span>
                  {typeof option.count === 'number' && <span className="theme-text-subtle tabular-nums">{option.count}</span>}
                </button>
              </div>
            );
          })}
        </div>,
        document.body,
      )}
    </>
  );
}
