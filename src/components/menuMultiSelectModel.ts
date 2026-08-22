export interface MultiSelectValueOption {
  value: string;
  group?: string;
  disabled?: boolean;
}

export function groupSelectionState(values: string[], options: MultiSelectValueOption[]) {
  const selectable = options.filter((option) => !option.disabled);
  const selectedCount = selectable.filter((option) => values.includes(option.value)).length;
  return {
    all: selectable.length > 0 && selectedCount === selectable.length,
    some: selectedCount > 0 && selectedCount < selectable.length,
  };
}

export function toggleMultiSelectGroup(values: string[], options: MultiSelectValueOption[]) {
  const selectableValues = options.filter((option) => !option.disabled).map((option) => option.value);
  const allSelected = selectableValues.every((value) => values.includes(value));
  return allSelected
    ? values.filter((value) => !selectableValues.includes(value))
    : [...new Set([...values, ...selectableValues])];
}

export function initialMultiSelectScrollKey(values: string[], options: MultiSelectValueOption[]) {
  const firstSelected = options.find((option) => values.includes(option.value));
  if (!firstSelected) return undefined;
  return firstSelected.group ? `group:${firstSelected.group}` : `option:${firstSelected.value}`;
}
