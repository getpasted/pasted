import { Play, Settings2, Workflow } from 'lucide-react';
import { ToolPageHeader } from './ToolPageHeader';

export type TransformWorkspace = 'transforms' | 'advanced' | 'playground';

interface TransformWorkspaceHeaderProps {
  activeWorkspace: TransformWorkspace;
  transformCount: number;
  operationCount: number;
  onChange: (workspace: TransformWorkspace) => void;
}

export function TransformWorkspaceHeader({
  activeWorkspace,
  transformCount,
  operationCount,
  onChange,
}: TransformWorkspaceHeaderProps) {
  return (
    <ToolPageHeader
      icon={<Workflow className="w-4 h-4" />}
      title="Transformations"
      description="Describe the result. Pasted builds the reusable steps."
      actions={(
        <div className="theme-surface transform-workspace-tabs flex items-center gap-1 rounded-xl border p-1" role="tablist" aria-label="Transformation workspace">
        <button
          type="button"
          role="tab"
          aria-selected={activeWorkspace === 'transforms'}
          onClick={() => onChange('transforms')}
          className={`transform-workspace-tab pipelines flex h-8 items-center gap-2 rounded-lg px-3 text-xs font-semibold transition-colors ${activeWorkspace === 'transforms' ? 'is-active' : ''}`}
        >
          <Workflow className="w-4 h-4" />
          <span>Transforms</span>
          <span className="transform-workspace-count font-mono">{transformCount}</span>
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={activeWorkspace === 'advanced'}
          onClick={() => onChange('advanced')}
          className={`transform-workspace-tab operations flex h-8 items-center gap-2 rounded-lg px-3 text-xs font-semibold transition-colors ${activeWorkspace === 'advanced' ? 'is-active' : ''}`}
        >
          <Settings2 className="w-4 h-4" />
          <span>Advanced</span>
          <span className="transform-workspace-count font-mono">{operationCount}</span>
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={activeWorkspace === 'playground'}
          onClick={() => onChange('playground')}
          className={`transform-workspace-tab pipelines flex h-8 items-center gap-2 rounded-lg px-3 text-xs font-semibold transition-colors ${activeWorkspace === 'playground' ? 'is-active' : ''}`}
        >
          <Play className="w-4 h-4" />
          <span>Playground</span>
        </button>
        </div>
      )}
    />
  );
}
