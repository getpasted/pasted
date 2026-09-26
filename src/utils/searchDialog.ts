export interface SearchDialogState {
  isOpen: boolean;
  draft: string;
}

export const CLOSED_SEARCH_DIALOG: SearchDialogState = {
  isOpen: false,
  draft: '',
};

export function openSearchDialog(committedQuery: string): SearchDialogState {
  return { isOpen: true, draft: committedQuery };
}

export function updateSearchDraft(state: SearchDialogState, draft: string): SearchDialogState {
  return { ...state, draft };
}

export function closeSearchDialog(): SearchDialogState {
  return CLOSED_SEARCH_DIALOG;
}

export function commitSearchDialog(state: SearchDialogState): string {
  return state.draft;
}
