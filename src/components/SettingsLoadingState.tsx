import { LoaderCircle } from 'lucide-react';

export function SettingsLoadingState({
  label,
  className = '',
}: {
  label: string;
  className?: string;
}) {
  return (
    <div
      className={`settings-loading-state theme-text-muted flex items-center justify-center gap-2 text-xs ${className}`.trim()}
      role="status"
      aria-busy="true"
      aria-live="polite"
    >
      <LoaderCircle className="h-4 w-4 shrink-0 animate-spin" aria-hidden="true" />
      <span>{label}</span>
    </div>
  );
}
