import type { KeyboardEvent, MouseEvent, ReactNode } from 'react';

interface RegistryListItemProps {
  selected: boolean;
  onSelect: () => void;
  icon: ReactNode;
  title: ReactNode;
  subtitle?: ReactNode;
  trailing?: ReactNode;
  muted?: boolean;
  className?: string;
  onContextMenu?: (event: MouseEvent<HTMLDivElement>) => void;
}

export function RegistryListItem({
  selected,
  onSelect,
  icon,
  title,
  subtitle,
  trailing,
  muted = false,
  className = '',
  onContextMenu,
}: RegistryListItemProps) {
  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.target !== event.currentTarget || !['Enter', ' '].includes(event.key)) return;
    event.preventDefault();
    onSelect();
  };

  return (
    <div
      role="button"
      aria-pressed={selected}
      tabIndex={0}
      onClick={onSelect}
      onKeyDown={handleKeyDown}
      onContextMenu={onContextMenu}
      className={`theme-menu-item flex w-full cursor-pointer items-center gap-2 rounded-lg border border-transparent px-2 py-2 text-left ${selected ? 'is-selected' : ''} ${muted ? 'opacity-55' : ''} ${className}`}
    >
      <span className="shrink-0">{icon}</span>
      <span className="min-w-0 flex-1">
        <span className="theme-text-main block truncate text-xs font-semibold">{title}</span>
        {subtitle && <span className="theme-text-subtle mt-0.5 block truncate text-[9px]">{subtitle}</span>}
      </span>
      {trailing && <span className="flex shrink-0 items-center" onClick={(event) => event.stopPropagation()}>{trailing}</span>}
    </div>
  );
}
