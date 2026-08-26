import type { ReactNode } from 'react';
import { MenuSelect, type MenuSelectOption } from './MenuSelect';
import { translate } from '../localization/runtime';

export type TransformCategoryOption = MenuSelectOption;

interface TransformCategorySelectProps {
  accent: 'manual-transforms' | 'operations';
  value: string;
  options: TransformCategoryOption[];
  onChange: (value: string) => void;
  label?: string;
  leadingIcon?: ReactNode;
  searchable?: boolean;
  searchPlaceholder?: string;
}

export function TransformCategorySelect({
  accent,
  value,
  options,
  onChange,
  label = translate('component.transformCategorySelect.filter'),
  leadingIcon,
  searchable = false,
  searchPlaceholder,
}: TransformCategorySelectProps) {
  return (
    <MenuSelect
      value={value}
      options={options}
      onChange={onChange}
      label={label}
      leadingIcon={leadingIcon}
      className={`transform-category-select ${accent}`}
      searchable={searchable}
      searchPlaceholder={searchPlaceholder}
    />
  );
}
