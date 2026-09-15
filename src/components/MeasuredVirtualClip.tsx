import { useLayoutEffect, useRef, type ReactNode } from 'react';

const CLIP_GAP = 10;

export function MeasuredVirtualClip({
  children,
  clipId,
  index,
  onMeasure,
  start,
  totalCount,
}: {
  children: ReactNode;
  clipId: number;
  index: number;
  onMeasure: (clipId: number, height: number) => void;
  start?: number;
  totalCount: number;
}) {
  const ref = useRef<HTMLDivElement | null>(null);
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) return undefined;
    const measure = () => onMeasure(clipId, element.getBoundingClientRect().height - CLIP_GAP);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [clipId, onMeasure]);
  return <div
    ref={ref}
    role="listitem"
    aria-posinset={index + 1}
    aria-setsize={totalCount}
    className={start === undefined ? 'pb-2.5' : 'absolute inset-x-0 pb-2.5'}
    style={start === undefined ? undefined : { transform: `translateY(${start}px)` }}
  >
    {children}
  </div>;
}
