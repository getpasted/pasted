import type { ReactNode } from 'react';
import { translate } from '../localization/runtime';

export function ModifiedFieldLabel({ children, modified }: { children: ReactNode; modified: boolean }) {
  return (
    <span className="theme-text-muted font-semibold" title={modified ? translate('component.modifiedFieldLabel.modifiedFromDefault') : undefined}>
      {children}
    </span>
  );
}
