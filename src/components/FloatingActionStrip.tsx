import type { PointerEvent, MouseEvent, ReactNode } from 'react';

interface FloatingActionStripProps {
  children: ReactNode;
  label: string;
  visible?: boolean;
  revealOnGroupInteraction?: boolean;
}

export function FloatingActionStrip({
  children,
  label,
  visible = true,
  revealOnGroupInteraction = false,
}: FloatingActionStripProps) {
  const visibilityClass = revealOnGroupInteraction
    ? 'pointer-events-none opacity-0 group-hover:pointer-events-auto group-hover:opacity-100 group-focus-within:pointer-events-auto group-focus-within:opacity-100'
    : visible
      ? 'visible opacity-100'
      : 'invisible pointer-events-none opacity-0';

  const stopPointer = (event: PointerEvent<HTMLDivElement> | MouseEvent<HTMLDivElement>) => {
    event.stopPropagation();
  };

  return (
    <div
      className={`floating-action-strip absolute bottom-2 right-2 z-10 flex items-center gap-1 rounded-lg border p-1 shadow-xl transition-opacity ${visibilityClass}`}
      aria-label={label}
      onPointerDown={stopPointer}
      onMouseDown={stopPointer}
    >
      {children}
    </div>
  );
}
