import { useRef, useState, type ReactNode } from 'react';
import { ChevronDown } from 'lucide-react';
import { AnchoredMenu, MenuItem } from './AnchoredMenu';

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
  const [isOpen, setIsOpen] = useState(false);
  const selected = options.find((option) => option.value === value) ?? options[0];

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

      {isOpen && (
        <AnchoredMenu
          anchor={{ kind: 'element', ref: triggerRef, align: 'start' }}
          ariaLabel={label}
          onClose={() => setIsOpen(false)}
          className="max-h-80 overflow-y-auto"
          style={{
            width: Math.min(
              Math.max(triggerRef.current?.getBoundingClientRect().width ?? 220, 220),
              window.innerWidth - 16,
            ),
          }}
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
                <MenuItem
                  type="button"
                  role="menuitemradio"
                  aria-checked={active}
                  active={active}
                  disabled={option.disabled}
                  style={option.color ? { color: option.color } : undefined}
                  className={`gap-2 px-2.5 ${compact ? 'py-1.5' : 'py-2'}`}
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
                </MenuItem>
              </div>
            );
          })}
        </AnchoredMenu>
      )}
    </>
  );
}
