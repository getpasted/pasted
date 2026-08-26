import { CircleAlert, CircleCheck } from 'lucide-react';

export function ExtractorAvailabilityBadge({
  selectedId,
  runtimeConfigurationChanged,
  available,
  title,
  label,
}: {
  selectedId: number | 'new' | null;
  runtimeConfigurationChanged: boolean;
  available?: boolean;
  title: string;
  label: string;
}) {
  const persisted = selectedId !== 'new' && !runtimeConfigurationChanged;
  const ready = persisted && available;
  return <span
    title={title}
    className={`${ready
      ? 'theme-status-success-text'
      : persisted
        ? 'theme-status-warning-text'
        : 'theme-text-muted'} flex min-w-0 max-w-[70%] shrink items-center gap-1.5 text-[10px] font-semibold`}
  >
    {ready
      ? <CircleCheck className="h-3.5 w-3.5 shrink-0" />
      : <CircleAlert className="h-3.5 w-3.5 shrink-0" />}
    <span className="truncate">{label}</span>
  </span>;
}
