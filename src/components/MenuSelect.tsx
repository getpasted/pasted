import { useRef, useState, type ReactNode } from 'react';
import { ChevronDown, Search } from 'lucide-react';
import { AnchoredMenu, MenuItem } from './AnchoredMenu';
import { OverflowText } from './OverflowText';
import { translate } from '../localization/runtime';

export interface MenuSelectOption {
  value: string;
  label: string;
  group?: string;
  count?: number;
  icon?: ReactNode;
  color?: string;
  disabled?: boolean;
  dividerBefore?: boolean;
}

interface MenuSelectProps {
  value: string;
  options: MenuSelectOption[];
  onChange: (value: string) => void;
  label: string;
  leadingIcon?: ReactNode;
  className?: string;
  compact?: boolean;
  disabled?: boolean;
  searchable?: boolean;
  searchPlaceholder?: string;
}

export function MenuSelect({
  value,
  options,
  onChange,
  label,
  leadingIcon,
  className = '',
  compact = false,
  disabled = false,
  searchable = false,
  searchPlaceholder = translate('component.menuSelect.search'),
}: MenuSelectProps) {
  const triggerRef = useRef<HTMLButtonElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const [query, setQuery] = useState('');
  const selected = options.find((option) => option.value === value) ?? options[0];
  const visibleOptions = searchable && query.trim()
    ? options.filter((option) => `${option.label} ${option.value}`.toLowerCase().includes(query.trim().toLowerCase()))
    : options;
  const selectedText = selected
    ? typeof selected.count === 'number'
      ? translate('component.menuSelect.selectionWithCount', { label: selected.label, count: selected.count })
      : selected.label
    : translate('component.menuSelect.noSelection');

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        className={`menu-select-trigger flex min-w-0 items-center gap-2 border text-start ${compact ? 'rounded-lg px-2' : 'ui-field-radius px-2.5'} ${className}`}
        aria-label={label}
        aria-haspopup="menu"
        aria-expanded={isOpen}
        disabled={disabled}
        onClick={() => setIsOpen((open) => {
          if (!open) setQuery('');
          return !open;
        })}
      >
        {leadingIcon}
        <OverflowText
          text={selectedText}
          className={`bidi-interface-align min-w-0 flex-1 truncate text-xs font-semibold ${compact ? 'py-1.5' : 'py-2'}`}
          style={selected?.color ? { color: selected.color } : undefined}
        />
        <ChevronDown className={`h-3.5 w-3.5 shrink-0 transition-transform ${isOpen ? 'rotate-180' : ''}`} aria-hidden="true" />
      </button>

      {isOpen && !disabled && (
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
          {searchable && (
            <label className="theme-menu-search theme-divider sticky top-0 z-10 flex items-center gap-2 border-b p-2">
              <Search className="theme-text-subtle h-3.5 w-3.5 shrink-0" aria-hidden="true" />
              <input
                autoFocus
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder={searchPlaceholder}
                className="theme-input min-w-0 flex-1 rounded-md border px-2 py-1.5 text-xs"
              />
            </label>
          )}
          {visibleOptions.map((option, index) => {
            const active = option.value === value;
            const showGroup = option.group && option.group !== visibleOptions[index - 1]?.group;
            return (
              <div key={option.value} role="none">
                {option.dividerBefore && index > 0 && !showGroup
                  && <div role="separator" className="theme-divider my-1 border-t" />}
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
                  <OverflowText text={option.label} className="bidi-interface-align min-w-0 flex-1 truncate" />
                  {typeof option.count === 'number' && <span className="theme-text-subtle tabular-nums">{option.count}</span>}
                </MenuItem>
              </div>
            );
          })}
          {visibleOptions.length === 0 && <div className="theme-text-subtle px-3 py-4 text-center text-xs">{translate('component.menuSelect.noMatches')}</div>}
        </AnchoredMenu>
      )}
    </>
  );
}
