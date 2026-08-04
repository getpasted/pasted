import { Sliders, Wrench } from 'lucide-react';
import { ToolPageHeader } from './ToolPageHeader';

export type TransformWorkspace = 'pipelines' | 'operations';

interface TransformWorkspaceHeaderProps {
  activeWorkspace: TransformWorkspace;
  filterCount: number;
  operationCount: number;
  onChange: (workspace: TransformWorkspace) => void;
}

export function TransformWorkspaceHeader({
  activeWorkspace,
  filterCount,
  operationCount,
  onChange,
}: TransformWorkspaceHeaderProps) {
  const isPipelines = activeWorkspace === 'pipelines';

  return (
    <ToolPageHeader
      icon={<Sliders className="w-4 h-4" />}
      title="Filters & Operations"
      actions={(
        <div className="theme-surface transform-workspace-tabs flex items-center gap-1 rounded-xl border p-1" role="tablist" aria-label="Filters and operations">
        <button
          type="button"
          role="tab"
          aria-selected={isPipelines}
          onClick={() => onChange('pipelines')}
          className={`transform-workspace-tab pipelines flex h-8 items-center gap-2 rounded-lg px-3 text-xs font-semibold transition-colors ${isPipelines ? 'is-active' : ''}`}
        >
          <Sliders className="w-4 h-4" />
          <span>Pipelines</span>
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
      )}
    />
  );
}
