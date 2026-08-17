import type { ReactNode } from 'react';
import { Play } from 'lucide-react';
import { translate } from '../localization/runtime';

export function TransformationPreviewPanel({
  title = translate('component.transformationPreviewPanel.livePreview'),
  description,
  status,
  input,
  output,
}: {
  title?: string;
  description?: string;
  status?: ReactNode;
  input: ReactNode;
  output: ReactNode;
}) {
  return (
    <section className="theme-subtle-surface space-y-3 rounded-xl border p-3 @container" aria-label={title}>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="theme-status-info-text flex items-center gap-1.5 text-xs font-semibold">
          <Play className="h-3.5 w-3.5" />
          {title}
        </span>
        {description && <span className="theme-text-muted text-[10px]">{description}</span>}
      </div>
      {status}
      <div className="grid grid-cols-1 gap-3 text-xs font-mono @md:grid-cols-2">
        <div>
          <span className="theme-text-muted mb-1 block font-sans">{translate('common.input')}</span>
          {input}
        </div>
        <div>
          <span className="theme-text-muted mb-1 block font-sans font-semibold">{translate('common.output')}</span>
          {output}
        </div>
      </div>
    </section>
  );
}
