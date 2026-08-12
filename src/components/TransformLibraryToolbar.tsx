import type { ReactNode } from 'react';
import { Sparkles } from 'lucide-react';
import { ActionButton } from './AppDialogLayout';

interface TransformLibraryToolbarProps {
  createLabel: string;
  onCreate: () => void;
  children: ReactNode;
  secondaryAction?: { label: string; onClick: () => void };
}

export function TransformLibraryToolbar({
  createLabel,
  onCreate,
  children,
  secondaryAction,
}: TransformLibraryToolbarProps) {
  return (
    <div className="transform-library-toolbar @container flex flex-wrap items-center justify-end gap-2">
      <div className="flex min-w-0 basis-full items-center gap-2 @md:basis-auto @md:flex-1">
        {children}
      </div>
      {secondaryAction && (
        <ActionButton onClick={secondaryAction.onClick} className="h-8 min-h-8 shrink-0 px-3">
          {secondaryAction.label}
        </ActionButton>
      )}
      <ActionButton
        variant="primary"
        onClick={onCreate}
        className="h-8 min-h-8 shrink-0 px-3"
      >
        <Sparkles className="w-3.5 h-3.5" />
        <span>{createLabel}</span>
      </ActionButton>
    </div>
  );
}
