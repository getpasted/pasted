import { useCallback, useRef } from 'react';

export function useNewItemSelection<
  TId extends string | number,
  TEmpty extends 'new' | null,
>({
  selectedId,
  setSelectedId,
  itemIds,
  emptySelection,
}: {
  selectedId: TId | 'new' | TEmpty;
  setSelectedId: (selection: TId | 'new' | TEmpty) => void;
  itemIds: readonly TId[];
  emptySelection: TEmpty;
}) {
  const previousSelectedIdRef = useRef<TId | null>(null);

  const beginNew = useCallback(() => {
    if (selectedId !== 'new' && selectedId !== null) previousSelectedIdRef.current = selectedId as TId;
    setSelectedId('new');
  }, [selectedId, setSelectedId]);

  const cancelNew = useCallback(() => {
    const previousId = previousSelectedIdRef.current;
    setSelectedId(previousId !== null && itemIds.includes(previousId)
      ? previousId
      : itemIds[0] ?? emptySelection);
  }, [emptySelection, itemIds, setSelectedId]);

  return { beginNew, cancelNew };
}
