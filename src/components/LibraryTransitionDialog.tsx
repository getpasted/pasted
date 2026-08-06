import { Clipboard, Upload } from 'lucide-react';
import { AppDialog } from './AppDialog';

interface LibraryTransitionDialogProps {
  isOpen: boolean;
  variant: 'reset' | 'import';
  title: string;
  description: string;
}

export function libraryTransitionDuration() {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 250 : 2000;
}

export async function waitForMinimumLibraryTransition(startedAt: number) {
  const remaining = Math.max(0, libraryTransitionDuration() - (performance.now() - startedAt));
  if (remaining > 0) {
    await new Promise((resolve) => window.setTimeout(resolve, remaining));
  }
}

export function LibraryTransitionDialog({
  isOpen,
  variant,
  title,
  description,
}: LibraryTransitionDialogProps) {
  const Icon = variant === 'import' ? Upload : Clipboard;
  const titleId = `library-${variant}-transition-title`;

  return (
    <AppDialog
      isOpen={isOpen}
      onClose={() => { /* Operations in flight cannot be dismissed. */ }}
      labelledBy={titleId}
      overlayClassName={`p-4 library-transition-overlay is-active is-${variant}`}
      panelClassName={`library-transition-panel is-active theme-panel w-full max-w-sm rounded-2xl border overflow-hidden font-sans is-${variant}`}
    >
      <div className={`library-transition-stage is-${variant}`} role="status" aria-live="polite">
        <div className="library-transition-mark" aria-hidden="true">
          <span className="library-transition-card is-left" />
          <span className="library-transition-card is-right" />
          <Icon />
        </div>
        <h2 id={titleId} className="app-dialog-title">{title}</h2>
        <p className="app-dialog-description">{description}</p>
        <div className="library-transition-progress" aria-hidden="true"><span /></div>
      </div>
    </AppDialog>
  );
}
