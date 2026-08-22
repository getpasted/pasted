import React, { useRef, useState } from 'react';
import { ChevronDown } from 'lucide-react';

import { translate } from '../localization/runtime';
import { AnchoredMenu, MenuDivider, MenuItem, MenuSubmenu } from './AnchoredMenu';
import type { SmartConditionRow, SmartConditionTarget, SmartTargetChoice, SmartTargetSection } from './binModalModel';

export function SmartConditionTargetSelect({
  condition,
  sections,
  onSelect,
}: {
  condition: SmartConditionRow;
  sections: SmartTargetSection[];
  onSelect: (target: SmartConditionTarget, value: string) => void;
}) {
  const triggerRef = useRef<HTMLButtonElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const [activeSubmenu, setActiveSubmenu] = useState<SmartConditionTarget | null>(null);
  const selectedSection = sections.find(({ target }) => target === condition.target);

  const close = () => {
    setIsOpen(false);
    setActiveSubmenu(null);
    triggerRef.current?.focus();
  };

  return <>
    <button
      ref={triggerRef}
      type="button"
      className="menu-select-trigger flex w-28 min-w-0 items-center gap-2 rounded-lg border px-2 text-start"
      aria-label={translate('component.binModal.conditionTarget')}
      aria-haspopup="menu"
      aria-expanded={isOpen}
      onClick={() => setIsOpen((open) => !open)}
    >
      <span className="bidi-interface-align min-w-0 flex-1 truncate py-1.5 text-xs font-semibold">
        {selectedSection?.label ?? condition.target}
      </span>
      <ChevronDown className={`h-3.5 w-3.5 shrink-0 transition-transform ${isOpen ? 'rotate-180' : ''}`} aria-hidden="true" />
    </button>
    {isOpen && (
      <AnchoredMenu
        anchor={{ kind: 'element', ref: triggerRef, align: 'start' }}
        ariaLabel={translate('component.binModal.conditionTarget')}
        onClose={close}
        className="w-56"
      >
        {sections.map((section) => <React.Fragment key={section.target}>
          {section.dividerBefore && <MenuDivider />}
          {section.choices ? (
            <MenuSubmenu
              label={section.label}
              open={activeSubmenu === section.target}
              onOpenChange={(open) => setActiveSubmenu((current) => (
                open ? section.target : current === section.target ? null : current
              ))}
              onSelect={() => {
                onSelect(
                  section.target,
                  section.target === 'clip_type'
                    ? condition.target === 'clip_type' ? condition.value : 'text'
                    : condition.target === section.target ? condition.value : '',
                );
                close();
              }}
              panelClassName="w-64 max-h-72 overflow-y-auto"
            >
              {section.choices.map((choice, index) => <React.Fragment key={`${section.target}:${choice.value}`}>
                {choice.group && choice.group !== section.choices?.[index - 1]?.group && (
                  <div className={`theme-text-subtle px-2.5 pb-1 pt-2 text-[10px] font-bold uppercase tracking-wider ${index > 0 ? 'theme-divider mt-1 border-t' : ''}`}>
                    {choice.group}
                  </div>
                )}
                <MenuItem
                  active={condition.target === section.target && condition.value === choice.value}
                  disabled={choice.disabled}
                  role="menuitemradio"
                  aria-checked={condition.target === section.target && condition.value === choice.value}
                  className="px-2.5 py-1.5"
                  onClick={() => {
                    if (choice.disabled) return;
                    onSelect(section.target, choice.value);
                    close();
                  }}
                >
                  {choice.label}
                </MenuItem>
              </React.Fragment>)}
            </MenuSubmenu>
          ) : (
            <MenuItem
              active={condition.target === section.target}
              role="menuitemradio"
              aria-checked={condition.target === section.target}
              className="px-3 py-1.5"
              onClick={() => {
                onSelect(section.target, '');
                close();
              }}
            >
              {section.label}
            </MenuItem>
          )}
        </React.Fragment>)}
      </AnchoredMenu>
    )}
  </>;
}

export function SmartConditionValueInput({
  value,
  choices,
  label,
  onChange,
}: {
  value: string;
  choices: SmartTargetChoice[];
  label: string;
  onChange: (value: string) => void;
}) {
  const anchorRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const [showAll, setShowAll] = useState(true);
  const selectedChoice = choices.find((choice) => choice.value === value);
  const displayedValue = selectedChoice?.label ?? value;
  const normalizedValue = displayedValue.trim().toLowerCase();
  const visibleChoices = showAll || !normalizedValue
    ? choices
    : choices.filter((choice) => (
      `${choice.label} ${choice.value}`.toLowerCase().includes(normalizedValue)
    ));

  return <div ref={anchorRef} className="theme-input form-field-valid flex min-w-0 flex-1 items-center rounded-lg border">
    <input
      ref={inputRef}
      type="text"
      role="combobox"
      aria-label={label}
      aria-autocomplete="list"
      aria-expanded={isOpen}
      placeholder={label}
      value={displayedValue}
      onFocus={() => {
        setShowAll(true);
        setIsOpen(true);
      }}
      onChange={(event) => {
        setShowAll(false);
        setIsOpen(true);
        onChange(event.target.value);
      }}
      onKeyDown={(event) => {
        if (event.key !== 'ArrowDown') return;
        event.preventDefault();
        setShowAll(true);
        setIsOpen(true);
      }}
      className="bidi-content min-w-0 flex-1 bg-transparent px-3 py-1.5 text-xs font-semibold focus:outline-none"
    />
    <button
      type="button"
      className="theme-text-muted grid self-stretch shrink-0 place-items-center px-2"
      aria-label={label}
      aria-haspopup="menu"
      aria-expanded={isOpen}
      onClick={() => {
        if (isOpen) {
          setIsOpen(false);
          return;
        }
        setShowAll(true);
        setIsOpen(true);
        inputRef.current?.focus({ preventScroll: true });
      }}
    >
      <ChevronDown className={`h-3.5 w-3.5 transition-transform ${isOpen ? 'rotate-180' : ''}`} aria-hidden="true" />
    </button>
    {isOpen && (
      <AnchoredMenu
        anchor={{ kind: 'element', ref: anchorRef, align: 'start', gap: 4 }}
        ariaLabel={label}
        onClose={() => setIsOpen(false)}
        restoreFocus={false}
        className="max-h-72 overflow-y-auto"
        style={{ width: anchorRef.current?.getBoundingClientRect().width ?? 220 }}
      >
        {visibleChoices.map((choice, index) => <React.Fragment key={choice.value}>
          {choice.group && choice.group !== visibleChoices[index - 1]?.group && (
            <div className={`theme-text-subtle px-2.5 pb-1 pt-2 text-[10px] font-bold uppercase tracking-wider ${index > 0 ? 'theme-divider mt-1 border-t' : ''}`}>
              {choice.group}
            </div>
          )}
          <MenuItem
            active={value === choice.value}
            disabled={choice.disabled}
            role="menuitemradio"
            aria-checked={value === choice.value}
            className="px-2.5 py-1.5"
            onClick={() => {
              if (choice.disabled) return;
              onChange(choice.value);
              setIsOpen(false);
            }}
          >
            {choice.label}
          </MenuItem>
        </React.Fragment>)}
        {visibleChoices.length === 0 && (
          <div className="theme-text-subtle px-3 py-4 text-center text-xs">
            {translate('component.menuSelect.noMatches')}
          </div>
        )}
      </AnchoredMenu>
    )}
  </div>;
}
