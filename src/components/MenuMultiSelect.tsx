import { useRef, useState, type ReactNode } from 'react';
import { Check, CheckSquare2, ChevronDown, MinusSquare, Square } from 'lucide-react';
import { AnchoredMenu, MenuItem } from './AnchoredMenu';
import { OverflowText } from './OverflowText';
import { groupSelectionState, initialMultiSelectScrollKey, toggleMultiSelectGroup } from './menuMultiSelectModel';

export interface MenuMultiSelectOption {
  value: string;
  label: string;
  group?: string;
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
  groupToggleLabel,
}: {
  values: string[];
  options: MenuMultiSelectOption[];
  onChange: (values: string[]) => void;
  label: string;
  placeholder: string;
  className?: string;
  disabled?: boolean;
  groupToggleLabel?: string;
}) {
  const triggerRef = useRef<HTMLButtonElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const selected = options.filter((option) => values.includes(option.value));
  const summary = selected.length > 0
    ? selected.map((option) => option.label).join(', ')
    : placeholder;
  const initialScrollKey = initialMultiSelectScrollKey(values, options);

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
      initialScrollTarget={initialScrollKey}
      className="max-h-80 overflow-y-auto"
      style={{ width: Math.max(triggerRef.current?.getBoundingClientRect().width ?? 220, 220) }}
    >
      {options.map((option, index) => {
        const active = values.includes(option.value);
        const showGroup = option.group && option.group !== options[index - 1]?.group;
        const groupOptions = option.group
          ? options.filter((candidate) => candidate.group === option.group)
          : [];
        const groupState = groupSelectionState(values, groupOptions);
        return <div key={option.value} role="none">
          {showGroup && <div data-menu-scroll-key={`group:${option.group}`} className={`theme-text-subtle flex items-center justify-between gap-2 px-2.5 pb-1 pt-2 text-[10px] font-bold uppercase tracking-wider ${index > 0 ? 'theme-divider mt-1 border-t' : ''}`}>
            <span>{option.group}</span>
            {groupToggleLabel && <button
              type="button"
              role="menuitemcheckbox"
              aria-checked={groupState.some ? 'mixed' : groupState.all}
              aria-label={groupToggleLabel}
              title={groupToggleLabel}
              className={`theme-menu-item flex h-6 items-center gap-1.5 rounded px-1.5 normal-case tracking-normal ${groupState.all || groupState.some ? 'is-selected' : ''}`}
              onClick={() => onChange(toggleMultiSelectGroup(values, groupOptions))}
            >
              <span>{groupToggleLabel}</span>
              {groupState.all
                ? <CheckSquare2 className="h-3.5 w-3.5" aria-hidden="true" />
                : groupState.some
                  ? <MinusSquare className="h-3.5 w-3.5" aria-hidden="true" />
                  : <Square className="h-3.5 w-3.5" aria-hidden="true" />}
            </button>}
          </div>}
          <MenuItem
            data-menu-scroll-key={`option:${option.value}`}
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
              onChange(next);
            }}
          >
            {option.icon && <span className="grid h-4 w-4 shrink-0 place-items-center">{option.icon}</span>}
            <OverflowText text={option.label} className="bidi-interface-align min-w-0 flex-1 truncate" />
            <Check className={`h-3.5 w-3.5 shrink-0 ${active ? '' : 'invisible'}`} aria-hidden="true" />
          </MenuItem>
        </div>;
      })}
    </AnchoredMenu>}
  </>;
}
