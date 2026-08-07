import type { ReactNode } from 'react';
import { Sparkles } from 'lucide-react';

interface TransformLibraryToolbarProps {
  accent: 'pipelines' | 'operations';
  createLabel: string;
  onCreate: () => void;
  children: ReactNode;
}

export function TransformLibraryToolbar({
  accent,
  createLabel,
  onCreate,
  children,
}: TransformLibraryToolbarProps) {
  return (
    <div className="transform-library-toolbar flex items-center gap-3">
      <div className="flex min-w-0 flex-1 items-center gap-2">
        {children}
      </div>
      <button
        type="button"
        onClick={onCreate}
        className={`transform-workspace-action ui-control-radius ${accent} flex h-8 shrink-0 items-center gap-2 px-3 text-xs font-bold shadow-sm active:scale-[0.98] transition-[background-color,border-color,color,transform]`}
      >
        <Sparkles className="w-3.5 h-3.5" />
        <span>{createLabel}</span>
      </button>
    </div>
  );
}
