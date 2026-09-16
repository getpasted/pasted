import { useEffect, useRef, type FormEvent } from 'react';
import { Search } from 'lucide-react';
import { translate } from '../localization/runtime';
import type { FeatureId } from '../utils/features';
import { AppDialog } from './AppDialog';
import {
  AppDialogBody,
  AppDialogButton,
  AppDialogFooter,
  AppDialogHeader,
  AppDialogHeading,
} from './AppDialogLayout';
import { getSearchHelpers } from './searchHelpers';

interface SearchDialogProps {
  isOpen: boolean;
  draft: string;
  features: Record<FeatureId, boolean>;
  onDraftChange: (draft: string) => void;
  onCancel: () => void;
  onSearch: () => void;
}

export function SearchDialog({
  isOpen,
  draft,
  features,
  onDraftChange,
  onCancel,
  onSearch,
}: SearchDialogProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const helpers = getSearchHelpers(features);

  useEffect(() => {
    if (!isOpen) return;
    let focusFrame = 0;
    const portalFrame = requestAnimationFrame(() => {
      focusFrame = requestAnimationFrame(() => {
        inputRef.current?.focus();
        inputRef.current?.select();
      });
    });
    return () => {
      cancelAnimationFrame(portalFrame);
      cancelAnimationFrame(focusFrame);
    };
  }, [isOpen]);

  const submit = (event: FormEvent) => {
    event.preventDefault();
    onSearch();
  };
  const chooseHelper = (prefix: string) => {
    onDraftChange(prefix);
    requestAnimationFrame(() => inputRef.current?.focus());
  };

  return (
    <AppDialog
      isOpen={isOpen}
      onClose={onCancel}
      labelledBy="search-dialog-title"
      panelClassName="theme-panel w-full max-w-lg overflow-hidden border"
    >
      {({ requestClose }) => (
        <form onSubmit={submit}>
          <AppDialogHeader onClose={requestClose}>
            <AppDialogHeading
              id="search-dialog-title"
              title={translate('collection.search')}
              description={translate('collection.searchActiveAndTrashedClips')}
              icon={<Search />}
              tone="info"
            />
          </AppDialogHeader>
          <AppDialogBody className="space-y-4">
            <input
              ref={inputRef}
              data-search-dialog-input
              type="text"
              autoComplete="off"
              autoCorrect="off"
              autoCapitalize="off"
              spellCheck={false}
              aria-label={translate('component.sidebar.searchAllClips')}
              placeholder={translate('component.sidebar.searchAllClips')}
              value={draft}
              onChange={(event) => onDraftChange(event.target.value)}
              className="theme-input ui-field-radius h-10 w-full border px-3 text-sm focus:outline-none"
            />
            <section aria-labelledby="search-dialog-filters" className="space-y-2">
              <h3 id="search-dialog-filters" className="theme-text-muted text-[10px] font-semibold uppercase tracking-wide">
                {translate('component.sidebar.searchFilters')}
              </h3>
              <div className="grid grid-cols-2 gap-1.5">
                {helpers.map((helper) => (
                  <button
                    type="button"
                    key={helper.prefix}
                    onClick={() => chooseHelper(helper.prefix)}
                    className="theme-menu-item ui-control-radius flex min-w-0 items-center justify-between gap-3 px-2.5 py-2 text-start"
                  >
                    <span className="font-mono text-[11px] font-semibold" dir="ltr">{helper.prefix}</span>
                    <span className="theme-text-subtle truncate text-[10px]">{helper.description}</span>
                  </button>
                ))}
              </div>
            </section>
          </AppDialogBody>
          <AppDialogFooter>
            <AppDialogButton onClick={requestClose}>{translate('common.cancel')}</AppDialogButton>
            <AppDialogButton type="submit" variant="primary">{translate('collection.search')}</AppDialogButton>
          </AppDialogFooter>
        </form>
      )}
    </AppDialog>
  );
}
