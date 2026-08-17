import { useRef, useState, type ReactNode } from 'react';
import { Check, ChevronDown } from 'lucide-react';
import { AnchoredMenu, MenuItem } from './AnchoredMenu';
import { OverflowText } from './OverflowText';

export interface MenuMultiSelectOption {
  value: string;
  label: string;
  icon?: ReactNode;
  disabled?: boolean;
}

export function MenuMultiSelect({
  values,
  options,
  onChange,
  label,
  placeholder,
  className = '',
  disabled = false,
}: {
  values: string[];
  options: MenuMultiSelectOption[];
  onChange: (values: string[]) => void;
  label: string;
  placeholder: string;
  className?: string;
  disabled?: boolean;
}) {
  const triggerRef = useRef<HTMLButtonElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const selected = options.filter((option) => values.includes(option.value));
  const summary = selected.length > 0
    ? selected.map((option) => option.label).join(', ')
    : placeholder;

  return <>
    <button
      ref={triggerRef}
      type="button"
      className={`menu-select-trigger ui-field-radius flex min-w-0 items-center gap-2 border px-2.5 text-start ${className}`}
      aria-label={label}
      aria-haspopup="menu"
      aria-expanded={isOpen}
      disabled={disabled}
      onClick={() => setIsOpen((open) => !open)}
    >
      <OverflowText text={summary} className="bidi-interface-align min-w-0 flex-1 truncate py-2 text-xs font-semibold" />
      <ChevronDown className={`h-3.5 w-3.5 shrink-0 transition-transform ${isOpen ? 'rotate-180' : ''}`} aria-hidden="true" />
    </button>
    {isOpen && !disabled && <AnchoredMenu
      anchor={{ kind: 'element', ref: triggerRef, align: 'start' }}
      ariaLabel={label}
      onClose={() => setIsOpen(false)}
      style={{ width: Math.max(triggerRef.current?.getBoundingClientRect().width ?? 220, 220) }}
    >
      {options.map((option) => {
        const active = values.includes(option.value);
        return <MenuItem
          key={option.value}
          type="button"
          role="menuitemcheckbox"
          aria-checked={active}
          active={active}
          disabled={option.disabled}
          className="gap-2 px-2.5 py-2"
          onClick={() => {
            if (option.disabled) return;
            const next = active
              ? values.filter((value) => value !== option.value)
              : [...values, option.value];
            if (next.length > 0) onChange(next);
          }}
        >
          {option.icon && <span className="grid h-4 w-4 shrink-0 place-items-center">{option.icon}</span>}
          <OverflowText text={option.label} className="bidi-interface-align min-w-0 flex-1 truncate" />
          <Check className={`h-3.5 w-3.5 shrink-0 ${active ? '' : 'invisible'}`} aria-hidden="true" />
        </MenuItem>;
      })}
    </AnchoredMenu>}
  </>;
}
