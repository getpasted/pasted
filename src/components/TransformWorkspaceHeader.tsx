import { Settings2, Sparkles, Workflow } from 'lucide-react';
import { ToolPageHeader } from './ToolPageHeader';

export type TransformWorkspace = 'recipes' | 'advanced';

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
  return (
    <ToolPageHeader
      icon={<Workflow className="w-4 h-4" />}
      title="Transformations"
      description="Describe what should happen. Pasted handles the recipe."
      actions={(
        <div className="theme-surface transform-workspace-tabs flex items-center gap-1 rounded-xl border p-1" role="tablist" aria-label="Transformation workspace">
        <button
          type="button"
          role="tab"
          aria-selected={activeWorkspace === 'recipes'}
          onClick={() => onChange('recipes')}
          className={`transform-workspace-tab pipelines flex h-8 items-center gap-2 rounded-lg px-3 text-xs font-semibold transition-colors ${activeWorkspace === 'recipes' ? 'is-active' : ''}`}
        >
          <Sparkles className="w-4 h-4" />
          <span>Recipes</span>
          <span className="transform-workspace-count font-mono">{filterCount}</span>
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
        </div>
      )}
    />
  );
}
