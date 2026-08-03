import { Sliders, Sparkles, Wrench } from 'lucide-react';
import { startWindowDrag } from '../utils/windowDrag';

export type TransformWorkspace = 'pipelines' | 'operations';

interface TransformWorkspaceHeaderProps {
  activeWorkspace: TransformWorkspace;
  filterCount: number;
  operationCount: number;
  onChange: (workspace: TransformWorkspace) => void;
  onCreate: () => void;
}

export function TransformWorkspaceHeader({
  activeWorkspace,
  filterCount,
  operationCount,
  onChange,
  onCreate,
}: TransformWorkspaceHeaderProps) {
  const isPipelines = activeWorkspace === 'pipelines';

  return (
    <header
      className="theme-toolbar transform-workspace-header h-[60px] border-b px-4 flex items-center justify-between gap-4 shrink-0 titlebar-drag-handle"
      onMouseDown={startWindowDrag}
    >
      <div className="theme-surface transform-workspace-tabs flex items-center gap-1 rounded-xl border p-1" role="tablist" aria-label="Filters and operations">
        <button
          type="button"
          role="tab"
          aria-selected={isPipelines}
          onClick={() => onChange('pipelines')}
          className={`transform-workspace-tab pipelines flex h-8 items-center gap-2 rounded-lg px-3 text-xs font-semibold transition-colors ${isPipelines ? 'is-active' : ''}`}
        >
          <Sliders className="w-4 h-4" />
          <span>Filter Pipelines</span>
          <span className="transform-workspace-count font-mono">{filterCount}</span>
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={!isPipelines}
          onClick={() => onChange('operations')}
          className={`transform-workspace-tab operations flex h-8 items-center gap-2 rounded-lg px-3 text-xs font-semibold transition-colors ${!isPipelines ? 'is-active' : ''}`}
        >
          <Wrench className="w-4 h-4" />
          <span>Operations</span>
          <span className="transform-workspace-count font-mono">{operationCount}</span>
        </button>
      </div>

      <button
        type="button"
        onClick={onCreate}
        className={`transform-workspace-action ${isPipelines ? 'pipelines' : 'operations'} flex h-9 items-center gap-2 rounded-xl px-3.5 text-xs font-bold shadow-sm active:scale-[0.98] transition-[background-color,border-color,color,transform]`}
      >
        <Sparkles className="w-3.5 h-3.5" />
        <span>{isPipelines ? 'New Pipeline' : 'New Operation'}</span>
      </button>
    </header>
  );
}
