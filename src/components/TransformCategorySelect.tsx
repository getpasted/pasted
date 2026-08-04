import type { ReactNode } from 'react';
import { Filter } from 'lucide-react';
import { MenuSelect, type MenuSelectOption } from './MenuSelect';

export type TransformCategoryOption = MenuSelectOption;

interface TransformCategorySelectProps {
  accent: 'pipelines' | 'operations';
  value: string;
  options: TransformCategoryOption[];
  onChange: (value: string) => void;
  label?: string;
  leadingIcon?: ReactNode;
}

export function TransformCategorySelect({
  accent,
  value,
  options,
  onChange,
  label = 'Filter',
  leadingIcon,
}: TransformCategorySelectProps) {
  return (
    <MenuSelect
      value={value}
      options={options}
      onChange={onChange}
      label={label}
      leadingIcon={leadingIcon ?? <Filter className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />}
      className={`transform-category-select ${accent}`}
    />
  );
}
